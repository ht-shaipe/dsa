use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "alert_notifications", comment = "告警通知投递记录", primary = "identity")]
pub struct AlertNotification {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "triggerId", comment = "触发记录ID")]
    pub trigger_id: i64,

    #[field(required = true, comment = "通知渠道: dingtalk/feishu/wecom/telegram/bark/email")]
    pub channel: String,

    #[field(default_value = "1", comment = "重试次数")]
    pub attempt: i32,

    #[field(default_value = "0", comment = "是否成功")]
    pub success: i8,

    #[field(rename = "errorCode", comment = "错误码")]
    pub error_code: String,

    #[field(default_value = "0", comment = "是否可重试")]
    pub retryable: i8,

    #[field(rename = "latencyMs", comment = "延迟ms")]
    pub latency_ms: i32,

    #[field(comment = "诊断信息")]
    pub diagnostics: String,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
