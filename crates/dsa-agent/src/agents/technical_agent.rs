//! 技术分析Agent - 分析MA趋势、MACD、RSI、成交量形态、筹码分布

use async_trait::async_trait;
use dsa_core::{DsaError, DsaResult};
use tube::Value;

use ai_llm_kit::LlmService;

use super::base_agent::BaseAgent;
use crate::tools::analysis_tools::AnalysisTools;

/// 技术分析Agent
pub struct TechnicalAgent {
    llm: Box<dyn LlmService>,
    model: String,
}

impl TechnicalAgent {
    pub fn new(llm: Box<dyn LlmService>, model: &str) -> Self {
        Self { llm, model: model.to_string() }
    }
}

#[async_trait(?Send)]
impl BaseAgent for TechnicalAgent {
    fn name(&self) -> &str { "technical" }
    fn role(&self) -> &str { "技术面分析专家" }

    async fn process(&self, input: &Value) -> DsaResult<Value> {
        // 从输入获取代码
        let code = input.get("code")
            .and_then(|c| c.as_str())
            .unwrap_or_default();

        if code.is_empty() {
            return Err(DsaError::Validation("技术分析需要股票代码".to_string()));
        }

        // 获取已有的K线数据(如果输入中有)
        let kline_data = input.get("klineData")
            .and_then(|k| Value::as_array(k))
            .unwrap_or_default();

        // 本地计算技术指标
        let trend_analysis = AnalysisTools::analyze_trend(&kline_data);
        let volume_analysis = AnalysisTools::analyze_volume(&kline_data);

        // 提取趋势和量能信号用于prompt
        let trend_str = trend_analysis.get("trend")
            .and_then(|t| t.as_str())
            .unwrap_or_default();
        let vol_signal = volume_analysis.get("signal")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let ma5 = trend_analysis.get("ma5")
            .and_then(|m| m.as_f64())
            .unwrap_or(0.0);
        let ma20 = trend_analysis.get("ma20")
            .and_then(|m| m.as_f64())
            .unwrap_or(0.0);

        // 构建LLM提示
        let system_prompt = "你是一位资深技术分析专家，擅长分析股票的MA趋势、MACD、RSI、成交量形态和筹码分布。请根据提供的技术指标数据进行专业分析，给出明确的趋势判断和操作建议。用中文回答。";

        let user_prompt = format!(
            "请对股票{}进行技术面分析：\n\
             趋势判断：{}\n\
             5日均线：{:.2}\n\
             20日均线：{:.2}\n\
             量能信号：{}\n\
             \n\
             请从以下维度分析：\n\
             1. MA趋势分析（5日/10日/20日/60日均线排列）\n\
             2. MACD指标判断（金叉/死叉/零轴位置）\n\
             3. RSI指标判断（超买/超卖/中性）\n\
             4. 成交量形态（放量/缩量/量价配合）\n\
             5. 筹码分布（集中度/获利比例）\n\
             \n\
             请以JSON格式返回分析结果，包含以下字段：\n\
             - trend: 趋势方向(up/down/sideways)\n\
             - trendStrength: 趋势强度(0-100)\n\
             - maSignal: 均线信号(bullish/bearish/neutral)\n\
             - macdSignal: MACD信号(golden_cross/death_cross/neutral)\n\
             - rsiSignal: RSI信号(overbought/oversold/neutral)\n\
             - volumeSignal: 量能信号\n\
             - chipSignal: 筹码信号\n\
             - technicalScore: 技术面综合评分(0-100)\n\
             - summary: 技术面总结(200字以内)",
            code, trend_str, ma5, ma20, vol_signal
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
            .map_err(|e| DsaError::LlmAnalysis(format!("技术分析Agent调用LLM失败: {}", e)))?;

        let content = response.get("choices")
            .and_then(|c| Value::as_array(c))
            .and_then(|a| a.first().cloned())
            .and_then(|f| f.get("message").cloned())
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .unwrap_or_default();

        Ok(value!({
            "agent": self.name(),
            "code": code,
            "localAnalysis": {
                "trend": trend_analysis,
                "volume": volume_analysis,
            },
            "llmAnalysis": content,
        }))
    }
}
