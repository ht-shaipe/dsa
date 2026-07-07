//! LLM使用记录表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "llm_usage", comment = "LLM使用记录", primary = "identity")]
pub struct LlmUsage {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "llmProvider", comment = "LLM提供商")]
    pub llm_provider: String,

    #[field(required = true, rename = "llmModel", comment = "LLM模型")]
    pub llm_model: String,

    #[field(rename = "operationType", comment = "操作类型")]
    pub operation_type: String,

    #[field(rename = "promptTokens", default_value = "0")]
    pub prompt_tokens: i32,

    #[field(rename = "completionTokens", default_value = "0")]
    pub completion_tokens: i32,

    #[field(rename = "totalTokens", default_value = "0")]
    pub total_tokens: i32,

    #[field(rename = "cacheHit", default_value = "0", comment = "是否缓存命中")]
    pub cache_hit: i8,

    #[field(rename = "latencyMs", default_value = "0", comment = "延迟ms")]
    pub latency_ms: i64,

    #[field(rename = "stockCode", comment = "关联股票代码")]
    pub stock_code: String,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "callType", length = 32, default_value = "'analysis'", comment = "调用类型")]
    pub call_type: String,

    #[field(length = 64, default_value = "''", comment = "提供商")]
    pub provider: String,

    #[field(rename = "costEstimate", default_value = "0", comment = "费用估算")]
    pub cost_estimate: f64,

    #[field(rename = "requestId", length = 64, default_value = "''", comment = "请求ID")]
    pub request_id: String,

    #[field(rename = "errorMessage", default_value = "''", comment = "错误信息")]
    pub error_message: String,
}
