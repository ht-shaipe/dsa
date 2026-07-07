//! 形态工具 - K线形态识别

use tube::Value;

use super::registry::{ToolParameter, ToolRegistry};

pub struct PatternTools;

impl PatternTools {
    pub fn new() -> Self { Self }

    /// 分析K线形态
    pub fn analyze_patterns(kline_data: &[Value]) -> Value {
        if kline_data.len() < 3 {
            return value!({"patterns": [], "summary": "数据不足"});
        }
        let mut patterns = Vec::new();
        let len = kline_data.len();
        let last = &kline_data[len - 1];
        let prev = &kline_data[len - 2];

        let last_open = last.get("open").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let last_close = last.get("close").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let last_high = last.get("high").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let last_low = last.get("low").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let prev_open = prev.get("open").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let prev_close = prev.get("close").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let body = (last_close - last_open).abs();
        let upper_shadow = last_high - last_close.max(last_open);
        let lower_shadow = last_close.min(last_open) - last_low;

        // Doji
        if body < (last_high - last_low) * 0.1 && (last_high - last_low) > 0.0 {
            patterns.push(value!({"name": "十字星", "signal": "neutral", "confidence": 0.6}));
        }
        // Hammer
        if lower_shadow > body * 2.0 && upper_shadow < body * 0.5 && last_close > last_open {
            patterns.push(value!({"name": "锤子线", "signal": "bullish", "confidence": 0.65}));
        }
        // Shooting star
        if upper_shadow > body * 2.0 && lower_shadow < body * 0.5 && last_close < last_open {
            patterns.push(value!({"name": "射击之星", "signal": "bearish", "confidence": 0.65}));
        }
        // Engulfing bullish
        if last_close > last_open && prev_close < prev_open && last_close > prev_open && last_open < prev_close {
            patterns.push(value!({"name": "看涨吞没", "signal": "bullish", "confidence": 0.7}));
        }
        // Engulfing bearish
        if last_close < last_open && prev_close > prev_open && last_open > prev_close && last_close < prev_open {
            patterns.push(value!({"name": "看跌吞没", "signal": "bearish", "confidence": 0.7}));
        }

        let summary = if patterns.is_empty() { "无显著形态" } else { "检测到K线形态" };
        value!({"patterns": patterns, "summary": summary})
    }
}

/// 异步封装: 检测K线形态
async fn detect_patterns(params: &Value) -> dsa_core::DsaResult<Value> {
    let kline_data = params
        .get("kline_data")
        .and_then(|v| v.as_array())
        .unwrap_or_default();
    Ok(PatternTools::analyze_patterns(&kline_data))
}

/// 注册形态工具到 ToolRegistry
pub fn register(registry: &mut ToolRegistry) {
    registry.register(
        "detect_patterns",
        "Detect K-line patterns from candlestick data",
        vec![ToolParameter {
            name: "kline_data".into(),
            param_type: "array".into(),
            description: "Array of kline data points".into(),
            required: true,
            default_value: None,
        }],
        |params| Box::pin(async move { detect_patterns(&params).await }),
    );
}
