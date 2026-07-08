//! 决策信号服务 - 信号创建/追踪/评估

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use qta_crawler::Real;
use tube::Value;

/// 决策信号服务
pub struct DecisionService {}

impl DecisionService {
    /// 创建决策信号服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: create, list, latest, detail, update_status, outcomes, feedback, feedback_list, reassess, data_quality, profile_policy, evaluate_outcomes, stats
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "create" => self.create(params).await,
            "list" => self.list(params).await,
            "latest" => self.latest(params).await,
            "detail" => self.detail(params).await,
            "update_status" => self.update_status(params).await,
            "outcomes" => self.outcomes(params).await,
            "feedback" => self.feedback(params).await,
            "feedback_list" => self.feedback_list(params).await,
            "reassess" => self.reassess(params).await,
            "data_quality" => self.data_quality(params).await,
            "profile_policy" => self.profile_policy(params).await,
            "evaluate_outcomes" => self.evaluate_outcomes(params).await,
            "stats" => self.stats(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "decision不支持方法: {}",
                method
            ))),
        }
    }

    /// 创建决策信号 (含去重)
    async fn create(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(DsaError::Validation("请提供股票代码".to_string()));
        }

        let action = utils::param_string(params, "action");
        if action.is_empty() {
            return Err(DsaError::Validation("请提供信号动作".to_string()));
        }

        let connector = utils::get_db_connector()?;
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

        // 去重检查: 同一股票+动作+日期的活跃信号
        let dedup_sql = "SELECT id FROM decision_signals \
             WHERE stock_code = :code AND action = :action AND signal_date = :sdate AND status = 1 \
             LIMIT 1";
        let dedup_rows = Helper::query_rows(
            dedup_sql,
            vec![
                ("code".to_string(), Value::from(code.as_str())),
                ("action".to_string(), Value::from(action.as_str())),
                ("sdate".to_string(), Value::from(signal_date.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("去重检查失败: {}", e)))?;

        if !dedup_rows.is_empty() {
            let existing_id: i64 = dedup_rows[0].get_value(0).as_f64().unwrap_or(0.0) as i64;
            return Ok(value!({
                "id": existing_id, "message": "信号已存在，跳过重复创建", "duplicate": true
            }));
        }

        let scope_val = if scope_type.is_empty() {
            "watchlist"
        } else {
            &scope_type
        };

        let insert_sql = "INSERT INTO decision_signals \
             (stock_code, stock_name, signal_date, action, sentiment_score, confidence_level, \
              entry_price, stop_loss, target_price, reasoning, evidence, scope_type, scope_value, \
              analysis_id, plan_quality, status, creator_id, create_time, modify_time) \
             VALUES (:code, :name, :sdate, :action, :score, :conf, \
              :entry, :sl, :tp, :reasoning, :evidence, :scope, '', \
              :aid, 0, 1, 0, NOW(), NOW())";

        let result = Helper::execute(
            insert_sql,
            vec![
                ("code".to_string(), Value::from(code.as_str())),
                ("name".to_string(), Value::from(name.as_str())),
                ("sdate".to_string(), Value::from(signal_date.as_str())),
                ("action".to_string(), Value::from(action.as_str())),
                ("score".to_string(), Value::from(sentiment_score)),
                ("conf".to_string(), Value::from(confidence_level.as_str())),
                ("entry".to_string(), Value::from(entry_price)),
                ("sl".to_string(), Value::from(stop_loss)),
                ("tp".to_string(), Value::from(target_price)),
                ("reasoning".to_string(), Value::from(reasoning.as_str())),
                ("evidence".to_string(), Value::from(evidence.as_str())),
                ("scope".to_string(), Value::from(scope_val)),
                ("aid".to_string(), Value::from(analysis_id)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("创建决策信号失败: {}", e)))?;

        Ok(value!({
            "id": result as i64, "code": code, "action": action, "duplicate": false
        }))
    }

    /// 查询决策信号列表
    async fn list(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let code = utils::param_string(params, "code");
        let status = utils::param_string(params, "status");
        let action = utils::param_string(params, "action");
        let holding_only = params
            .get("holdingOnly")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let mut conditions = vec!["ds.status >= 1".to_string()];
        let mut p: Vec<(String, Value)> = Vec::new();

        if !code.is_empty() {
            conditions.push("ds.stock_code = :code".to_string());
            p.push(("code".to_string(), Value::from(code.as_str())));
        }
        if !status.is_empty() {
            conditions.push("ds.status = :status".to_string());
            let status_val = match status.as_str() {
                "active" => 1,
                "expired" => 2,
                "invalidated" => 3,
                "closed" => 4,
                "archived" => 5,
                _ => 1,
            };
            p.push(("status".to_string(), Value::from(status_val)));
        }
        if !action.is_empty() {
            conditions.push("ds.action = :action".to_string());
            p.push(("action".to_string(), Value::from(action.as_str())));
        }
        if holding_only > 0 {
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

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询决策信号列表失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(Value::Array(results))
    }

    /// 获取最新活跃信号
    async fn latest(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(DsaError::Validation("请提供股票代码".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, stock_code, stock_name, signal_date, action, \
             sentiment_score, confidence_level, entry_price, stop_loss, target_price, \
             reasoning, evidence, scope_type, analysis_id, plan_quality, status, create_time \
             FROM decision_signals WHERE stock_code = :code AND status = 1 \
             ORDER BY create_time DESC LIMIT 1";
        let rows = Helper::query_rows(
            sql,
            vec![("code".to_string(), Value::from(code.as_str()))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询最新信号失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(Value::Null);
        }

        Ok(rows[0].to_value2())
    }

    /// 获取信号详情
    async fn detail(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供信号ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, stock_code, stock_name, signal_date, action, \
             sentiment_score, confidence_level, entry_price, stop_loss, target_price, \
             reasoning, evidence, scope_type, scope_value, analysis_id, plan_quality, \
             status, creator_id, create_time, modify_time \
             FROM decision_signals WHERE id = :id";
        let rows = Helper::query_rows(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| DsaError::Database(format!("查询信号详情失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(Value::Null);
        }

        Ok(rows[0].to_value2())
    }

    /// 更新信号状态
    async fn update_status(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供信号ID".to_string()));
        }

        let new_status = utils::param_string(params, "status");
        if new_status.is_empty() {
            return Err(DsaError::Validation("请提供新状态".to_string()));
        }

        let status_val = match new_status.as_str() {
            "active" => 1,
            "expired" => 2,
            "invalidated" => 3,
            "closed" => 4,
            "archived" => 5,
            _ => return Err(DsaError::Validation(format!("无效状态: {}", new_status))),
        };

        let connector = utils::get_db_connector()?;
        let sql = "UPDATE decision_signals SET status = :status, modify_time = NOW() WHERE id = :id";
        Helper::execute(
            sql,
            vec![
                ("status".to_string(), Value::from(status_val)),
                ("id".to_string(), Value::from(id)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("更新信号状态失败: {}", e)))?;

        Ok(value!({"id": id, "newStatus": new_status}))
    }

    /// 查询信号结果
    async fn outcomes(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let signal_id = params
            .get("signalId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let (sql, p) = if signal_id > 0 {
            (
                "SELECT id, signal_id, stock_code, eval_horizon, eval_date, actual_return, \
                 max_drawdown, direction_correct, hit_target, hit_stop_loss, status, create_time \
                 FROM decision_signal_outcomes WHERE signal_id = :sid AND status = 1 \
                 ORDER BY eval_date DESC LIMIT :limit"
                    .to_string(),
                vec![
                    ("sid".to_string(), Value::from(signal_id)),
                    ("limit".to_string(), Value::from(limit)),
                ],
            )
        } else {
            (
                "SELECT id, signal_id, stock_code, eval_horizon, eval_date, actual_return, \
                 max_drawdown, direction_correct, hit_target, hit_stop_loss, status, create_time \
                 FROM decision_signal_outcomes WHERE status = 1 \
                 ORDER BY eval_date DESC LIMIT :limit"
                    .to_string(),
                vec![("limit".to_string(), Value::from(limit))],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询信号结果失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(Value::Array(results))
    }

    /// 用户反馈 (upsert to decision_signal_feedback)
    async fn feedback(&self, params: &Value) -> DsaResult<Value> {
        let signal_id = params
            .get("signalId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if signal_id == 0 {
            return Err(DsaError::Validation("请提供信号ID".to_string()));
        }

        let feedback = utils::param_string(params, "feedback");
        if feedback.is_empty() {
            return Err(DsaError::Validation("请提供反馈值".to_string()));
        }

        // Validate feedback value
        let fb_val = match feedback.as_str() {
            "agree" | "disagree" | "partial" => feedback.as_str(),
            _ => return Err(DsaError::Validation(format!("无效反馈值: {}，支持: agree/disagree/partial", feedback))),
        };

        let reason_code = utils::param_string(params, "reasonCode");
        let note = utils::param_string(params, "note");
        // rating is kept for API compatibility but ignored
        let _rating = params
            .get("rating")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i32;

        let connector = utils::get_db_connector()?;
        let sql = "INSERT INTO decision_signal_feedback \
             (signal_id, feedback_value, reason_code, note, source, create_time, modify_time) \
             VALUES (:sid, :fb, :rc, :note, 'api', NOW(), NOW()) \
             ON DUPLICATE KEY UPDATE feedback_value = VALUES(feedback_value), \
             reason_code = VALUES(reason_code), note = VALUES(note), modify_time = NOW()";
        Helper::execute(
            sql,
            vec![
                ("sid".to_string(), Value::from(signal_id)),
                ("fb".to_string(), Value::from(fb_val)),
                ("rc".to_string(), Value::from(reason_code.as_str())),
                ("note".to_string(), Value::from(note.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("保存反馈失败: {}", e)))?;

        Ok(value!({"signalId": signal_id, "feedback": feedback}))
    }

    /// 查询信号反馈列表
    async fn feedback_list(&self, params: &Value) -> DsaResult<Value> {
        let signal_id = params
            .get("signalId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if signal_id == 0 {
            return Err(DsaError::Validation("请提供信号ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, signal_id, feedback_value, reason_code, note, source, create_time \
             FROM decision_signal_feedback WHERE signal_id = :sid ORDER BY create_time DESC";
        let rows = Helper::query_rows(
            sql,
            vec![("sid".to_string(), Value::from(signal_id))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询反馈列表失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(Value::Array(results))
    }

    /// 重新评估活跃信号 (检查止损/目标价触发)
    async fn reassess(&self, _params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;

        // Find active signals older than 1 day
        let sql = "SELECT id, stock_code, action, entry_price, stop_loss, target_price, create_time \
             FROM decision_signals WHERE status = 1 AND create_time < DATE_SUB(NOW(), INTERVAL 1 DAY)";
        let rows = Helper::query_rows(sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询活跃信号失败: {}", e)))?;

        let mut reassessed = 0i64;
        let real = Real::new();

        for row in &rows {
            let id: i64 = row.get_value(0).as_f64().unwrap_or(0.0) as i64;
            let code: String = row.get_value(1).as_str().unwrap_or_default().to_string();
            let stop_loss: f64 = row.get_value(4).as_f64().unwrap_or(0.0);
            let target_price: f64 = row.get_value(5).as_f64().unwrap_or(0.0);

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
            // For buy/hold signals: stop_loss is below, target is above
            // For sell signals: stop_loss is above, target is below (inverse logic)
            let action: String = row.get_value(2).as_str().unwrap_or_default().to_string();
            let is_bearish = matches!(action.as_str(), "sell" | "reduce" | "avoid");

            let (hit_stop, hit_target) = if is_bearish {
                // Bearish: stop_loss is above entry (price rising against us), target is below
                (stop_loss > 0.0 && current_price >= stop_loss,
                 target_price > 0.0 && current_price <= target_price)
            } else {
                // Bullish/neutral: stop_loss is below entry (price falling against us), target is above
                (stop_loss > 0.0 && current_price <= stop_loss,
                 target_price > 0.0 && current_price >= target_price)
            };

            if hit_stop {
                let update_sql = "UPDATE decision_signals SET status = 3, modify_time = NOW() WHERE id = :id";
                Helper::execute(update_sql, vec![("id".to_string(), Value::from(id))], &connector)
                    .map_err(|e| DsaError::Database(format!("更新信号状态失败: {}", e)))?;
                reassessed += 1;
            } else if hit_target {
                let update_sql = "UPDATE decision_signals SET status = 4, modify_time = NOW() WHERE id = :id";
                Helper::execute(update_sql, vec![("id".to_string(), Value::from(id))], &connector)
                    .map_err(|e| DsaError::Database(format!("更新信号状态失败: {}", e)))?;
                reassessed += 1;
            }
        }

        Ok(value!({"reassessed": reassessed, "totalChecked": rows.len() as i64}))
    }

    /// 信号数据质量评分
    async fn data_quality(&self, params: &Value) -> DsaResult<Value> {
        let signal_id = params
            .get("signalId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if signal_id == 0 {
            return Err(DsaError::Validation("请提供信号ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, stock_code, reasoning, evidence, entry_price, stop_loss, target_price \
             FROM decision_signals WHERE id = :id";
        let rows = Helper::query_rows(sql, vec![("id".to_string(), Value::from(signal_id))], &connector)
            .map_err(|e| DsaError::Database(format!("查询信号详情失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(Value::Null);
        }

        let row = &rows[0];
        let has_reasoning = !row.get_value(2).as_str().unwrap_or_default().is_empty();
        let has_evidence = !row.get_value(3).as_str().unwrap_or_default().is_empty();
        let entry_price = row.get_value(4).as_f64().unwrap_or(0.0);
        let stop_loss = row.get_value(5).as_f64().unwrap_or(0.0);
        let target_price = row.get_value(6).as_f64().unwrap_or(0.0);
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

    /// 决策配置策略查询
    async fn profile_policy(&self, _params: &Value) -> DsaResult<Value> {
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

    /// 评估信号预测结果
    async fn evaluate_outcomes(&self, params: &Value) -> DsaResult<Value> {
        let eval_window = params
            .get("evalWindow")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        let tracker = dsa_backtest::SignalTracker::new();
        let outcomes = tracker.evaluate_outcomes(eval_window).await?;

        Ok(Value::Array(outcomes))
    }

    /// 信号统计
    async fn stats(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let code = utils::param_string(params, "code");

        let (sql, p) = if code.is_empty() {
            (
                "SELECT COUNT(*) as total, \
                 SUM(CASE WHEN action IN ('buy','add') THEN 1 ELSE 0 END) as bullish, \
                 SUM(CASE WHEN action IN ('sell','reduce','avoid') THEN 1 ELSE 0 END) as bearish, \
                 SUM(CASE WHEN action = 'hold' THEN 1 ELSE 0 END) as neutral, \
                 AVG(sentiment_score) as avg_score \
                 FROM decision_signals WHERE status = 1"
                    .to_string(),
                vec![],
            )
        } else {
            (
                "SELECT COUNT(*) as total, \
                 SUM(CASE WHEN action IN ('buy','add') THEN 1 ELSE 0 END) as bullish, \
                 SUM(CASE WHEN action IN ('sell','reduce','avoid') THEN 1 ELSE 0 END) as bearish, \
                 SUM(CASE WHEN action = 'hold' THEN 1 ELSE 0 END) as neutral, \
                 AVG(sentiment_score) as avg_score \
                 FROM decision_signals WHERE status = 1 AND stock_code = :code"
                    .to_string(),
                vec![("code".to_string(), Value::from(code.as_str()))],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询信号统计失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(value!({
                "total": 0, "bullish": 0, "bearish": 0, "neutral": 0, "avgScore": 0.0
            }));
        }

        let row = &rows[0];
        Ok(value!({
            "total": row.get_value(0).as_f64().unwrap_or(0.0) as i64,
            "bullish": row.get_value(1).as_f64().unwrap_or(0.0) as i64,
            "bearish": row.get_value(2).as_f64().unwrap_or(0.0) as i64,
            "neutral": row.get_value(3).as_f64().unwrap_or(0.0) as i64,
            "avgScore": row.get_value(4).as_f64().unwrap_or(0.0),
        }))
    }
}
