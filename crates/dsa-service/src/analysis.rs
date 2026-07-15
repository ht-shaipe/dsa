//! 分析服务 - 编排 pipeline 分析流程
//!
//! 主模型: dsa_core::models::db::AnalysisHistory as AnalysisHistoryModel
//! 使用 deck DataTable/TableService 模式

use deck::sqlite::{DataTable, SelectExecutor};
use deck::QueryExecutor;
use deck::TableService;
use dsa_core::db::query_rows;
use dsa_core::models::db::AnalysisHistory as AnalysisHistoryModel;
use dsa_core::models::AnalysisReport;
use dsa_core::utils;
use dsa_pipeline::pipeline::AnalysisPipeline;
use dsa_pipeline::report_renderer::ReportRenderer;

use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct Analysis {
    request: RequestParameter,
}

impl DataTable<AnalysisHistoryModel> for Analysis {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}

impl TableService<AnalysisHistoryModel> for Analysis {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}

impl Analysis {
    pub fn new(param: &RequestParameter) -> Self {
        Analysis {
            request: param.clone(),
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "analyze" => self.analyze().await,
            "batch" => self.batch_analyze().await,
            "report" => self.get_report().await,
            "list" => self.list_reports().await,
            "market-review" => self.market_review().await,
            "history_list" => self.history_list().await,
            "history_detail" => self.history_detail().await,
            "history_compare" => self.history_compare().await,
            "history_search" => self.history_search().await,
            _ => Err(error!("analysis不支持方法: {}", method)),
        }
    }

    fn params(&self) -> &Value {
        &self.request.value
    }

    // ─── DB 操作 (通过 deck fluent builder) ─────────────────────────

    /// 保存分析报告到数据库 (INSERT ON DUPLICATE KEY UPDATE)
    fn save_report_to_db(
        &self,
        code: &str,
        name: &str,
        report: &AnalysisReport,
        report_json: &serde_json::Value,
    ) {
        let sentiment_score = report.sentiment_score.unwrap_or(0);
        let decision_type = report.decision_type.as_deref().unwrap_or("");
        let operation_advice = report.operation_advice.as_deref().unwrap_or("");
        let analysis_summary = report.analysis_summary.as_deref().unwrap_or("");
        let risk_warning = report.risk_warning.as_deref().unwrap_or("");
        let report_json_str = match serde_json::to_string(report_json) {
            Ok(s) => s,
            Err(_) => "{}".to_string(),
        };

        let conf = dsa_core::get_global_config();
        let connector = match self.get_connector() {
            Some(c) => c,
            None => {
                tracing::error!("save_report_to_db: 数据库连接未初始化");
                return;
            }
        };

        let is_sqlite = conf.database.is_sqlite();
        let now_expr = if is_sqlite {
            "datetime('now')"
        } else {
            "NOW()"
        };

        let sql = if is_sqlite {
            format!(
                "INSERT INTO analysis_history \
                 (stock_code, stock_name, sentiment_score, decision_type, operation_advice, \
                  analysis_summary, risk_warning, report_json, report_type, status, \
                  llm_provider, llm_model, create_time, modify_time) \
                 VALUES (:code, :name, :score, :dtype, :advice, :summary, :risk, :rjson, :rtype, 1, \
                  :provider, :model, {}, {})",
                now_expr, now_expr
            )
        } else {
            format!(
                "INSERT INTO analysis_history \
                 (stock_code, stock_name, sentiment_score, decision_type, operation_advice, \
                  analysis_summary, risk_warning, report_json, report_type, status, \
                  llm_provider, llm_model, create_time, modify_time) \
                 VALUES (:code, :name, :score, :dtype, :advice, :summary, :risk, :rjson, :rtype, 1, \
                  :provider, :model, {}, {})",
                now_expr, now_expr
            )
        };

        if let Err(e) = dsa_core::db::execute(
            &sql,
            vec![
                ("code".to_string(), Value::from(code.to_string())),
                ("name".to_string(), Value::from(name.to_string())),
                ("score".to_string(), Value::from(sentiment_score)),
                ("dtype".to_string(), Value::from(decision_type.to_string())),
                (
                    "advice".to_string(),
                    Value::from(operation_advice.to_string()),
                ),
                (
                    "summary".to_string(),
                    Value::from(analysis_summary.to_string()),
                ),
                ("risk".to_string(), Value::from(risk_warning.to_string())),
                ("rjson".to_string(), Value::from(report_json_str)),
                ("rtype".to_string(), Value::from("full".to_string())),
                (
                    "provider".to_string(),
                    Value::from(conf.llm.provider.clone()),
                ),
                ("model".to_string(), Value::from(conf.llm.model.clone())),
            ],
            &connector,
        ) {
            tracing::error!("save_report_to_db 失败: {}", e);
        }
    }

    /// 根据ID查询单条记录
    fn find_by_id(&self, id: i64) -> Result<Option<Value>> {
        let res = self.select().r#where(conds![{ "id" = id }]).one()?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    /// 根据queryId查询单条记录
    fn find_by_query_id(&self, query_id: &str) -> Result<Option<Value>> {
        let res = self
            .select()
            .r#where(conds![{ "query_id" = query_id }])
            .one()?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    /// 查询报告列表 (按股票代码过滤)
    fn query_report_list(&self, code: &str, limit: i64) -> Result<Vec<Value>> {
        let mut q = self
            .select()
            .columns(cols![
                "id",
                "stock_code",
                "stock_name",
                "sentiment_score",
                "decision_type",
                "operation_advice",
                "analysis_summary",
                "create_time"
            ])
            .r#where(conds![{ "status" = 1 }])
            .order(ord!("create_time DESC"))
            .limit(limit as u64);

        if !code.is_empty() {
            q = q.r#where(conds![{ "stock_code" = code }]);
        }

        q.query_values()
    }

    /// 查询历史列表 (分页 + 按股票代码过滤)
    fn query_history_list(&self, code: &str, limit: i64, offset: i64) -> Result<Vec<Value>> {
        let mut q = self
            .select()
            .columns(cols![
                "id",
                "stock_code",
                "stock_name",
                "sentiment_score",
                "decision_type",
                "operation_advice",
                "analysis_summary",
                "risk_warning",
                "report_type",
                "query_id",
                "status",
                "create_time",
                "modify_time"
            ])
            .r#where(conds![{ "status" = 1 }])
            .order(ord!("create_time DESC"))
            .limit(limit as u64)
            .offset(offset as u64);

        if !code.is_empty() {
            q = q.r#where(conds![{ "stock_code" = code }]);
        }

        q.query_values()
    }

    /// 根据ID查询历史详情
    fn query_history_detail(&self, id: i64) -> Result<Option<Value>> {
        let res = self
            .select()
            .columns(cols![
                "id",
                "stock_code",
                "stock_name",
                "sentiment_score",
                "decision_type",
                "operation_advice",
                "analysis_summary",
                "risk_warning",
                "report_json",
                "context_snapshot",
                "report_type",
                "query_id",
                "status",
                "create_time",
                "modify_time"
            ])
            .r#where(conds![{ "id" = id }])
            .one()?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    /// 对比查询两条记录
    fn query_history_compare(&self, id1: i64, id2: i64) -> Result<Vec<Value>> {
        self.select()
            .columns(cols![
                "id",
                "stock_code",
                "stock_name",
                "sentiment_score",
                "decision_type",
                "operation_advice",
                "analysis_summary",
                "risk_warning",
                "report_json",
                "context_snapshot",
                "report_type",
                "query_id",
                "status",
                "create_time",
                "modify_time"
            ])
            .r#where(conds![{ "id" in vec![Value::from(id1), Value::from(id2)] }])
            .order(ord!("id ASC"))
            .limit(2)
            .query_values()
    }

    /// 关键词搜索 (LIKE 查询 - 保留原始SQL)
    fn query_history_search(&self, keyword: &str, limit: i64) -> Result<Vec<Value>> {
        let connector = self
            .get_connector()
            .ok_or_else(|| error!("MySQL连接未初始化"))?;
        let kw_pattern = format!("%{}%", keyword);
        let sql = format!(
            "SELECT id, stock_code, stock_name, sentiment_score, decision_type, operation_advice, \
              analysis_summary, create_time \
              FROM analysis_history WHERE status = 1 \
              AND (stock_name LIKE :kw OR analysis_summary LIKE :kw) \
              ORDER BY create_time DESC LIMIT {}",
            limit
        );
        let rows = query_rows(
            &sql,
            vec![("kw".to_string(), Value::from(kw_pattern.as_str()))],
            &connector,
        )?;

        Ok(rows)
    }

    // ─── 业务方法 ────────────────────────────────────────────

    async fn analyze(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        let name = utils::param_string(params, "name");

        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Err(error!("API Key 未配置"));
        }

        let pipeline = AnalysisPipeline::new(
            &conf.llm.provider,
            &api_key,
            &conf.llm.model,
            conf.llm.temperature,
            conf.llm.timeout_seconds,
        )
        .map_err(|e| tube::Error::from(format!("Pipeline初始化失败: {}", e)))?;

        let kline_data = utils::fetch_kline(&code, "daily")
            .await
            .map_err(|e| tube::Error::from(format!("{}", e)))?;
        let realtime = utils::fetch_realtime_quote(&code)
            .await
            .map_err(|e| tube::Error::from(format!("{}", e)))?;
        let market_ctx = utils::fetch_market_context().await;

        let stock_name = if name.is_empty() {
            realtime
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| code.clone())
        } else {
            name
        };

        let report = pipeline
            .analyze_stock(
                &code,
                &stock_name,
                &kline_data,
                Some(&realtime),
                market_ctx.as_deref(),
            )
            .await
            .map_err(|e| tube::Error::from(format!("{}", e)))?;

        let renderer = ReportRenderer::new();
        let markdown = renderer.render_markdown(&report);
        let text = renderer.render_text(&report);

        let report_json = serde_json::to_value(&report)
            .map_err(|e| tube::Error::from(format!("报告序列化失败: {}", e)))?;

        let mut report_json_for_db = report_json.clone();
        if let serde_json::Value::Object(ref mut map) = report_json_for_db {
            map.insert(
                "markdown".to_string(),
                serde_json::Value::String(markdown.clone()),
            );
            map.insert("text".to_string(), serde_json::Value::String(text.clone()));
        }

        let result = value!({
            "report": report_json,
            "markdown": markdown,
            "text": text,
            "code": code.clone(),
            "name": stock_name.clone(),
        });

        self.save_report_to_db(&code, &stock_name, &report, &report_json_for_db);

        Ok(result)
    }

    async fn batch_analyze(&self) -> Result<Value> {
        let params = self.params();
        let codes_val = utils::param_string(params, "codes");
        if codes_val.is_empty() {
            return Err(error!("请提供股票代码列表"));
        }

        let codes: Vec<String> = codes_val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Err(error!("API Key 未配置"));
        }

        let pipeline = AnalysisPipeline::new(
            &conf.llm.provider,
            &api_key,
            &conf.llm.model,
            conf.llm.temperature,
            conf.llm.timeout_seconds,
        )
        .map_err(|e| tube::Error::from(format!("Pipeline初始化失败: {}", e)))?;

        let mut results = Vec::new();
        for code in codes {
            match self.analyze_single(&pipeline, &code).await {
                Ok(report) => {
                    let renderer = ReportRenderer::new();
                    let text = renderer.render_text(&report);
                    let markdown = renderer.render_markdown(&report);
                    let json = serde_json::to_value(&report).unwrap_or_default();

                    let name = report.stock_name.clone().unwrap_or_else(|| code.clone());
                    let mut json_for_db = json.clone();
                    if let serde_json::Value::Object(ref mut map) = json_for_db {
                        map.insert(
                            "markdown".to_string(),
                            serde_json::Value::String(markdown.clone()),
                        );
                        map.insert("text".to_string(), serde_json::Value::String(text.clone()));
                    }
                    self.save_report_to_db(&code, &name, &report, &json_for_db);

                    results
                        .push(value!({"code": code, "status": "ok", "text": text, "report": json}));
                }
                Err(e) => {
                    results.push(value!({"code": code, "status": "error", "error": e.to_string()}));
                }
            }
        }

        Ok(Value::Array(results))
    }

    async fn analyze_single(
        &self,
        pipeline: &AnalysisPipeline,
        code: &str,
    ) -> Result<AnalysisReport> {
        let kline_data = utils::fetch_kline(code, "daily")
            .await
            .map_err(|e| tube::Error::from(format!("{}", e)))?;
        let realtime = utils::fetch_realtime_quote(code)
            .await
            .map_err(|e| tube::Error::from(format!("{}", e)))?;
        let market_ctx = utils::fetch_market_context().await;
        let name = realtime
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| code.to_string());

        pipeline
            .analyze_stock(
                code,
                &name,
                &kline_data,
                Some(&realtime),
                market_ctx.as_deref(),
            )
            .await
            .map_err(|e| tube::Error::from(format!("{}", e)))
    }

    async fn get_report(&self) -> Result<Value> {
        let params = self.params();
        let id = params.get("id").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
        let query_id = utils::param_string(params, "queryId");

        if id > 0 {
            let data = self.find_by_id(id)?;
            Ok(data.unwrap_or_else(|| value!({})))
        } else if !query_id.is_empty() {
            let data = self.find_by_query_id(&query_id)?;
            Ok(data.unwrap_or_else(|| value!({})))
        } else {
            Err(error!("请提供id或queryId"))
        }
    }

    async fn list_reports(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(20.0) as i64;

        let results = self.query_report_list(&code, limit)?;
        Ok(Value::Array(results))
    }

    async fn market_review(&self) -> Result<Value> {
        let gen = dsa_pipeline::market_review::MarketReviewGenerator::new();
        gen.generate(self.params())
            .await
            .map_err(|e| tube::Error::from(format!("{}", e)))
    }

    async fn history_list(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(20.0) as i64;
        let offset = params.get("offset").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;

        let results = self.query_history_list(&code, limit, offset)?;
        Ok(Value::Array(results))
    }

    async fn history_detail(&self) -> Result<Value> {
        let params = self.params();
        let id = params.get("id").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
        if id <= 0 {
            return Err(error!("请提供有效的id"));
        }

        let data = self.query_history_detail(id)?;
        Ok(data.unwrap_or_else(|| value!({})))
    }

    async fn history_compare(&self) -> Result<Value> {
        let params = self.params();
        let id1 = params.get("id1").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
        let id2 = params.get("id2").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
        if id1 <= 0 || id2 <= 0 {
            return Err(error!("请提供有效的id1和id2"));
        }

        let results = self.query_history_compare(id1, id2)?;
        if results.len() < 2 {
            return Err(error!("未找到两条记录进行对比"));
        }
        Ok(Value::Array(results))
    }

    async fn history_search(&self) -> Result<Value> {
        let params = self.params();
        let keyword = utils::param_string(params, "keyword");
        if keyword.is_empty() {
            return Err(error!("请提供搜索关键词"));
        }
        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(20.0) as i64;

        let results = self.query_history_search(&keyword, limit)?;
        Ok(Value::Array(results))
    }
}
