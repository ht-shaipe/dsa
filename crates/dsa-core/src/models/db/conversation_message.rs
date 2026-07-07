//! Agent对话消息表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "conversation_messages", comment = "Agent对话消息", primary = "identity")]
pub struct ConversationMessage {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "session_id", comment = "会话ID")]
    pub session_id: String,

    #[field(required = true, comment = "角色: user/assistant/system")]
    pub role: String,

    #[field(required = true, r#type = "text", comment = "内容")]
    pub content: String,

    #[field(rename = "llm_provider", comment = "LLM提供商")]
    pub llm_provider: String,

    #[field(rename = "llm_model", comment = "LLM模型")]
    pub llm_model: String,

    #[field(rename = "prompt_tokens", default_value = "0")]
    pub prompt_tokens: i32,

    #[field(rename = "completion_tokens", default_value = "0")]
    pub completion_tokens: i32,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
