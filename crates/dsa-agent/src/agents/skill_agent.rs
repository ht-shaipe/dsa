//! 技能Agent - 基于技能路由评估交易策略适用性

use dsa_core::{DsaError, DsaResult};
use tube::Value;

use ai_llm_kit::LlmService;

use crate::skills::defaults::default_skills;
use crate::skills::router::SkillRouter;

/// 技能Agent - 使用技能路由评估并LLM综合建议
pub struct SkillAgent {
    llm: Box<dyn LlmService>,
    model: String,
}

impl SkillAgent {
    pub fn new(llm: Box<dyn LlmService>, model: &str) -> Self {
        Self { llm, model: model.to_string() }
    }

    pub async fn evaluate_skills(&self, code: &str, context: &Value) -> DsaResult<Value> {
        let router = SkillRouter::new(default_skills());
        let skill_context = value!({
            "code": code,
            "trend": context.get("trend").and_then(|v| v.as_str()).unwrap_or_default(),
            "volumeSignal": context.get("volumeSignal").and_then(|v| v.as_str()).unwrap_or_default(),
            "changePercent": context.get("changePercent").and_then(|v| v.as_f64()).unwrap_or(0.0),
            "chip_concentration": context.get("chipConcentration").and_then(|v| v.as_f64()).unwrap_or(50.0),
        });

        let routed = router.route(&skill_context);
        let all_scores = router.evaluate_all(&skill_context);

        // 使用LLM综合技能结果生成建议
        let skill_summary: String = all_scores.iter()
            .filter_map(|s| {
                let name = s.get("skill").and_then(|v| v.as_str()).unwrap_or_default();
                let strength = s.get("strength").and_then(|v| v.as_f64()).unwrap_or(0.0);
                if strength > 30.0 {
                    Some(format!("{}: {:.0}", name, strength))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let best_skill = routed.first()
            .and_then(|s| s.get("skill").and_then(|v| v.as_str()))
            .unwrap_or_else(|| "无".to_string());

        let prompt = format!(
            "基于以下技能评估结果，给出综合建议:\n\
             股票: {}\n\
             技能评分: {}\n\
             最匹配技能: {}\n\n\
             请简要说明当前哪个策略最适用以及为什么。",
            code, skill_summary, best_skill,
        );

        let body = value!({
            "model": &self.model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.5,
        });

        let start = std::time::Instant::now();
        let response = self.llm.chat(&body).await
            .map_err(|e| DsaError::LlmAnalysis(format!("技能Agent调用LLM失败: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as i64;

        let conf = dsa_core::get_global_config();
        dsa_core::utils::record_llm_usage_from_response(
            &response, &conf.llm.provider, &self.model, "agent_skill", elapsed, &code,
        );

        let analysis = response.get("choices")
            .and_then(|c| Value::as_array(c))
            .and_then(|a| a.first().cloned())
            .and_then(|f| f.get("message").cloned())
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .unwrap_or_default()
            .to_string();

        Ok(value!({
            "skillAnalysis": analysis,
            "bestSkill": best_skill,
            "routedSkills": routed,
            "allScores": all_scores,
        }))
    }
}
