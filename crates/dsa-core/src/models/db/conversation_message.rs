//! Agent对话消息表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "conversation_messages", comment = "Agent对话消息", primary = "identity")]
pub struct ConversationMessage {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "sessionId", comment = "会话ID")]
    pub session_id: String,

    #[field(required = true, comment = "角色: user/assistant/system")]
    pub role: String,

    #[field(required = true, comment = "内容")]
    pub content: String,

    #[field(rename = "llmProvider", comment = "LLM提供商")]
    pub llm_provider: String,

    #[field(rename = "llmModel", comment = "LLM模型")]
    pub llm_model: String,

    #[field(rename = "promptTokens", default_value = "0")]
    pub prompt_tokens: i32,

    #[field(rename = "completionTokens", default_value = "0")]
    pub completion_tokens: i32,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
