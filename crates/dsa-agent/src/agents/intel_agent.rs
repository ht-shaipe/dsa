//! 情报Agent - 分析新闻和市场情报

use async_trait::async_trait;
use dsa_core::{DsaError, DsaResult};
use tube::Value;

use ai_llm_kit::LlmService;

use super::base_agent::BaseAgent;

/// 情报分析Agent
pub struct IntelAgent {
    llm: Box<dyn LlmService>,
    model: String,
}

impl IntelAgent {
    pub fn new(llm: Box<dyn LlmService>, model: &str) -> Self {
        Self { llm, model: model.to_string() }
    }
}

#[async_trait(?Send)]
impl BaseAgent for IntelAgent {
    fn name(&self) -> &str { "intel" }
    fn role(&self) -> &str { "情报分析专家" }

    async fn process(&self, input: &Value) -> DsaResult<Value> {
        let code = input.get("code")
            .and_then(|c| c.as_str())
            .unwrap_or_default();

        if code.is_empty() {
            return Err(DsaError::Validation("情报分析需要股票代码".to_string()));
        }

        // 获取新闻数据（如有）
        let news = input.get("news")
            .and_then(|n| Value::as_array(n))
            .unwrap_or_default();

        // 获取行业信息（如有）
        let industry = input.get("industry")
            .and_then(|i| i.as_str())
            .unwrap_or_default();

        let news_summary = if news.is_empty() {
            "暂无近期新闻数据".to_string()
        } else {
            let headlines: Vec<String> = news.iter()
                .take(5)
                .filter_map(|n| n.get("title").and_then(|t| t.as_str()))
                .collect();
            format!("近期新闻: {}", headlines.join("; "))
        };

        let system_prompt = "你是一位资深市场情报分析专家，擅长从新闻资讯、行业动态、政策变化中提炼对股价有影响的关键信息。请基于提供的情报数据给出专业的分析判断。用中文回答。";

        let user_prompt = format!(
            "请对股票{}进行情报面分析：\n\
             行业：{}\n\
             {}\n\
             \n\
             请从以下维度分析：\n\
             1. 新闻面分析（重大利好/利空/中性）\n\
             2. 行业景气度（高景气/一般/低迷）\n\
             3. 政策影响（利好政策/利空政策/无显著影响）\n\
             4. 市场情绪（乐观/悲观/中性）\n\
             \n\
             请以JSON格式返回分析结果，包含以下字段：\n\
             - newsImpact: 新闻面影响(positive/negative/neutral)\n\
             - industryOutlook: 行业景气度(high/medium/low)\n\
             - policyImpact: 政策影响(positive/negative/neutral)\n\
             - marketSentiment: 市场情绪(optimistic/pessimistic/neutral)\n\
             - intelScore: 情报面综合评分(0-100)\n\
             - keyFactors: 关键影响因素数组\n\
             - summary: 情报面总结(200字以内)",
            code, industry, news_summary
        );

        let body = value!({
            "model": &self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.3,
        });

        let response = self.llm.chat(&body).await
            .map_err(|e| DsaError::LlmAnalysis(format!("情报Agent调用LLM失败: {}", e)))?;

        let content = response.get("choices")
            .and_then(|c| Value::as_array(c))
            .and_then(|a| a.first().cloned())
            .and_then(|f| f.get("message").cloned())
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .unwrap_or_default();

        Ok(value!({
            "agent": self.name(),
            "code": code,
            "llmAnalysis": content,
        }))
    }
}
