//! 组合Agent - 考虑现有持仓、配置、相关性的组合分析

use async_trait::async_trait;
use dsa_core::{DsaError, DsaResult};
use tube::Value;

use ai_llm_kit::LlmService;

use super::base_agent::BaseAgent;

/// 组合管理Agent
pub struct PortfolioAgent {
    llm: Box<dyn LlmService>,
    model: String,
}

impl PortfolioAgent {
    pub fn new(llm: Box<dyn LlmService>, model: &str) -> Self {
        Self {
            llm,
            model: model.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl BaseAgent for PortfolioAgent {
    fn name(&self) -> &str {
        "portfolio"
    }
    fn role(&self) -> &str {
        "组合管理专家"
    }

    async fn process(&self, input: &Value) -> DsaResult<Value> {
        let code = input
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or_default();

        if code.is_empty() {
            return Err(DsaError::Validation("组合分析需要股票代码".to_string()));
        }

        // 获取决策结果
        let decision = input
            .get("decision")
            .and_then(|d| d.as_str())
            .unwrap_or_default();

        // 获取现有持仓（如有）
        let positions = input
            .get("positions")
            .and_then(|p| Value::as_array(p))
            .unwrap_or_default();

        // 获取总资产
        let total_assets = input.get("totalAssets").and_then(|a| a.as_f64());
        let total_assets_str = match total_assets {
            Some(v) => format!("{:.0}元", v),
            None => "未知（无持仓数据）".to_string(),
        };

        let positions_summary = if positions.is_empty() {
            "当前无持仓".to_string()
        } else {
            let summaries: Vec<String> = positions
                .iter()
                .take(10)
                .filter_map(|p| {
                    let pos_code = p.get("code").and_then(|c| c.as_str()).unwrap_or_default();
                    let pos_name = p.get("name").and_then(|n| n.as_str()).unwrap_or_default();
                    let pos_pct = p
                        .get("positionPercent")
                        .and_then(|pp| pp.as_f64())
                        .unwrap_or(0.0);
                    Some(format!("{}({}): {:.1}%", pos_code, pos_name, pos_pct))
                })
                .collect();
            format!("当前持仓: {}", summaries.join(", "))
        };

        let system_prompt = "你是一位资深投资组合管理专家，擅长从组合配置的角度评估个股，考虑相关性、集中度、行业分布等维度。请基于提供的组合信息给出专业的组合建议。用中文回答。";

        let user_prompt = format!(
            "请从组合管理角度评估股票{}：\n\
             总资产：{:.0}元\n\
             {}\n\
             \n\
             决策建议参考：\n{}\n\
             \n\
             请从以下维度分析：\n\
             1. 组合集中度（是否过度集中某一行业/个股）\n\
             2. 相关性风险（与现有持仓的相关性）\n\
             3. 行业配置平衡\n\
             4. 仓位分配建议\n\
             \n\
             请以JSON格式返回分析结果：\n\
             - addToPortfolio: 是否建议加入组合(true/false)\n\
             - portfolioFit: 组合适配度(0-100)\n\
             - concentrationRisk: 集中度风险(low/medium/high)\n\
             - correlationRisk: 相关性风险(low/medium/high)\n\
             - sectorBalance: 行业配置评估(balanced/unbalanced)\n\
             - suggestedWeight: 建议权重(0-100%)\n\
             - rebalanceAdvice: 调仓建议\n\
             - summary: 组合建议总结(200字以内)",
            code, total_assets_str, positions_summary, decision
        );

        let body = value!({
            "model": &self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.3,
        });

        let start = std::time::Instant::now();
        let response = self
            .llm
            .chat(&body)
            .await
            .map_err(|e| DsaError::LlmAnalysis(format!("组合Agent调用LLM失败: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as i64;

        let conf = dsa_core::get_global_config();
        dsa_core::utils::record_llm_usage_from_response(
            &response,
            &conf.llm.provider,
            &self.model,
            "agent_portfolio",
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
            "portfolioAnalysis": content,
        }))
    }
}
