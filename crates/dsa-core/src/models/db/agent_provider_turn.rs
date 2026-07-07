use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "agent_provider_turns", comment = "Agent Provider协议追踪", primary = "identity")]
pub struct AgentProviderTurn {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "sessionId", required = true, comment = "会话ID")]
    pub session_id: String,

    #[field(rename = "runId", required = true, comment = "运行ID")]
    pub run_id: String,

    #[field(required = true, comment = "Provider名称")]
    pub provider: String,

    #[field(required = true, comment = "模型名")]
    pub model: String,

    #[field(rename = "anchorUserMessageId", default_value = "0", comment = "锚点用户消息ID")]
    pub anchor_user_message_id: i64,

    #[field(rename = "anchorAssistantMessageId", default_value = "0", comment = "锚点助手消息ID")]
    pub anchor_assistant_message_id: i64,

    #[field(rename = "messagesJson", comment = "消息JSON")]
    pub messages_json: String,

    #[field(rename = "containsReasoning", default_value = "0", comment = "包含推理")]
    pub contains_reasoning: i8,

    #[field(rename = "containsToolCalls", default_value = "0", comment = "包含工具调用")]
    pub contains_tool_calls: i8,

    #[field(rename = "containsThinkingBlocks", default_value = "0", comment = "包含thinking块")]
    pub contains_thinking_blocks: i8,

    #[field(rename = "mustRoundtrip", default_value = "0", comment = "需要roundtrip")]
    pub must_roundtrip: i8,

    #[field(rename = "estimatedTokens", default_value = "0", comment = "预估token")]
    pub estimated_tokens: i32,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
