use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(
    name = "alert_triggers",
    comment = "告警触发记录",
    primary = "identity"
)]
pub struct AlertTrigger {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "rule_id", comment = "关联规则ID")]
    pub rule_id: i64,

    #[field(required = true, rename = "stock_code", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "trigger_type", comment = "触发类型")]
    pub trigger_type: String,

    #[field(rename = "trigger_value", comment = "触发值")]
    pub trigger_value: f64,

    #[field(rename = "condition_snapshot", comment = "条件快照JSON")]
    pub condition_snapshot: String,

    #[field(rename = "notified", default_value = "0", comment = "是否已通知")]
    pub notified: i8,

    #[field(default_value = "1", comment = "状态")]
    pub status: i8,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(length = 64, default_value = "''", comment = "目标")]
    pub target: String,

    #[field(rename = "observed_value", default_value = "0", comment = "观测值")]
    pub observed_value: f64,

    #[field(default_value = "0", comment = "阈值")]
    pub threshold: f64,

    #[field(default_value = "''", comment = "原因")]
    pub reason: String,

    #[field(
        rename = "data_source",
        length = 64,
        default_value = "''",
        comment = "数据源"
    )]
    pub data_source: String,

    #[field(rename = "data_timestamp", comment = "数据时间戳")]
    pub data_timestamp: Option<chrono::NaiveDateTime>,

    #[field(
        rename = "trigger_status",
        length = 16,
        default_value = "'fired'",
        comment = "触发状态"
    )]
    pub trigger_status: String,
}
