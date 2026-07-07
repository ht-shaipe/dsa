use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "alert_cooldowns", comment = "告警冷却状态", primary = "identity")]
pub struct AlertCooldown {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "ruleId", comment = "规则ID")]
    pub rule_id: i64,

    #[field(rename = "ruleKey", comment = "规则键(非DB规则)")]
    pub rule_key: String,

    #[field(required = true, comment = "目标(股票代码等)")]
    pub target: String,

    #[field(default_value = "'warning'", comment = "严重级别")]
    pub severity: String,

    #[field(rename = "lastTriggeredAt", comment = "上次触发时间")]
    pub last_triggered_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "cooldownUntil", comment = "冷却至")]
    pub cooldown_until: Option<chrono::NaiveDateTime>,

    #[field(comment = "原因")]
    pub reason: String,

    #[field(default_value = "'active'", comment = "状态")]
    pub state: String,

    #[field(rename = "updatedTime", default_value = "current_timestamp()")]
    pub updated_time: Option<chrono::NaiveDateTime>,
}
