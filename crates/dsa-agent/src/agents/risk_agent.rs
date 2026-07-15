//! 风控Agent - 评估市场风险、仓位控制、止损建议

use async_trait::async_trait;
use dsa_core::{DsaError, DsaResult};
use tube::Value;

use ai_llm_kit::LlmService;

use super::base_agent::BaseAgent;

/// 风控Agent
pub struct RiskAgent {
    llm: Box<dyn LlmService>,
    model: String,
}

impl RiskAgent {
    pub fn new(llm: Box<dyn LlmService>, model: &str) -> Self {
        Self {
            llm,
            model: model.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl BaseAgent for RiskAgent {
    fn name(&self) -> &str {
        "risk"
    }
    fn role(&self) -> &str {
        "风险管理专家"
    }

    async fn process(&self, input: &Value) -> DsaResult<Value> {
        let code = input
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or_default();

        if code.is_empty() {
            return Err(DsaError::Validation("风控分析需要股票代码".to_string()));
        }

        let current_price = input
            .get("currentPrice")
            .and_then(|p| p.as_f64())
            .unwrap_or(0.0);
        let change_pct = input
            .get("changePercent")
            .and_then(|c| c.as_f64())
            .unwrap_or(0.0);
        let turnover_rate = input
            .get("turnoverRate")
            .and_then(|t| t.as_f64())
            .unwrap_or(0.0);
        let market_trend = input
            .get("marketTrend")
            .and_then(|m| m.as_str())
            .unwrap_or_default();

        // 收集技术分析结果（如有）
        let technical = input
            .get("technical")
            .and_then(|t| t.as_str())
            .unwrap_or_default();

        let system_prompt = "你是一位资深风险管理专家，擅长评估投资风险、制定仓位管理策略和止损止盈方案。请根据提供的数据给出专业的风险评估和管理建议。用中文回答。";

        let user_prompt = format!(
            "请对股票{}进行风险评估：\n\
             当前价格：{:.2}\n\
             当日涨跌幅：{:.2}%\n\
             换手率：{:.2}%\n\
             大盘趋势：{}\n\
             \n\
             技术分析参考：\n{}\n\
             \n\
             请从以下维度评估风险：\n\
             1. 市场风险（大盘系统性风险）\n\
             2. 个股波动风险（振幅、换手率）\n\
             3. 流动性风险（成交量、换手率）\n\
             4. 仓位管理建议\n\
             5. 止损止盈价位建议\n\
             \n\
             请以JSON格式返回评估结果：\n\
             - riskScore: 风险评分(0-100, 100为最高风险)\n\
             - riskLevel: 风险等级(low/medium/high/extreme)\n\
             - marketRisk: 市场风险评估\n\
             - volatilityRisk: 波动风险评估\n\
             - liquidityRisk: 流动性风险评估\n\
             - suggestedPosition: 建议仓位比例(0-100%)\n\
             - stop_loss_price: 建议止损价\n\
             - take_profit_price: 建议止盈价\n\
             - maxLossAmount: 最大可承受亏损金额(按10万本金)\n\
             - riskWarnings: 风险提示数组\n\
             - summary: 风险评估总结(200字以内)",
            code, current_price, change_pct, turnover_rate, market_trend, technical
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
            .map_err(|e| DsaError::LlmAnalysis(format!("风控Agent调用LLM失败: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as i64;

        let conf = dsa_core::get_global_config();
        dsa_core::utils::record_llm_usage_from_response(
            &response,
            &conf.llm.provider,
            &self.model,
            "agent_risk",
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
            "riskAnalysis": content,
        }))
    }
}
