//! 告警服务 - 价格/涨跌幅/成交量告警规则

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use qta_crawler::Real;
use tube::Value;

/// 告警服务
pub struct AlertService {}

impl AlertService {
    /// 创建告警服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: rules, rule_create, rule_update, rule_delete, rule_enable, rule_disable, rule_test, triggers, notifications, cooldowns, cooldown_create, cooldown_clear, notification_log, notification_log_create, check_all, market_light
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "rules" => self.rules(params).await,
            "rule_create" => self.rule_create(params).await,
            "rule_update" => self.rule_update(params).await,
            "rule_delete" => self.rule_delete(params).await,
            "rule_enable" => self.rule_enable(params).await,
            "rule_disable" => self.rule_disable(params).await,
            "rule_test" => self.rule_test(params).await,
            "triggers" => self.triggers(params).await,
            "notifications" => self.notifications(params).await,
            "cooldowns" => self.cooldowns(params).await,
            "cooldown_create" => self.cooldown_create(params).await,
            "cooldown_clear" => self.cooldown_clear(params).await,
            "notification_log" => self.notification_log(params).await,
            "notification_log_create" => self.notification_log_create(params).await,
            "check_all" => self.check_all(params).await,
            "market_light" => self.market_light(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "alert不支持方法: {}",
                method
            ))),
        }
    }

    /// 列出告警规则
    async fn rules(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let code = utils::param_string(params, "code");

        let (sql, p) = if code.is_empty() {
            (
                "SELECT id, stockCode, stockName, ruleType, conditionJson, \
                 enabled, lastTriggeredAt, triggerCount, createTime, modifyTime \
                 FROM alert_rules WHERE status >= 1 ORDER BY createTime DESC".to_string(),
                vec![],
            )
        } else {
            (
                "SELECT id, stockCode, stockName, ruleType, conditionJson, \
                 enabled, lastTriggeredAt, triggerCount, createTime, modifyTime \
                 FROM alert_rules WHERE status >= 1 AND stockCode = :code ORDER BY createTime DESC".to_string(),
                vec![("code".to_string(), Value::from(code.as_str()))],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询告警规则失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 创建告警规则
    async fn rule_create(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(DsaError::Validation("请提供股票代码".to_string()));
        }

        let rule_type = utils::param_string(params, "ruleType");
        if rule_type.is_empty() {
            return Err(DsaError::Validation("请提供规则类型".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let name = utils::param_string(params, "name");
        let condition_json = utils::param_string(params, "condition");

        // 使用 alert_rules 表 (如果不存在则用 intelligence_source 表临时存储)
        // 实际应使用 alert_rules 表，此处简化用 SQL 直接操作
        let sql = "INSERT INTO alert_rules \
             (stockCode, stockName, ruleType, conditionJson, enabled, triggerCount, status, createTime, modifyTime) \
             VALUES (:code, :name, :type, :cond, 1, 0, 1, NOW(), NOW())";

        let result = Helper::execute(
            sql,
            vec![
                ("code".to_string(), Value::from(code.as_str())),
                ("name".to_string(), Value::from(name.as_str())),
                ("type".to_string(), Value::from(rule_type.as_str())),
                ("cond".to_string(), Value::from(condition_json.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("创建告警规则失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"id": result as i64, "code": code, "ruleType": rule_type}}))
    }

    /// 更新告警规则
    async fn rule_update(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供规则ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let condition_json = utils::param_string(params, "condition");

        let sql = "UPDATE alert_rules SET conditionJson = :cond, modifyTime = NOW() WHERE id = :id";
        Helper::execute(
            sql,
            vec![
                ("cond".to_string(), Value::from(condition_json.as_str())),
                ("id".to_string(), Value::from(id)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("更新告警规则失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"id": id}}))
    }

    /// 删除告警规则
    async fn rule_delete(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供规则ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "UPDATE alert_rules SET status = 0, modifyTime = NOW() WHERE id = :id";
        Helper::execute(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| DsaError::Database(format!("删除告警规则失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"id": id}}))
    }

    /// 启用规则
    async fn rule_enable(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供规则ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "UPDATE alert_rules SET enabled = 1, modifyTime = NOW() WHERE id = :id";
        Helper::execute(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| DsaError::Database(format!("启用规则失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"id": id, "enabled": true}}))
    }

    /// 禁用规则
    async fn rule_disable(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供规则ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "UPDATE alert_rules SET enabled = 0, modifyTime = NOW() WHERE id = :id";
        Helper::execute(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| DsaError::Database(format!("禁用规则失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"id": id, "enabled": false}}))
    }

    /// 测试规则 (干跑)
    async fn rule_test(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(DsaError::Validation("请提供股票代码".to_string()));
        }

        let condition_json = utils::param_string(params, "condition");
        let condition: Value = serde_json::from_str(&condition_json).unwrap_or(value!({}));

        let real = Real::new();
        let prefix = utils::market_prefix(&code);

        let quote = real
            .get_price(&format!("{}{}", prefix, code))
            .await
            .map_err(|e| DsaError::StockData(format!("获取行情失败: {}", e)))?;

        let current_price = quote.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let change_pct = quote
            .get("changePercent")
            .or_else(|| quote.get("change_pct"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // 评估条件
        let price_above = condition
            .get("priceAbove")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let price_below = condition
            .get("priceBelow")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let change_above = condition
            .get("changeAbove")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let change_below = condition
            .get("changeBelow")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let mut triggered = false;
        let mut reasons = Vec::new();

        if price_above > 0.0 && current_price >= price_above {
            triggered = true;
            reasons.push(format!("价格{:.2}>={:.2}", current_price, price_above));
        }
        if price_below > 0.0 && current_price <= price_below {
            triggered = true;
            reasons.push(format!("价格{:.2}<={:.2}", current_price, price_below));
        }
        if change_above > 0.0 && change_pct >= change_above {
            triggered = true;
            reasons.push(format!("涨跌幅{:.2}%>={:.2}%", change_pct, change_above));
        }
        if change_below < 0.0 && change_pct <= change_below {
            triggered = true;
            reasons.push(format!("涨跌幅{:.2}%<={:.2}%", change_pct, change_below));
        }

        Ok(value!({
            "status": "ok",
            "data": {
                "triggered": triggered,
                "currentPrice": current_price,
                "changePct": change_pct,
                "reasons": reasons,
            }
        }))
    }

    /// 触发历史
    async fn triggers(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let sql = "SELECT id, ruleId, stockCode, triggerType, triggerValue, \
             conditionSnapshot, notified, createTime \
             FROM alert_triggers WHERE status >= 1 \
             ORDER BY createTime DESC LIMIT :limit";
        let rows = Helper::query_rows(sql, vec![("limit".to_string(), Value::from(limit))], &connector)
            .map_err(|e| DsaError::Database(format!("查询触发历史失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 通知列表
    async fn notifications(&self, params: &Value) -> DsaResult<Value> {
        // 简化: 返回触发记录中已通知的
        let connector = utils::get_db_connector()?;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let sql = "SELECT id, ruleId, stockCode, triggerType, triggerValue, \
             conditionSnapshot, createTime \
             FROM alert_triggers WHERE status >= 1 AND notified = 1 \
             ORDER BY createTime DESC LIMIT :limit";
        let rows = Helper::query_rows(sql, vec![("limit".to_string(), Value::from(limit))], &connector)
            .map_err(|e| DsaError::Database(format!("查询通知列表失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 查询活跃冷却记录
    async fn cooldowns(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let sql = "SELECT id, ruleId, ruleKey, target, severity, lastTriggeredAt, \
             cooldownUntil, reason, state, updatedTime \
             FROM alert_cooldowns WHERE state = 'active' \
             ORDER BY cooldownUntil ASC LIMIT :limit";
        let rows = Helper::query_rows(sql, vec![("limit".to_string(), Value::from(limit))], &connector)
            .map_err(|e| DsaError::Database(format!("查询冷却记录失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 创建冷却记录
    async fn cooldown_create(&self, params: &Value) -> DsaResult<Value> {
        let rule_id = params
            .get("ruleId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if rule_id == 0 {
            return Err(DsaError::Validation("请提供规则ID".to_string()));
        }

        let target = utils::param_string(params, "target");
        let severity = utils::param_string(params, "severity");
        let cooldown_minutes = params
            .get("cooldownMinutes")
            .and_then(|v| v.as_f64())
            .unwrap_or(30.0) as i64;
        let reason = utils::param_string(params, "reason");

        let connector = utils::get_db_connector()?;
        let sql = "INSERT INTO alert_cooldowns \
             (ruleId, ruleKey, target, severity, lastTriggeredAt, cooldownUntil, reason, state, updatedTime) \
             VALUES (:rid, '', :target, :severity, NOW(), DATE_ADD(NOW(), INTERVAL :minutes MINUTE), :reason, 'active', NOW())";

        let result = Helper::execute(
            sql,
            vec![
                ("rid".to_string(), Value::from(rule_id)),
                ("target".to_string(), Value::from(target.as_str())),
                ("severity".to_string(), Value::from(severity.as_str())),
                ("minutes".to_string(), Value::from(cooldown_minutes)),
                ("reason".to_string(), Value::from(reason.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("创建冷却记录失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"id": result as i64, "ruleId": rule_id, "cooldownMinutes": cooldown_minutes}}))
    }

    /// 清除冷却记录 (设为expired)
    async fn cooldown_clear(&self, params: &Value) -> DsaResult<Value> {
        let rule_id = params
            .get("ruleId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if rule_id == 0 {
            return Err(DsaError::Validation("请提供规则ID".to_string()));
        }

        let target = utils::param_string(params, "target");
        if target.is_empty() {
            return Err(DsaError::Validation("请提供target".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "UPDATE alert_cooldowns SET state = 'expired', updatedTime = NOW() \
             WHERE ruleId = :rid AND target = :target AND state = 'active'";
        Helper::execute(
            sql,
            vec![
                ("rid".to_string(), Value::from(rule_id)),
                ("target".to_string(), Value::from(target.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("清除冷却记录失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"ruleId": rule_id, "target": target, "state": "expired"}}))
    }

    /// 通知投递日志
    async fn notification_log(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let sql = "SELECT id, triggerId, channel, attempt, success, errorCode, \
             retryable, latencyMs, diagnostics, createTime \
             FROM alert_notifications \
             ORDER BY createTime DESC LIMIT :limit";
        let rows = Helper::query_rows(sql, vec![("limit".to_string(), Value::from(limit))], &connector)
            .map_err(|e| DsaError::Database(format!("查询通知投递日志失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(value!({"status": "ok", "data": results}))
    }

    /// 创建通知投递记录
    async fn notification_log_create(&self, params: &Value) -> DsaResult<Value> {
        let trigger_id = params
            .get("triggerId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if trigger_id == 0 {
            return Err(DsaError::Validation("请提供触发ID".to_string()));
        }

        let channel = utils::param_string(params, "channel");
        if channel.is_empty() {
            return Err(DsaError::Validation("请提供通知渠道".to_string()));
        }

        let success = params
            .get("success")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let error_code = utils::param_string(params, "errorCode");
        let retryable = params
            .get("retryable")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let latency_ms = params
            .get("latencyMs")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let diagnostics = utils::param_string(params, "diagnostics");

        let connector = utils::get_db_connector()?;
        let sql = "INSERT INTO alert_notifications \
             (triggerId, channel, attempt, success, errorCode, retryable, latencyMs, diagnostics, createTime) \
             VALUES (:tid, :channel, 1, :success, :ec, :retry, :lat, :diag, NOW())";

        let result = Helper::execute(
            sql,
            vec![
                ("tid".to_string(), Value::from(trigger_id)),
                ("channel".to_string(), Value::from(channel.as_str())),
                ("success".to_string(), Value::from(success)),
                ("ec".to_string(), Value::from(error_code.as_str())),
                ("retry".to_string(), Value::from(retryable)),
                ("lat".to_string(), Value::from(latency_ms)),
                ("diag".to_string(), Value::from(diagnostics.as_str())),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("创建通知投递记录失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": {"id": result as i64, "triggerId": trigger_id, "channel": channel}}))
    }

    /// 检查所有启用的告警规则
    async fn check_all(&self, _params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;

        // 查询所有启用的规则
        let sql = "SELECT id, stockCode, stockName, ruleType, conditionJson, \
             enabled, lastTriggeredAt, triggerCount \
             FROM alert_rules WHERE enabled = 1 AND status >= 1";
        let rules = Helper::query_rows(sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询告警规则失败: {}", e)))?;

        let mut triggered_count: i64 = 0;
        let real = Real::new();

        for rule_row in &rules {
            let rule_id: i64 = rule_row.get_value(0).as_f64().unwrap_or(0.0) as i64;
            let stock_code = rule_row.get_string(1);
            let condition_json = rule_row.get_string(4);

            if stock_code.is_empty() {
                continue;
            }

            // 获取实时行情
            let prefix = utils::market_prefix(&stock_code);
            let quote = match real
                .get_price(&format!("{}{}", prefix, stock_code))
                .await
            {
                Ok(q) => q,
                Err(_) => continue,
            };

            let current_price = quote.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let change_pct = quote
                .get("changePercent")
                .or_else(|| quote.get("change_pct"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            // 解析条件
            let condition: Value = serde_json::from_str(&condition_json).unwrap_or(value!({}));
            let price_above = condition
                .get("priceAbove")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let price_below = condition
                .get("priceBelow")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let change_above = condition
                .get("changeAbove")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let change_below = condition
                .get("changeBelow")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let mut triggered = false;
            let mut reasons = Vec::new();

            if price_above > 0.0 && current_price >= price_above {
                triggered = true;
                reasons.push(format!("价格{:.2}>={:.2}", current_price, price_above));
            }
            if price_below > 0.0 && current_price <= price_below {
                triggered = true;
                reasons.push(format!("价格{:.2}<={:.2}", current_price, price_below));
            }
            if change_above > 0.0 && change_pct >= change_above {
                triggered = true;
                reasons.push(format!("涨跌幅{:.2}%>={:.2}%", change_pct, change_above));
            }
            if change_below < 0.0 && change_pct <= change_below {
                triggered = true;
                reasons.push(format!("涨跌幅{:.2}%<={:.2}%", change_pct, change_below));
            }

            if !triggered {
                continue;
            }

            let reason_str = reasons.join("; ");

            // 插入触发记录
            let trigger_sql = "INSERT INTO alert_triggers \
                 (ruleId, stockCode, triggerType, triggerValue, conditionSnapshot, notified, status, createTime) \
                 VALUES (:rid, :code, 'price', :val, :cond, 1, 1, NOW())";
            let trigger_result = Helper::execute(
                trigger_sql,
                vec![
                    ("rid".to_string(), Value::from(rule_id)),
                    ("code".to_string(), Value::from(stock_code.as_str())),
                    ("val".to_string(), Value::from(current_price)),
                    ("cond".to_string(), Value::from(condition_json.as_str())),
                ],
                &connector,
            );

            if let Ok(trigger_id) = trigger_result {
                // 创建通知投递记录
                let _ = Helper::execute(
                    "INSERT INTO alert_notifications \
                     (triggerId, channel, attempt, success, errorCode, retryable, latencyMs, diagnostics, createTime) \
                     VALUES (:tid, 'system', 1, 1, '', 0, 0, :diag, NOW())",
                    vec![
                        ("tid".to_string(), Value::from(trigger_id as i64)),
                        ("diag".to_string(), Value::from(reason_str.as_str())),
                    ],
                    &connector,
                );

                // 创建冷却记录
                let _ = Helper::execute(
                    "INSERT INTO alert_cooldowns \
                     (ruleId, ruleKey, target, severity, lastTriggeredAt, cooldownUntil, reason, state, updatedTime) \
                     VALUES (:rid, '', :code, 'warning', NOW(), DATE_ADD(NOW(), INTERVAL 30 MINUTE), :reason, 'active', NOW())",
                    vec![
                        ("rid".to_string(), Value::from(rule_id)),
                        ("code".to_string(), Value::from(stock_code.as_str())),
                        ("reason".to_string(), Value::from(reason_str.as_str())),
                    ],
                    &connector,
                );
            }

            // 更新规则触发计数
            let _ = Helper::execute(
                "UPDATE alert_rules SET lastTriggeredAt = NOW(), triggerCount = triggerCount + 1, modifyTime = NOW() WHERE id = :id",
                vec![("id".to_string(), Value::from(rule_id))],
                &connector,
            );

            triggered_count += 1;
        }

        Ok(value!({
            "status": "ok",
            "data": {
                "totalRules": rules.len() as i64,
                "triggeredCount": triggered_count,
            }
        }))
    }

    /// 市场信号灯 (红黄绿)
    async fn market_light(&self, _params: &Value) -> DsaResult<Value> {
        let real = Real::new();

        let sh = real.get_price("sh000001").await.ok();
        let sz = real.get_price("sz399001").await.ok();
        let cy = real.get_price("sz399006").await.ok();

        let sh_change = sh
            .as_ref()
            .and_then(|v| v.get("changePercent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);
        let sz_change = sz
            .as_ref()
            .and_then(|v| v.get("changePercent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);
        let cy_change = cy
            .as_ref()
            .and_then(|v| v.get("changePercent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);

        // 判断整体信号: 全涨=green, 全跌=red, 混合=yellow
        let up_count = [sh_change, sz_change, cy_change]
            .iter()
            .filter(|&&c| c > 0.0)
            .count();
        let down_count = [sh_change, sz_change, cy_change]
            .iter()
            .filter(|&&c| c < 0.0)
            .count();

        let light = if up_count == 3 {
            "green"
        } else if down_count == 3 {
            "red"
        } else {
            "yellow"
        };

        Ok(value!({
            "status": "ok",
            "data": {
                "light": light,
                "indices": {
                    "上证指数": {
                        "changePercent": sh_change,
                    },
                    "深证成指": {
                        "changePercent": sz_change,
                    },
                    "创业板指": {
                        "changePercent": cy_change,
                    },
                },
                "upCount": up_count as i64,
                "downCount": down_count as i64,
            }
        }))
    }
}
