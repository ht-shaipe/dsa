//! 分析流水线核心 - 对齐原项目 pipeline.py

use crate::context_builder::AnalysisContextBuilder;
use crate::prompt_templates::build_analysis_prompt;
use crate::technical::TechnicalAnalyzer;
use ai_llm_kit::{LlmFactory, LlmProvider, LlmService};
use dsa_core::models::AnalysisReport;
use dsa_core::{DsaError, DsaResult};
use std::sync::Arc;
use tube::Value;
use tracing::{info, warn};

pub struct AnalysisPipeline {
    llm: Arc<Box<dyn LlmService>>,
    model: String,
    temperature: f64,
    timeout_secs: u64,
    technical_analyzer: TechnicalAnalyzer,
}

impl AnalysisPipeline {
    pub fn new(provider: &str, api_key: &str, model: &str, temperature: f64, timeout_seconds: u64) -> DsaResult<Self> {
        let llm_provider = LlmProvider::instance(provider)
            .map_err(|e| DsaError::LlmAnalysis(format!("不支持的 LLM provider: {}", e)))?;
        let llm: Box<dyn LlmService> = LlmFactory::create(llm_provider, api_key);

        Ok(Self {
            llm: Arc::new(llm),
            model: model.to_string(),
            temperature,
            timeout_secs: if timeout_seconds > 0 { timeout_seconds } else { 60 },
            technical_analyzer: TechnicalAnalyzer::new(),
        })
    }

    pub async fn analyze_stock(
        &self,
        code: &str,
        name: &str,
        kline_data: &[dsa_core::models::KlineBar],
        realtime: Option<&Value>,
        market_context: Option<&str>,
    ) -> DsaResult<AnalysisReport> {
        info!("开始分析股票: {} ({})", name, code);

        let technical = self.technical_analyzer.calculate(kline_data, realtime);

        let context = AnalysisContextBuilder::build(code, name, kline_data, realtime, &technical, market_context);

        let prompt = build_analysis_prompt(&context);

        let start = std::time::Instant::now();
        let (report, usage_val) = match self.call_llm_and_parse(&prompt).await {
            Ok(r) => r,
            Err(e) => {
                let conf = dsa_core::get_global_config();
                if let (Some(fb_provider), Some(fb_api_key_env), Some(fb_model)) =
                    (&conf.llm.fallback_provider, &conf.llm.fallback_api_key_env, &conf.llm.fallback_model)
                {
                    warn!("主LLM调用失败({}), 尝试回退: {}/{}", e, fb_provider, fb_model);
                    let fb_key = std::env::var(fb_api_key_env).unwrap_or_default();
                    if !fb_key.is_empty() {
                        if let Ok(provider) = LlmProvider::instance(fb_provider) {
                            let fallback_llm: Box<dyn LlmService> = LlmFactory::create(provider, &fb_key);
                            let fallback_pipeline = AnalysisPipeline {
                                llm: Arc::new(fallback_llm),
                                model: fb_model.clone(),
                                temperature: self.temperature,
                                timeout_secs: self.timeout_secs,
                                technical_analyzer: TechnicalAnalyzer::new(),
                            };
                            fallback_pipeline.call_llm_and_parse(&prompt).await?
                        } else {
                            return Err(e);
                        }
                    } else {
                        return Err(e);
                    }
                } else {
                    return Err(e);
                }
            }
        };
        let elapsed = start.elapsed().as_millis() as i64;

        let usage_default = value!({});
        let usage = usage_val.as_ref().unwrap_or(&usage_default);
        let prompt_tokens = usage.get("prompt_tokens").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
        let completion_tokens = usage.get("completion_tokens").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;

        let conf = dsa_core::get_global_config();
        dsa_core::utils::record_llm_usage(
            &conf.llm.provider,
            &self.model,
            "analyze_stock",
            prompt_tokens,
            completion_tokens,
            elapsed,
            code,
        );

        info!(
            "股票 {} ({}) 分析完成: 评分={:?}, 操作={:?}",
            code, name, report.sentiment_score, report.operation_advice
        );

        Ok(report)
    }

    async fn call_llm_and_parse(&self, prompt: &AnalysisPrompt) -> DsaResult<(AnalysisReport, Option<Value>)> {
        let body = value!({
            "model": &self.model,
            "messages": [
                {"role": "system", "content": &prompt.system},
                {"role": "user", "content": &prompt.user}
            ],
            "temperature": self.temperature,
        });

        let response = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            self.llm.chat(&body),
        )
        .await
        .map_err(|_| DsaError::LlmAnalysis(format!("LLM 调用超时 ({}秒)", self.timeout_secs)))?
        .map_err(|e| DsaError::LlmAnalysis(format!("LLM 调用失败: {}", e)))?;

        let usage = response.get("usage").cloned();
        let content = self.extract_content(&response)?;
        let report = self.parse_report(&content)?;

        Ok((report, usage))
    }

    fn extract_content(&self, response: &Value) -> DsaResult<String> {
        if let Some(choices) = response.get("choices") {
            if let Some(first) = choices.as_array().and_then(|a| a.first().cloned()) {
                if let Some(message) = first.get("message") {
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        return Ok(content);
                    }
                }
            }
        }
        Err(DsaError::LlmAnalysis("无法从 LLM 响应中提取内容".to_string()))
    }

    fn parse_report(&self, content: &str) -> DsaResult<AnalysisReport> {
        let json_str = self.extract_json_from_content(content);

        let report: AnalysisReport = serde_json::from_str(&json_str)
            .map_err(|e| {
                warn!("报告 JSON 解析失败: {}", e);
                DsaError::ReportParse(format!("JSON 解析失败: {}", e))
            })?;

        Ok(report)
    }

    fn extract_json_from_content(&self, content: &str) -> String {
        let trimmed = content.trim();

        if trimmed.starts_with('{') {
            if let Some(end) = Self::find_matching_brace(trimmed) {
                return trimmed[..=end].to_string();
            }
            return trimmed.to_string();
        }

        if let Some(start) = trimmed.find('{') {
            let sub = &trimmed[start..];
            if let Some(end) = Self::find_matching_brace(sub) {
                return sub[..=end].to_string();
            }
        }

        if let Some(start) = trimmed.find("```json") {
            let after = &trimmed[start + 7..];
            if let Some(end) = after.find("```") {
                return after[..end].trim().to_string();
            }
        }

        if let Some(start) = trimmed.find("```") {
            let after = &trimmed[start + 3..];
            if let Some(end) = after.find("```") {
                return after[..end].trim().to_string();
            }
        }

        trimmed.to_string()
    }

    fn find_matching_brace(s: &str) -> Option<usize> {
        let mut depth = 0i32;
        for (_i, (byte_idx, ch)) in s.char_indices().enumerate() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let end_byte = byte_idx + ch.len_utf8() - 1;
                        return Some(end_byte);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

pub struct AnalysisPrompt {
    pub system: String,
    pub user: String,
}

pub struct AnalysisContext {
    pub stock_code: String,
    pub stock_name: String,
    pub kline_summary: String,
    pub technical_summary: String,
    pub realtime_summary: String,
    pub market_context: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_content_plain_json() {
        let pipeline = make_test_pipeline();
        let content = r#"{"sentiment_score": 8, "decision_type": "buy"}"#;
        let result = pipeline.extract_json_from_content(content);
        assert!(result.starts_with('{'));
        assert!(result.contains("sentimentScore"));
    }

    #[test]
    fn test_extract_json_from_content_markdown_code_block() {
        let pipeline = make_test_pipeline();
        let content = "Here is the analysis:\n```json\n{\"sentimentScore\": 7}\n```\nDone.";
        let result = pipeline.extract_json_from_content(content);
        assert!(result.contains("sentimentScore"));
    }

    #[test]
    fn test_extract_json_from_content_backtick_block() {
        let pipeline = make_test_pipeline();
        let content = "```\n{\"score\": 5}\n```";
        let result = pipeline.extract_json_from_content(content);
        assert!(result.contains("score"));
    }

    #[test]
    fn test_extract_json_from_content_embedded_json() {
        let pipeline = make_test_pipeline();
        let content = "The result is: {\"key\": \"value\"} and that's it.";
        let result = pipeline.extract_json_from_content(content);
        assert!(result.starts_with('{'));
    }

    #[test]
    fn test_find_matching_brace() {
        assert_eq!(AnalysisPipeline::find_matching_brace("{}"), Some(1));
        assert_eq!(AnalysisPipeline::find_matching_brace("{}}"), Some(1));
        assert_eq!(AnalysisPipeline::find_matching_brace("{\"a\": 1}"), Some(7));
        assert_eq!(AnalysisPipeline::find_matching_brace("{no close"), None);
    }

    #[test]
    fn test_extract_content_valid_response() {
        let pipeline = make_test_pipeline();
        let response = tube::value!({
            "choices": [{"message": {"content": "hello"}}]
        });
        let content = pipeline.extract_content(&response).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn test_extract_content_missing_choices() {
        let pipeline = make_test_pipeline();
        let response = tube::value!({"error": "bad"});
        let result = pipeline.extract_content(&response);
        assert!(result.is_err());
    }

    fn make_test_pipeline() -> AnalysisPipeline {
        AnalysisPipeline {
            llm: Arc::new(ai_llm_kit::LlmFactory::create(
                ai_llm_kit::LlmProvider::ChatGPT,
                "test-key",
            )),
            model: "test".to_string(),
            temperature: 0.7,
            timeout_secs: 60,
            technical_analyzer: crate::technical::TechnicalAnalyzer::new(),
        }
    }
}
