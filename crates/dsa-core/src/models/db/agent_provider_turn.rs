use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "agent_provider_turns", comment = "Agent Provider协议追踪", primary = "identity")]
pub struct AgentProviderTurn {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "session_id", required = true, comment = "会话ID")]
    pub session_id: String,

    #[field(rename = "run_id", required = true, comment = "运行ID")]
    pub run_id: String,

    #[field(required = true, comment = "Provider名称")]
    pub provider: String,

    #[field(required = true, comment = "模型名")]
    pub model: String,

    #[field(rename = "anchor_user_message_id", default_value = "0", comment = "锚点用户消息ID")]
    pub anchor_user_message_id: i64,

    #[field(rename = "anchor_assistant_message_id", default_value = "0", comment = "锚点助手消息ID")]
    pub anchor_assistant_message_id: i64,

    #[field(rename = "messages_json", comment = "消息JSON")]
    pub messages_json: String,

    #[field(rename = "contains_reasoning", default_value = "0", comment = "包含推理")]
    pub contains_reasoning: i8,

    #[field(rename = "contains_tool_calls", default_value = "0", comment = "包含工具调用")]
    pub contains_tool_calls: i8,

    #[field(rename = "contains_thinking_blocks", default_value = "0", comment = "包含thinking块")]
    pub contains_thinking_blocks: i8,

    #[field(rename = "must_roundtrip", default_value = "0", comment = "需要roundtrip")]
    pub must_roundtrip: i8,

    #[field(rename = "estimated_tokens", default_value = "0", comment = "预估token")]
    pub estimated_tokens: i32,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
