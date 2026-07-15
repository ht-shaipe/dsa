use dsa_core::db::{execute, query_rows, row_get_f64, row_get_string};
use dsa_core::models::db::AlertCooldown as AlertCooldownModel;
use dsa_core::models::db::AlertNotification as AlertNotificationModel;
use dsa_core::models::db::AlertRule as AlertRuleModel;
use dsa_core::models::db::AlertTrigger as AlertTriggerModel;
use dsa_core::utils;
use deck::sqlite::DataTable;
use deck::TableService;

use qta_crawler::Real;
use tube::{Result, Value};
use tube_web::RequestParameter;

struct TriggerTable { request: RequestParameter }

impl DataTable<AlertTriggerModel> for TriggerTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<AlertTriggerModel> for TriggerTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl TriggerTable {
    fn new(param: &RequestParameter) -> Self { TriggerTable { request: param.clone() } }

    fn query_triggers(&self, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, rule_id, stock_code, trigger_type, trigger_value, \
             condition_snapshot, notified, create_time \
             FROM alert_triggers WHERE status >= 1 \
             ORDER BY create_time DESC LIMIT :limit";
        query_rows(sql, vec![("limit".to_string(), Value::from(limit))], &connector)
            .map_err(|e| tube::Error::msg(format!("查询触发历史失败: {}", e)))
    }

    fn query_notifications(&self, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, rule_id, stock_code, trigger_type, trigger_value, \
             condition_snapshot, create_time \
             FROM alert_triggers WHERE status >= 1 AND notified = 1 \
             ORDER BY create_time DESC LIMIT :limit";
        query_rows(sql, vec![("limit".to_string(), Value::from(limit))], &connector)
            .map_err(|e| tube::Error::msg(format!("查询通知列表失败: {}", e)))
    }

    fn insert_trigger(&self, rule_id: i64, stock_code: &str, current_price: f64, condition_json: &str) -> Result<i64> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "INSERT INTO alert_triggers \
             (rule_id, stock_code, trigger_type, trigger_value, condition_snapshot, notified, status, create_time) \
             VALUES (:rid, :code, 'price', :val, :cond, 1, 1, NOW())";
        let result = execute(
            sql,
            vec![
                ("rid".to_string(), Value::from(rule_id)),
                ("code".to_string(), Value::from(stock_code)),
                ("val".to_string(), Value::from(current_price)),
                ("cond".to_string(), Value::from(condition_json)),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("插入触发记录失败: {}", e)))?;
        Ok(result as i64)
    }
}

struct NotificationTable { request: RequestParameter }

impl DataTable<AlertNotificationModel> for NotificationTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<AlertNotificationModel> for NotificationTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl NotificationTable {
    fn new(param: &RequestParameter) -> Self { NotificationTable { request: param.clone() } }

    fn query_notification_log(&self, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, trigger_id, channel, attempt, success, error_code, \
             retryable, latency_ms, diagnostics, create_time \
             FROM alert_notifications \
             ORDER BY create_time DESC LIMIT :limit";
        let rows = query_rows(sql, vec![("limit".to_string(), Value::from(limit))], &connector)
            .map_err(|e| tube::Error::msg(format!("查询通知投递日志失败: {}", e)))?;
        Ok(rows)
    }

    fn insert_notification_log(&self, trigger_id: i64, channel: &str, success: i64,
                               error_code: &str, retryable: i64, latency_ms: i64, diagnostics: &str) -> Result<Value> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "INSERT INTO alert_notifications \
             (trigger_id, channel, attempt, success, error_code, retryable, latency_ms, diagnostics, create_time) \
              VALUES (:tid, :channel, 1, :success, :ec, :retry, :lat, :diag, NOW())";
        let result = execute(
            sql,
            vec![
                ("tid".to_string(), Value::from(trigger_id)),
                ("channel".to_string(), Value::from(channel)),
                ("success".to_string(), Value::from(success)),
                ("ec".to_string(), Value::from(error_code)),
                ("retry".to_string(), Value::from(retryable)),
                ("lat".to_string(), Value::from(latency_ms)),
                ("diag".to_string(), Value::from(diagnostics)),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("创建通知投递记录失败: {}", e)))?;
        Ok(value!({"id": result as i64, "triggerId": trigger_id, "channel": channel}))
    }

    fn insert_delivery_record(&self, trigger_id: i64, reason_str: &str) -> Result<()> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let _ = execute(
            "INSERT INTO alert_notifications \
             (trigger_id, channel, attempt, success, error_code, retryable, latency_ms, diagnostics, create_time) \
             VALUES (:tid, 'system', 1, 1, '', 0, 0, :diag, NOW())",
            vec![
                ("tid".to_string(), Value::from(trigger_id)),
                ("diag".to_string(), Value::from(reason_str)),
            ],
            &connector,
        );
        Ok(())
    }
}

struct CooldownTable { request: RequestParameter }

impl DataTable<AlertCooldownModel> for CooldownTable {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}
impl TableService<AlertCooldownModel> for CooldownTable {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}
impl CooldownTable {
    fn new(param: &RequestParameter) -> Self { CooldownTable { request: param.clone() } }

    fn query_cooldowns(&self, limit: i64) -> Result<Vec<Value>> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, rule_id, rule_key, target, severity, last_triggered_at, \
             cooldown_until, reason, state, updated_time \
             FROM alert_cooldowns WHERE state = 'active' \
             ORDER BY cooldown_until ASC LIMIT :limit";
        query_rows(sql, vec![("limit".to_string(), Value::from(limit))], &connector)
            .map_err(|e| tube::Error::msg(format!("查询冷却记录失败: {}", e)))
    }

    fn insert_cooldown(&self, rule_id: i64, target: &str, severity: &str, cooldown_minutes: i64, reason: &str) -> Result<Value> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "INSERT INTO alert_cooldowns \
             (rule_id, rule_key, target, severity, last_triggered_at, cooldown_until, reason, state, updated_time) \
             VALUES (:rid, '', :target, :severity, NOW(), DATE_ADD(NOW(), INTERVAL :minutes MINUTE), :reason, 'active', NOW())";
        let result = execute(
            sql,
            vec![
                ("rid".to_string(), Value::from(rule_id)),
                ("target".to_string(), Value::from(target)),
                ("severity".to_string(), Value::from(severity)),
                ("minutes".to_string(), Value::from(cooldown_minutes)),
                ("reason".to_string(), Value::from(reason)),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("创建冷却记录失败: {}", e)))?;
        Ok(value!({"id": result as i64, "ruleId": rule_id, "cooldownMinutes": cooldown_minutes}))
    }

    fn clear_cooldown(&self, rule_id: i64, target: &str) -> Result<Value> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "UPDATE alert_cooldowns SET state = 'expired', updated_time = NOW() \
             WHERE rule_id = :rid AND target = :target AND state = 'active'";
        execute(
            sql,
            vec![
                ("rid".to_string(), Value::from(rule_id)),
                ("target".to_string(), Value::from(target)),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("清除冷却记录失败: {}", e)))?;
        Ok(value!({"ruleId": rule_id, "target": target, "state": "expired"}))
    }

    fn insert_auto_cooldown(&self, rule_id: i64, stock_code: &str, reason_str: &str) -> Result<()> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let _ = execute(
            "INSERT INTO alert_cooldowns \
             (rule_id, rule_key, target, severity, last_triggered_at, cooldown_until, reason, state, updated_time) \
             VALUES (:rid, '', :code, 'warning', NOW(), DATE_ADD(NOW(), INTERVAL 30 MINUTE), :reason, 'active', NOW())",
            vec![
                ("rid".to_string(), Value::from(rule_id)),
                ("code".to_string(), Value::from(stock_code)),
                ("reason".to_string(), Value::from(reason_str)),
            ],
            &connector,
        );
        Ok(())
    }
}

pub struct Alert { request: RequestParameter }

impl DataTable<AlertRuleModel> for Alert {
    fn datasource_key(&self) -> String { crate::DATASOURCE_KEY.to_owned() }
}

impl TableService<AlertRuleModel> for Alert {
    fn value(&self) -> Value { self.request.value.clone() }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) { self.request.get_auth_user() }
}

impl Alert {
    pub fn new(param: &RequestParameter) -> Self {
        Alert { request: param.clone() }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "rules" => self.rules().await,
            "rule_create" => self.rule_create().await,
            "rule_update" => self.rule_update().await,
            "rule_delete" => self.rule_delete().await,
            "rule_enable" => self.rule_enable().await,
            "rule_disable" => self.rule_disable().await,
            "rule_test" => self.rule_test().await,
            "triggers" => self.triggers().await,
            "notifications" => self.notifications().await,
            "cooldowns" => self.cooldowns().await,
            "cooldown_create" => self.cooldown_create().await,
            "cooldown_clear" => self.cooldown_clear().await,
            "notification_log" => self.notification_log().await,
            "notification_log_create" => self.notification_log_create().await,
            "check_all" => self.check_all().await,
            "market_light" => self.market_light().await,
            _ => Err(tube::Error::msg(format!("alert不支持方法: {}", method))),
        }
    }

    fn params(&self) -> &Value {
        &self.request.value
    }

    async fn rules(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;

        let (sql, p) = if code.is_empty() {
            ("SELECT id, stock_code, stock_name, rule_type, condition_json, \
              enabled, last_triggered_at, trigger_count, create_time, modify_time \
              FROM alert_rules WHERE status >= 1 ORDER BY create_time DESC".to_string(),
             vec![])
        } else {
            ("SELECT id, stock_code, stock_name, rule_type, condition_json, \
              enabled, last_triggered_at, trigger_count, create_time, modify_time \
              FROM alert_rules WHERE status >= 1 AND stock_code = :code ORDER BY create_time DESC".to_string(),
             vec![("code".to_string(), Value::from(code.as_str()))])
        };

        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| tube::Error::msg(format!("查询告警规则失败: {}", e)))?;

        Ok(Value::Array(rows))
    }

    async fn rule_create(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(tube::Error::msg("请提供股票代码".to_string()));
        }

        let rule_type = utils::param_string(params, "ruleType");
        if rule_type.is_empty() {
            return Err(tube::Error::msg("请提供规则类型".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let name = utils::param_string(params, "name");
        let condition_json = utils::param_string(params, "condition");

        let sql = "INSERT INTO alert_rules \
             (stock_code, stock_name, rule_type, condition_json, enabled, trigger_count, status, create_time, modify_time) \
             VALUES (:code, :name, :type, :cond, 1, 0, 1, NOW(), NOW())";

        let result = execute(
            sql,
            vec![
                ("code".to_string(), Value::from(code.as_str())),
                ("name".to_string(), Value::from(name.as_str())),
                ("type".to_string(), Value::from(rule_type.as_str())),
                ("cond".to_string(), Value::from(condition_json.as_str())),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("创建告警规则失败: {}", e)))?;

        Ok(value!({"id": result as i64, "code": code, "ruleType": rule_type}))
    }

    async fn rule_update(&self) -> Result<Value> {
        let params = self.params();
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(tube::Error::msg("请提供规则ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let condition_json = utils::param_string(params, "condition");

        let sql = "UPDATE alert_rules SET condition_json = :cond, modify_time = NOW() WHERE id = :id";
        execute(
            sql,
            vec![
                ("cond".to_string(), Value::from(condition_json.as_str())),
                ("id".to_string(), Value::from(id)),
            ],
            &connector,
        ).map_err(|e| tube::Error::msg(format!("更新告警规则失败: {}", e)))?;

        Ok(value!({"id": id}))
    }

    async fn rule_delete(&self) -> Result<Value> {
        let params = self.params();
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(tube::Error::msg("请提供规则ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "UPDATE alert_rules SET status = 0, modify_time = NOW() WHERE id = :id";
        execute(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| tube::Error::msg(format!("删除告警规则失败: {}", e)))?;

        Ok(value!({"id": id}))
    }

    async fn rule_enable(&self) -> Result<Value> {
        let params = self.params();
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(tube::Error::msg("请提供规则ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "UPDATE alert_rules SET enabled = 1, modify_time = NOW() WHERE id = :id";
        execute(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| tube::Error::msg(format!("启用规则失败: {}", e)))?;

        Ok(value!({"id": id, "enabled": true}))
    }

    async fn rule_disable(&self) -> Result<Value> {
        let params = self.params();
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(tube::Error::msg("请提供规则ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "UPDATE alert_rules SET enabled = 0, modify_time = NOW() WHERE id = :id";
        execute(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| tube::Error::msg(format!("禁用规则失败: {}", e)))?;

        Ok(value!({"id": id, "enabled": false}))
    }

    async fn rule_test(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(tube::Error::msg("请提供股票代码".to_string()));
        }

        let condition_json = utils::param_string(params, "condition");
        let condition: Value = serde_json::from_str(&condition_json).unwrap_or(value!({}));

        let real = Real::new();
        let prefix = utils::market_prefix(&code);

        let quote = real
            .get_price(&format!("{}{}", prefix, code))
            .await
            .map_err(|e| tube::Error::msg(format!("获取行情失败: {}", e)))?;

        let current_price = quote.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let change_pct = quote
            .get("changePercent")
            .or_else(|| quote.get("change_pct"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

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
            "triggered": triggered,
            "currentPrice": current_price,
            "changePct": change_pct,
            "reasons": reasons,
        }))
    }

    async fn triggers(&self) -> Result<Value> {
        let params = self.params();
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let table = TriggerTable::new(&self.request);
        let results = table.query_triggers(limit)?;
        Ok(Value::Array(results))
    }

    async fn notifications(&self) -> Result<Value> {
        let params = self.params();
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let table = TriggerTable::new(&self.request);
        let results = table.query_notifications(limit)?;
        Ok(Value::Array(results))
    }

    async fn cooldowns(&self) -> Result<Value> {
        let params = self.params();
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let table = CooldownTable::new(&self.request);
        let results = table.query_cooldowns(limit)?;
        Ok(Value::Array(results))
    }

    async fn cooldown_create(&self) -> Result<Value> {
        let params = self.params();
        let rule_id = params
            .get("ruleId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if rule_id == 0 {
            return Err(tube::Error::msg("请提供规则ID".to_string()));
        }

        let target = utils::param_string(params, "target");
        let severity = utils::param_string(params, "severity");
        let cooldown_minutes = params
            .get("cooldownMinutes")
            .and_then(|v| v.as_f64())
            .unwrap_or(30.0) as i64;
        let reason = utils::param_string(params, "reason");

        let table = CooldownTable::new(&self.request);
        table.insert_cooldown(rule_id, &target, &severity, cooldown_minutes, &reason)
    }

    async fn cooldown_clear(&self) -> Result<Value> {
        let params = self.params();
        let rule_id = params
            .get("ruleId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if rule_id == 0 {
            return Err(tube::Error::msg("请提供规则ID".to_string()));
        }

        let target = utils::param_string(params, "target");
        if target.is_empty() {
            return Err(tube::Error::msg("请提供target".to_string()));
        }

        let table = CooldownTable::new(&self.request);
        table.clear_cooldown(rule_id, &target)
    }

    async fn notification_log(&self) -> Result<Value> {
        let params = self.params();
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let table = NotificationTable::new(&self.request);
        let results = table.query_notification_log(limit)?;
        Ok(Value::Array(results))
    }

    async fn notification_log_create(&self) -> Result<Value> {
        let params = self.params();
        let trigger_id = params
            .get("triggerId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if trigger_id == 0 {
            return Err(tube::Error::msg("请提供触发ID".to_string()));
        }

        let channel = utils::param_string(params, "channel");
        if channel.is_empty() {
            return Err(tube::Error::msg("请提供通知渠道".to_string()));
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

        let table = NotificationTable::new(&self.request);
        table.insert_notification_log(trigger_id, &channel, success, &error_code, retryable, latency_ms, &diagnostics)
    }

    async fn check_all(&self) -> Result<Value> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;

        let sql = "SELECT id, stock_code, stock_name, rule_type, condition_json, \
             enabled, last_triggered_at, trigger_count \
             FROM alert_rules WHERE enabled = 1 AND status >= 1";
        let rules = query_rows(sql, vec![], &connector)
            .map_err(|e| tube::Error::msg(format!("查询告警规则失败: {}", e)))?;

        let mut triggered_count: i64 = 0;
        let real = Real::new();
        let trigger_table = TriggerTable::new(&self.request);
        let notification_table = NotificationTable::new(&self.request);
        let cooldown_table = CooldownTable::new(&self.request);

        for rule_row in &rules {
            let rule_id: i64 = row_get_f64(rule_row, "id") as i64;
            let stock_code = row_get_string(rule_row, "stockCode");
            let condition_json = row_get_string(rule_row, "conditionJson");

            if stock_code.is_empty() {
                continue;
            }

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

            if let Ok(trigger_id) = trigger_table.insert_trigger(rule_id, &stock_code, current_price, &condition_json) {
                let _ = notification_table.insert_delivery_record(trigger_id, &reason_str);
                let _ = cooldown_table.insert_auto_cooldown(rule_id, &stock_code, &reason_str);
            }

            let _ = execute(
                "UPDATE alert_rules SET last_triggered_at = NOW(), trigger_count = trigger_count + 1, modify_time = NOW() WHERE id = :id",
                vec![("id".to_string(), Value::from(rule_id))],
                &connector,
            );

            triggered_count += 1;
        }

        Ok(value!({
            "totalRules": rules.len() as i64,
            "triggeredCount": triggered_count,
        }))
    }

    async fn market_light(&self) -> Result<Value> {
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
        }))
    }
}
