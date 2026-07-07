use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "alert_rules", comment = "告警规则", primary = "identity")]
pub struct AlertRule {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "stockCode", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "stockName", comment = "股票名称")]
    pub stock_name: String,

    #[field(required = true, rename = "ruleType", comment = "规则类型: price/change/volume")]
    pub rule_type: String,

    #[field(rename = "conditionJson", comment = "条件JSON")]
    pub condition_json: String,

    #[field(default_value = "1", comment = "是否启用")]
    pub enabled: i8,

    #[field(rename = "lastTriggeredAt", comment = "最后触发时间")]
    pub last_triggered_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "triggerCount", default_value = "0", comment = "触发次数")]
    pub trigger_count: i32,

    #[field(default_value = "1", comment = "状态")]
    pub status: i8,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modifyTime", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,

    #[field(length = 64, default_value = "''", comment = "规则名称")]
    pub name: String,

    #[field(rename = "targetScope", length = 32, default_value = "'stock'", comment = "目标范围")]
    pub target_scope: String,

    #[field(length = 64, default_value = "''", comment = "目标")]
    pub target: String,

    #[field(rename = "alertType", length = 32, default_value = "''", comment = "告警类型")]
    pub alert_type: String,

    #[field(default_value = "''", comment = "参数JSON")]
    pub parameters: String,

    #[field(length = 16, default_value = "'info'", comment = "严重程度")]
    pub severity: String,

    #[field(length = 16, default_value = "'system'", comment = "来源")]
    pub source: String,

    #[field(rename = "cooldownPolicy", default_value = "''", comment = "冷却策略")]
    pub cooldown_policy: String,

    #[field(rename = "notificationPolicy", default_value = "''", comment = "通知策略")]
    pub notification_policy: String,
}
