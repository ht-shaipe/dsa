//! 决策信号服务 - 信号创建/追踪/评估
//!
//! 主模型: dsa_core::models::db::DecisionSignal as DecisionSignalModel
//! 辅助模型: DecisionSignalOutcome, DecisionSignalFeedback
//! 使用 deck DataTable/TableService 模式

use dsa_core::db::{query_rows, row_get_f64, row_get_i64, row_get_string, row_get_value};
use dsa_core::models::db::DecisionSignal as DecisionSignalModel;
use dsa_core::models::db::DecisionSignalFeedback as DecisionSignalFeedbackModel;
use dsa_core::models::db::DecisionSignalOutcome as DecisionSignalOutcomeModel;
use dsa_core::utils;
use deck::sqlite::{DataTable, SelectExecutor};
use deck::QueryExecutor;
use deck::TableService;

use qta_crawler::Real;
use tube::{Result, Value};
use tube_web::RequestParameter;

// ─── 辅助表: DecisionSignalOutcome ────────────────────────────────

struct OutcomeTable { request: RequestParameter }

impl DataTable<DecisionSignalOutcomeModel> for OutcomeTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<DecisionSignalOutcomeModel> for OutcomeTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl OutcomeTable {
    fn new(param: &RequestParameter) -> Self { OutcomeTable { request: param.clone() } }

    fn query_outcomes(&self, signal_id: i64, limit: i64) -> Result<Vec<Value>> {
        let mut q = self.select()
            .columns(cols![
                "id", "signal_id", "stock_code", "eval_horizon", "eval_date",
                "actual_return", "max_drawdown", "direction_correct",
                "hit_target", "hit_stop_loss", "status", "create_time"
            ])
            .r#where(conds![{ "status" = 1 }])
            .order(ord!("eval_date DESC"))
            .limit(limit as u64);

        if signal_id > 0 {
            q = q.r#where(conds![{ "signal_id" = signal_id }]);
        }

        q.query_values()
    }
}

// ─── 辅助表: DecisionSignalFeedback ──────────────────────────────

struct FeedbackTable { request: RequestParameter }

impl DataTable<DecisionSignalFeedbackModel> for FeedbackTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<DecisionSignalFeedbackModel> for FeedbackTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl FeedbackTable {
    fn new(param: &RequestParameter) -> Self { FeedbackTable { request: param.clone() } }

    fn query_feedback_list(&self, signal_id: i64) -> Result<Vec<Value>> {
        self.select()
            .columns(cols![
                "id", "signal_id", "feedback_value", "reason_code",
                "note", "source", "create_time"
            ])
            .r#where(conds![{ "signal_id" = signal_id }])
            .order(ord!("create_time DESC"))
            .query_values()
    }

    fn upsert_feedback(&self, signal_id: i64, feedback: &str, reason_code: &str, note: &str) -> Result<Value> {
        let data = value!({
            "signal_id": signal_id,
            "feedback_value": feedback,
            "reason_code": reason_code,
            "note": note,
            "source": "api",
        });
        self.duplicate().data(&data).execute()
    }
}

// ─── 主服务: Decision ────────────────────────────────────────────

pub struct Decision { request: RequestParameter }

impl DataTable<DecisionSignalModel> for Decision {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}

impl TableService<DecisionSignalModel> for Decision {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}

impl Decision {
    pub fn new(param: &RequestParameter) -> Self {
        Decision { request: param.clone() }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "create" => self.create().await,
            "list" => self.list().await,
            "latest" => self.latest().await,
            "detail" => self.detail().await,
            "update_status" => self.update_status().await,
            "outcomes" => self.outcomes().await,
            "feedback" => self.feedback().await,
            "feedback_list" => self.feedback_list().await,
            "reassess" => self.reassess().await,
            "data_quality" => self.data_quality().await,
            "profile_policy" => self.profile_policy().await,
            "evaluate_outcomes" => self.evaluate_outcomes().await,
            "stats" => self.stats().await,
            _ => Err(error!("decision不支持方法: {}", method)),
        }
    }

    fn params(&self) -> &Value {
        &self.request.value
    }

    // ─── DB 操作 (通过 deck fluent builder) ────────────────────────

    /// 去重检查: 同一股票+动作+日期的活跃信号
    fn check_duplicate(&self, code: &str, action: &str, signal_date: &str) -> Result<Option<i64>> {
        let res = self.select()
            .columns(cols!["id"])
            .r#where(conds![
                { "stock_code" = code },
                { "action" = action },
                { "signal_date" = signal_date },
                { "status" = 1 }
            ])
            .limit(1)
            .one()?;
        if res.is_null() {
            Ok(None)
        } else {
            Ok(res.get("id").and_then(|v| v.as_f64()).map(|v| v as i64))
        }
    }

    /// 创建决策信号
    fn insert_signal(&self, code: &str, name: &str, signal_date: &str, action: &str,
                     sentiment_score: i32, confidence_level: &str,
                     entry_price: f64, stop_loss: f64, target_price: f64,
                     reasoning: &str, evidence: &str, scope_type: &str, analysis_id: i64) -> Result<Value> {
        let confidence: i32 = match confidence_level {
            "high" => 90,
            "medium" => 60,
            "low" => 30,
            _ => 50,
        };
        let data = value!({
            "stock_code": code,
            "stock_name": name,
            "signal_date": signal_date,
            "action": action,
            "sentiment_score": sentiment_score,
            "confidence_level": confidence_level,
            "entry_price": entry_price,
            "stop_loss": stop_loss,
            "target_price": target_price,
            "reasoning": reasoning,
            "evidence": evidence,
            "scope_type": scope_type,
            "scope_value": "",
            "analysis_id": analysis_id,
            "plan_quality": confidence,
            "status": 1,
        });
        self.insert().data(&data).execute()
    }

    /// 获取最新活跃信号
    fn query_latest(&self, code: &str) -> Result<Option<Value>> {
        let res = self.select()
            .columns(cols![
                "id", "stock_code", "stock_name", "signal_date", "action",
                "sentiment_score", "confidence_level", "entry_price",
                "stop_loss", "target_price", "reasoning", "evidence",
                "scope_type", "analysis_id", "plan_quality", "status", "create_time"
            ])
            .r#where(conds![{ "stock_code" = code }, { "status" = 1 }])
            .order(ord!("create_time DESC"))
            .limit(1)
            .one()?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    /// 获取信号详情
    fn query_detail(&self, id: i64) -> Result<Option<Value>> {
        let res = self.select()
            .columns(cols![
                "id", "stock_code", "stock_name", "signal_date", "action",
                "sentiment_score", "confidence_level", "entry_price",
                "stop_loss", "target_price", "reasoning", "evidence",
                "scope_type", "scope_value", "analysis_id", "plan_quality",
                "status", "creator_id", "create_time", "modify_time"
            ])
            .r#where(conds![{ "id" = id }])
            .one()?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    /// 更新信号状态
    fn update_signal_status(&self, id: i64, status_val: i32) -> Result<Value> {
        let data = value!({
            "status": status_val,
            "modifyTime": chrono::Local::now().naive_local(),
        });
        self.update()
            .data(&data)
            .r#where(conds![{ "id" = id }])
            .execute()
    }

    // ─── 业务方法 ───────────────────────────────────────────

    async fn create(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(error!("请提供股票代码"));
        }

        let action = utils::param_string(params, "action");
        if action.is_empty() {
            return Err(error!("请提供信号动作"));
        }

        let name = utils::param_string(params, "name");
        let signal_date = utils::param_string(params, "signalDate");
        let sentiment_score = params
            .get("sentimentScore")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i32;
        let confidence_level = utils::param_string(params, "confidenceLevel");
        let entry_price = params
            .get("entryPrice")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let stop_loss = params
            .get("stopLoss")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let target_price = params
            .get("targetPrice")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let reasoning = utils::param_string(params, "reasoning");
        let evidence = utils::param_string(params, "evidence");
        let analysis_id = params
            .get("analysisId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let scope_type = utils::param_string(params, "scopeType");

        // 去重检查
        if let Some(existing_id) = self.check_duplicate(&code, &action, &signal_date)? {
            return Ok(value!({
                "id": existing_id, "message": "信号已存在，跳过重复创建", "duplicate": true
            }));
        }

        let scope_val = if scope_type.is_empty() {
            "watchlist"
        } else {
            &scope_type
        };

        let result = self.insert_signal(
            &code, &name, &signal_date, &action,
            sentiment_score, &confidence_level,
            entry_price, stop_loss, target_price,
            &reasoning, &evidence, scope_val, analysis_id,
        ).map_err(|e| error!("创建决策信号失败: {}", e))?;

        Ok(value!({
            "id": result, "code": code, "action": action, "duplicate": false
        }))
    }

    async fn list(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        let status_str = utils::param_string(params, "status");
        let action = utils::param_string(params, "action");
        let holding_only = params
            .get("holdingOnly")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) > 0.0;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let connector = self.get_connector()
            .ok_or_else(|| error!("MySQL连接未初始化"))?;

        let mut conditions = vec!["ds.status >= 1".to_string()];
        let mut p: Vec<(String, Value)> = Vec::new();

        if !code.is_empty() {
            conditions.push("ds.stock_code = :code".to_string());
            p.push(("code".to_string(), Value::from(code.as_str())));
        }
        if !status_str.is_empty() {
            let status_val = match status_str.as_str() {
                "active" => 1,
                "expired" => 2,
                "invalidated" => 3,
                "closed" => 4,
                "archived" => 5,
                _ => 1,
            };
            conditions.push("ds.status = :status".to_string());
            p.push(("status".to_string(), Value::from(status_val)));
        }
        if !action.is_empty() {
            conditions.push("ds.action = :action".to_string());
            p.push(("action".to_string(), Value::from(action.as_str())));
        }
        if holding_only {
            conditions.push("ds.action IN ('buy', 'add', 'hold')".to_string());
        }

        conditions.push("1=1".to_string());
        let where_clause = conditions.join(" AND ");

        let sql = format!(
            "SELECT ds.id, ds.stock_code, ds.stock_name, ds.signal_date, ds.action, \
             ds.sentiment_score, ds.confidence_level, ds.entry_price, ds.stop_loss, \
             ds.target_price, ds.reasoning, ds.evidence, ds.scope_type, ds.analysis_id, \
             ds.plan_quality, ds.status, ds.create_time, ds.modify_time \
             FROM decision_signals ds WHERE {} ORDER BY ds.create_time DESC LIMIT :limit",
            where_clause
        );
        p.push(("limit".to_string(), Value::from(limit)));

        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| error!("查询决策信号列表失败: {}", e))?;

        Ok(Value::Array(rows))
    }

    async fn latest(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(error!("请提供股票代码"));
        }

        match self.query_latest(&code)? {
            Some(v) => Ok(v),
            None => Ok(Value::Null),
        }
    }

    async fn detail(&self) -> Result<Value> {
        let params = self.params();
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(error!("请提供信号ID"));
        }

        match self.query_detail(id)? {
            Some(v) => Ok(v),
            None => Ok(Value::Null),
        }
    }

    async fn update_status(&self) -> Result<Value> {
        let params = self.params();
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(error!("请提供信号ID"));
        }

        let new_status = utils::param_string(params, "status");
        if new_status.is_empty() {
            return Err(error!("请提供新状态"));
        }

        let status_val = match new_status.as_str() {
            "active" => 1,
            "expired" => 2,
            "invalidated" => 3,
            "closed" => 4,
            "archived" => 5,
            _ => return Err(error!("无效状态: {}", new_status)),
        };

        self.update_signal_status(id, status_val)?;

        Ok(value!({"id": id, "newStatus": new_status}))
    }

    async fn outcomes(&self) -> Result<Value> {
        let params = self.params();
        let signal_id = params
            .get("signalId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let table = OutcomeTable::new(&self.request);
        let results = table.query_outcomes(signal_id, limit)?;
        Ok(Value::Array(results))
    }

    async fn feedback(&self) -> Result<Value> {
        let params = self.params();
        let signal_id = params
            .get("signalId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if signal_id == 0 {
            return Err(error!("请提供信号ID"));
        }

        let feedback = utils::param_string(params, "feedback");
        if feedback.is_empty() {
            return Err(error!("请提供反馈值"));
        }

        let fb_val = match feedback.as_str() {
            "agree" | "disagree" | "partial" => feedback.as_str(),
            _ => return Err(error!("无效反馈值: {}，支持: agree/disagree/partial", feedback)),
        };

        let reason_code = utils::param_string(params, "reasonCode");
        let note = utils::param_string(params, "note");

        let table = FeedbackTable::new(&self.request);
        table.upsert_feedback(signal_id, fb_val, &reason_code, &note)?;

        Ok(value!({"signalId": signal_id, "feedback": feedback}))
    }

    async fn feedback_list(&self) -> Result<Value> {
        let params = self.params();
        let signal_id = params
            .get("signalId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if signal_id == 0 {
            return Err(error!("请提供信号ID"));
        }

        let table = FeedbackTable::new(&self.request);
        let results = table.query_feedback_list(signal_id)?;
        Ok(Value::Array(results))
    }

    async fn reassess(&self) -> Result<Value> {
        let connector = self.get_connector()
            .ok_or_else(|| error!("MySQL连接未初始化"))?;

        // Find active signals older than 1 day
        let sql = "SELECT id, stock_code, action, entry_price, stop_loss, target_price, create_time \
             FROM decision_signals WHERE status = 1 AND create_time < DATE_SUB(NOW(), INTERVAL 1 DAY)";
        let rows = query_rows(sql, vec![], &connector)
            .map_err(|e| error!("查询活跃信号失败: {}", e))?;

        let mut reassessed = 0i64;
        let real = Real::new();

        for row in &rows {
            let id: i64 = row_get_i64(row, "id");
            let code: String = row_get_string(row, "stockCode");
            let stop_loss: f64 = row_get_f64(row, "stopLoss");
            let target_price: f64 = row_get_f64(row, "targetPrice");

            if code.is_empty() {
                continue;
            }

            // Fetch current price
            let prefix = utils::market_prefix(&code);
            let quote = real
                .get_price(&format!("{}{}", prefix, code))
                .await
                .ok();

            let current_price = quote
                .as_ref()
                .and_then(|q| q.get("price").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);

            if current_price <= 0.0 {
                continue;
            }

            // Determine new status
            let action: String = row_get_string(row, "action");
            let is_bearish = matches!(action.as_str(), "sell" | "reduce" | "avoid");

            let (hit_stop, hit_target) = if is_bearish {
                (stop_loss > 0.0 && current_price >= stop_loss,
                 target_price > 0.0 && current_price <= target_price)
            } else {
                (stop_loss > 0.0 && current_price <= stop_loss,
                 target_price > 0.0 && current_price >= target_price)
            };

            if hit_stop {
                self.update_signal_status(id, 3)?;
                reassessed += 1;
            } else if hit_target {
                self.update_signal_status(id, 4)?;
                reassessed += 1;
            }
        }

        Ok(value!({"reassessed": reassessed, "totalChecked": rows.len() as i64}))
    }

    async fn data_quality(&self) -> Result<Value> {
        let params = self.params();
        let signal_id = params
            .get("signalId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if signal_id == 0 {
            return Err(error!("请提供信号ID"));
        }

        let connector = self.get_connector()
            .ok_or_else(|| error!("MySQL连接未初始化"))?;
        let sql = "SELECT id, stock_code, reasoning, evidence, entry_price, stop_loss, target_price \
             FROM decision_signals WHERE id = :id";
        let rows = query_rows(sql, vec![("id".to_string(), Value::from(signal_id))], &connector)
            .map_err(|e| error!("查询信号详情失败: {}", e))?;

        if rows.is_empty() {
            return Ok(Value::Null);
        }

        let row = &rows[0];
        let has_reasoning = !row_get_string(row, "reasoning").is_empty();
        let has_evidence = !row_get_string(row, "evidence").is_empty();
        let entry_price = row_get_f64(row, "entryPrice");
        let stop_loss = row_get_f64(row, "stopLoss");
        let target_price = row_get_f64(row, "targetPrice");
        let has_entry = entry_price > 0.0;
        let has_stop = stop_loss > 0.0;
        let has_target = target_price > 0.0;

        let mut score = 0i32;
        let mut breakdown = Vec::new();

        if has_reasoning {
            score += 1;
            breakdown.push(value!({"item": "reasoning", "present": true, "score": 1}));
        } else {
            breakdown.push(value!({"item": "reasoning", "present": false, "score": 0}));
        }

        if has_evidence {
            score += 1;
            breakdown.push(value!({"item": "evidence", "present": true, "score": 1}));
        } else {
            breakdown.push(value!({"item": "evidence", "present": false, "score": 0}));
        }

        if has_entry {
            score += 1;
            breakdown.push(value!({"item": "entryPrice", "present": true, "value": entry_price, "score": 1}));
        } else {
            breakdown.push(value!({"item": "entryPrice", "present": false, "score": 0}));
        }

        if has_stop {
            score += 1;
            breakdown.push(value!({"item": "stopLoss", "present": true, "value": stop_loss, "score": 1}));
        } else {
            breakdown.push(value!({"item": "stopLoss", "present": false, "score": 0}));
        }

        if has_target {
            score += 1;
            breakdown.push(value!({"item": "targetPrice", "present": true, "value": target_price, "score": 1}));
        } else {
            breakdown.push(value!({"item": "targetPrice", "present": false, "score": 0}));
        }

        let level = match score {
            5 => "excellent",
            4 => "good",
            3 => "fair",
            2 => "poor",
            _ => "incomplete",
        };

        Ok(value!({
            "signalId": signal_id,
            "score": score,
            "maxScore": 5,
            "level": level,
            "breakdown": breakdown,
        }))
    }

    async fn profile_policy(&self) -> Result<Value> {
        let config = dsa_core::get_global_config();
        let agent = &config.agent;

        Ok(value!({
            "enabled": agent.enabled,
            "orchestratorMode": agent.orchestrator_mode.clone(),
            "maxSteps": agent.max_steps,
            "arch": agent.arch.clone(),
            "skills": agent.skills.clone(),
        }))
    }

    async fn evaluate_outcomes(&self) -> Result<Value> {
        let params = self.params();
        let eval_window = params
            .get("evalWindow")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        let tracker = dsa_backtest::SignalTracker::new();
        let outcomes = tracker.evaluate_outcomes(eval_window).await
            .map_err(|e| error!("{}", e))?;

        Ok(Value::Array(outcomes))
    }

    async fn stats(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");

        let connector = self.get_connector()
            .ok_or_else(|| error!("MySQL连接未初始化"))?;

        let (sql, p) = if code.is_empty() {
            ("SELECT COUNT(*) as total, \
              SUM(CASE WHEN action IN ('buy','add') THEN 1 ELSE 0 END) as bullish, \
              SUM(CASE WHEN action IN ('sell','reduce','avoid') THEN 1 ELSE 0 END) as bearish, \
              SUM(CASE WHEN action = 'hold' THEN 1 ELSE 0 END) as neutral, \
              AVG(sentiment_score) as avg_score \
              FROM decision_signals WHERE status = 1".to_string(),
             vec![])
        } else {
            ("SELECT COUNT(*) as total, \
              SUM(CASE WHEN action IN ('buy','add') THEN 1 ELSE 0 END) as bullish, \
              SUM(CASE WHEN action IN ('sell','reduce','avoid') THEN 1 ELSE 0 END) as bearish, \
              SUM(CASE WHEN action = 'hold' THEN 1 ELSE 0 END) as neutral, \
              AVG(sentiment_score) as avg_score \
              FROM decision_signals WHERE status = 1 AND stock_code = :code".to_string(),
             vec![("code".to_string(), Value::from(code.as_str()))])
        };

        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| error!("查询信号统计失败: {}", e))?;

        if rows.is_empty() {
            return Ok(value!({
                "total": 0, "bullish": 0, "bearish": 0, "neutral": 0, "avgScore": 0.0
            }));
        }

        let row = &rows[0];
        Ok(value!({
            "total": row_get_f64(row, "total") as i64,
            "bullish": row_get_f64(row, "bullish") as i64,
            "bearish": row_get_f64(row, "bearish") as i64,
            "neutral": row_get_f64(row, "neutral") as i64,
            "avgScore": row_get_f64(row, "avgScore"),
        }))
    }
}
