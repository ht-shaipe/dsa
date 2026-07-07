use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "conversation_summaries", comment = "对话摘要压缩", primary = "identity")]
pub struct ConversationSummary {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "session_id", required = true, comment = "会话ID")]
    pub session_id: String,

    #[field(required = true, r#type = "text", comment = "摘要文本")]
    pub summary: String,

    #[field(rename = "covered_message_id", default_value = "0", comment = "已覆盖的最后消息ID")]
    pub covered_message_id: i64,

    #[field(rename = "source_message_count", default_value = "0", comment = "源消息数")]
    pub source_message_count: i32,

    #[field(rename = "estimated_tokens", default_value = "0", comment = "预估token数")]
    pub estimated_tokens: i32,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
