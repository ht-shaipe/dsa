//! 分析服务 - 编排 pipeline 分析流程

use dsa_core::models::AnalysisReport;
use dsa_core::{DsaError, DsaResult, utils};
use dsa_pipeline::pipeline::AnalysisPipeline;
use dsa_pipeline::report_renderer::ReportRenderer;
use deck_mysql::{self, DataRow};
use tube::Value;

/// 分析服务
pub struct AnalysisService {}

impl AnalysisService {
    /// 创建分析服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: analyze, batch, report, list, market-review, history_list, history_detail, history_compare, history_search
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "analyze" => self.analyze(params).await,
            "batch" => self.batch_analyze(params).await,
            "report" => self.get_report(params).await,
            "list" => self.list_reports(params).await,
            "market-review" => self.market_review(params).await,
            "history_list" => self.history_list(params).await,
            "history_detail" => self.history_detail(params).await,
            "history_compare" => self.history_compare(params).await,
            "history_search" => self.history_search(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "analysis不支持方法: {}",
                method
            ))),
        }
    }

    async fn analyze(&self, params: &Value) -> DsaResult<Value> {
        let code = Self::param_code(params);
        let name = utils::param_string(params, "name");

        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Err(DsaError::LlmAnalysis("API Key 未配置".to_string()));
        }

        let pipeline = AnalysisPipeline::new(
            &conf.llm.provider,
            &api_key,
            &conf.llm.model,
            conf.llm.temperature,
            conf.llm.timeout_seconds,
        )?;

        let kline_data = utils::fetch_kline(&code, "daily").await?;
        let realtime = utils::fetch_realtime_quote(&code).await?;
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
            .await?;

        let renderer = ReportRenderer::new();
        let markdown = renderer.render_markdown(&report);
        let text = renderer.render_text(&report);

        let report_json = serde_json::to_value(&report)
            .map_err(|e| DsaError::ReportParse(format!("报告序列化失败: {}", e)))?;

        Ok(value!({
            "status": "ok",
            "data": {
                "report": report_json,
                "markdown": markdown,
                "text": text,
                "code": code,
                "name": stock_name,
            }
        }))
    }

    async fn batch_analyze(&self, params: &Value) -> DsaResult<Value> {
        let codes_val = utils::param_string(params, "codes");
        if codes_val.is_empty() {
            return Err(DsaError::Validation("请提供股票代码列表".to_string()));
        }

        let codes: Vec<String> = codes_val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Err(DsaError::LlmAnalysis("API Key 未配置".to_string()));
        }

        let pipeline = AnalysisPipeline::new(
            &conf.llm.provider,
            &api_key,
            &conf.llm.model,
            conf.llm.temperature,
            conf.llm.timeout_seconds,
        )?;

        let mut results = Vec::new();
        for code in codes {
            match self.analyze_single(&pipeline, &code).await {
                Ok(report) => {
                    let renderer = ReportRenderer::new();
                    let text = renderer.render_text(&report);
                    let json = serde_json::to_value(&report).unwrap_or_default();
                    results.push(value!({"code": code, "status": "ok", "text": text, "report": json}));
                }
                Err(e) => {
                    results.push(value!({"code": code, "status": "error", "error": e.to_string()}));
                }
            }
        }

        Ok(value!({"status": "ok", "data": results}))
    }

    async fn analyze_single(
        &self,
        pipeline: &AnalysisPipeline,
        code: &str,
    ) -> DsaResult<AnalysisReport> {
        let kline_data = utils::fetch_kline(code, "daily").await?;
        let realtime = utils::fetch_realtime_quote(code).await?;
        let market_ctx = utils::fetch_market_context().await;
        let name = realtime
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| code.to_string());

        pipeline
            .analyze_stock(code, &name, &kline_data, Some(&realtime), market_ctx.as_deref())
            .await
    }

    async fn get_report(&self, params: &Value) -> DsaResult<Value> {
        let id = params.get("id").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
        let query_id = utils::param_string(params, "queryId");

        let connector = utils::get_db_connector()?;

        let (sql, p) = if id > 0 {
            ("SELECT id, stockCode, stockName, sentimentScore, decisionType, operationAdvice, \
              analysisSummary, riskWarning, rawResult, contextSnapshot, reportType, \
              queryId, status, createTime, modifyTime \
              FROM analysis_history WHERE id = :id".to_string(),
             vec![("id".to_string(), Value::from(id))])
        } else if !query_id.is_empty() {
            ("SELECT id, stockCode, stockName, sentimentScore, decisionType, operationAdvice, \
              analysisSummary, riskWarning, rawResult, contextSnapshot, reportType, \
              queryId, status, createTime, modifyTime \
              FROM analysis_history WHERE queryId = :qid".to_string(),
             vec![("qid".to_string(), Value::from(query_id.as_str()))])
        } else {
            return Err(DsaError::Validation("请提供id或queryId".to_string()));
        };

        let rows = deck_mysql::Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询报告失败: {}", e)))?;

        let data = rows.first().map(|r| r.to_value2()).unwrap_or_else(|| value!({}));
        Ok(value!({"status": "ok", "data": data}))
    }

    async fn list_reports(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(20.0) as i64;

        let connector = utils::get_db_connector()?;

        let (sql, p) = if !code.is_empty() {
            ("SELECT id, stockCode, stockName, sentimentScore, decisionType, operationAdvice, \
              analysisSummary, createTime \
              FROM analysis_history WHERE stockCode = :code AND status = 1 \
              ORDER BY createTime DESC LIMIT :limit".to_string(),
             vec![("code".to_string(), Value::from(code.as_str())),
                  ("limit".to_string(), Value::from(limit))])
        } else {
            ("SELECT id, stockCode, stockName, sentimentScore, decisionType, operationAdvice, \
              analysisSummary, createTime \
              FROM analysis_history WHERE status = 1 \
              ORDER BY createTime DESC LIMIT :limit".to_string(),
             vec![("limit".to_string(), Value::from(limit))])
        };

        let rows = deck_mysql::Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询报告列表失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    async fn market_review(&self, params: &Value) -> DsaResult<Value> {
        let gen = dsa_pipeline::market_review::MarketReviewGenerator::new();
        gen.generate(params).await
    }

    /// 历史记录列表 - 支持按股票代码过滤和分页
    async fn history_list(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(20.0) as i64;
        let offset = params.get("offset").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;

        let connector = utils::get_db_connector()?;

        let (sql, p) = if !code.is_empty() {
            ("SELECT id, stockCode, stockName, sentimentScore, decisionType, operationAdvice, \
              analysisSummary, riskWarning, reportType, queryId, status, createTime, modifyTime \
              FROM analysis_history WHERE stockCode = :code AND status = 1 \
              ORDER BY createTime DESC LIMIT :limit OFFSET :offset".to_string(),
             vec![("code".to_string(), Value::from(code.as_str())),
                  ("limit".to_string(), Value::from(limit)),
                  ("offset".to_string(), Value::from(offset))])
        } else {
            ("SELECT id, stockCode, stockName, sentimentScore, decisionType, operationAdvice, \
              analysisSummary, riskWarning, reportType, queryId, status, createTime, modifyTime \
              FROM analysis_history WHERE status = 1 \
              ORDER BY createTime DESC LIMIT :limit OFFSET :offset".to_string(),
             vec![("limit".to_string(), Value::from(limit)),
                  ("offset".to_string(), Value::from(offset))])
        };

        let rows = deck_mysql::Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询历史列表失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 历史记录详情 - 按ID查询单条记录
    async fn history_detail(&self, params: &Value) -> DsaResult<Value> {
        let id = params.get("id").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
        if id <= 0 {
            return Err(DsaError::Validation("请提供有效的id".to_string()));
        }

        let connector = utils::get_db_connector()?;

        let sql = "SELECT id, stockCode, stockName, sentimentScore, decisionType, operationAdvice, \
              analysisSummary, riskWarning, rawResult, contextSnapshot, reportType, \
              queryId, status, createTime, modifyTime \
              FROM analysis_history WHERE id = :id";
        let rows = deck_mysql::Helper::query_rows(
            sql,
            vec![("id".to_string(), Value::from(id))],
            &connector,
        ).map_err(|e| DsaError::Database(format!("查询历史详情失败: {}", e)))?;

        let data = rows.first().map(|r| r.to_value2()).unwrap_or_else(|| value!({}));
        Ok(value!({"status": "ok", "data": data}))
    }

    /// 历史记录对比 - 并排比较两条分析记录
    async fn history_compare(&self, params: &Value) -> DsaResult<Value> {
        let id1 = params.get("id1").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
        let id2 = params.get("id2").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
        if id1 <= 0 || id2 <= 0 {
            return Err(DsaError::Validation("请提供有效的id1和id2".to_string()));
        }

        let connector = utils::get_db_connector()?;

        let sql = "SELECT id, stockCode, stockName, sentimentScore, decisionType, operationAdvice, \
              analysisSummary, riskWarning, rawResult, contextSnapshot, reportType, \
              queryId, status, createTime, modifyTime \
              FROM analysis_history WHERE id IN (:id1, :id2) ORDER BY id";
        let rows = deck_mysql::Helper::query_rows(
            sql,
            vec![("id1".to_string(), Value::from(id1)), ("id2".to_string(), Value::from(id2))],
            &connector,
        ).map_err(|e| DsaError::Database(format!("查询历史对比失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        if results.len() < 2 {
            return Err(DsaError::Validation("未找到两条记录进行对比".to_string()));
        }
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 历史记录搜索 - 按关键词搜索stockName/analysisSummary
    async fn history_search(&self, params: &Value) -> DsaResult<Value> {
        let keyword = utils::param_string(params, "keyword");
        if keyword.is_empty() {
            return Err(DsaError::Validation("请提供搜索关键词".to_string()));
        }
        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(20.0) as i64;

        let connector = utils::get_db_connector()?;

        let sql = "SELECT id, stockCode, stockName, sentimentScore, decisionType, operationAdvice, \
              analysisSummary, createTime \
              FROM analysis_history WHERE status = 1 \
              AND (stockName LIKE :kw OR analysisSummary LIKE :kw) \
              ORDER BY createTime DESC LIMIT :limit";
        let kw_pattern = format!("%{}%", keyword);
        let rows = deck_mysql::Helper::query_rows(
            sql,
            vec![
                ("kw".to_string(), Value::from(kw_pattern.as_str())),
                ("limit".to_string(), Value::from(limit)),
            ],
            &connector,
        ).map_err(|e| DsaError::Database(format!("搜索历史记录失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    fn param_code(params: &Value) -> String {
        utils::param_string(params, "code")
    }
}
