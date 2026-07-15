//! 决策Agent - 综合技术+情报+风险，给出最终投资决策

use async_trait::async_trait;
use dsa_core::{DsaError, DsaResult};
use tube::Value;

use ai_llm_kit::LlmService;

use super::base_agent::BaseAgent;

/// 决策Agent
pub struct DecisionAgent {
    llm: Box<dyn LlmService>,
    model: String,
}

impl DecisionAgent {
    pub fn new(llm: Box<dyn LlmService>, model: &str) -> Self {
        Self {
            llm,
            model: model.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl BaseAgent for DecisionAgent {
    fn name(&self) -> &str {
        "decision"
    }
    fn role(&self) -> &str {
        "投资决策专家"
    }

    async fn process(&self, input: &Value) -> DsaResult<Value> {
        let code = input
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or_default();

        if code.is_empty() {
            return Err(DsaError::Validation("决策分析需要股票代码".to_string()));
        }

        // 收集各Agent的分析结果
        let technical = input
            .get("technical")
            .and_then(|t| t.as_str())
            .unwrap_or_default();
        let intel = input
            .get("intel")
            .and_then(|i| i.as_str())
            .unwrap_or_default();
        let risk = input
            .get("risk")
            .and_then(|r| r.as_str())
            .unwrap_or_default();

        let current_price = input
            .get("currentPrice")
            .and_then(|p| p.as_f64())
            .unwrap_or(0.0);

        let system_prompt = "你是一位资深投资决策专家，擅长综合技术面、情报面和风险面分析，给出明确的投资决策建议。你的决策必须具体、可执行，包含明确的操作方向、仓位建议和价格目标。用中文回答。";

        let user_prompt = format!(
            "请对股票{}做出综合投资决策：\n\
             当前价格：{:.2}\n\
             \n\
             技术面分析：\n{}\n\
             \n\
             情报面分析：\n{}\n\
             \n\
             风险面分析：\n{}\n\
             \n\
             请综合以上分析，给出最终投资决策，以JSON格式返回：\n\
             - action: 操作建议(buy/sell/hold)\n\
             - confidence: 决策信心度(0-100)\n\
             - score: 综合评分(0-100)\n\
             - positionSize: 建议仓位比例(0-100%)\n\
             - target_price: 目标价格\n\
             - stopLoss: 止损价格\n\
             - takeProfit: 止盈价格\n\
             - reasoning: 决策理由(300字以内)\n\
             - keyRisks: 主要风险点数组\n\
             - timeframe: 建议持有周期(short/medium/long)",
            code, current_price, technical, intel, risk
        );

        let body = value!({
            "model": &self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.2,
        });

        let start = std::time::Instant::now();
        let response = self
            .llm
            .chat(&body)
            .await
            .map_err(|e| DsaError::LlmAnalysis(format!("决策Agent调用LLM失败: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as i64;

        let conf = dsa_core::get_global_config();
        dsa_core::utils::record_llm_usage_from_response(
            &response,
            &conf.llm.provider,
            &self.model,
            "agent_decision",
            elapsed,
            &code,
        );

        let content = response
            .get("choices")
            .and_then(|c| Value::as_array(c))
            .and_then(|a| a.first().cloned())
            .and_then(|f| f.get("message").cloned())
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .unwrap_or_default();

        Ok(value!({
            "agent": self.name(),
            "code": code,
            "decision": content,
        }))
    }
}
