//! 分析上下文构建器 - 对齐原项目 analysis_context_builder.py

use crate::pipeline::AnalysisContext;
use dsa_core::models::{KlineBar, TechnicalIndicators};
use tube::Value;

pub struct AnalysisContextBuilder;

impl AnalysisContextBuilder {
    pub fn build(
        code: &str,
        name: &str,
        kline: &[KlineBar],
        realtime: Option<&Value>,
        technical: &TechnicalIndicators,
        market_context: Option<&str>,
    ) -> AnalysisContext {
        AnalysisContext {
            stock_code: code.to_string(),
            stock_name: name.to_string(),
            kline_summary: Self::summarize_kline(kline),
            technical_summary: Self::summarize_technical(technical, realtime),
            realtime_summary: Self::summarize_realtime(realtime),
            market_context: market_context.unwrap_or("暂无大盘上下文").to_string(),
        }
    }

    fn summarize_kline(kline: &[KlineBar]) -> String {
        if kline.is_empty() {
            return "无K线数据".to_string();
        }

        let last = &kline[kline.len() - 1];
        let len = kline.len();

        let mut recent_changes = String::new();
        let window = kline.len().min(5);
        let start = kline.len() - window;
        for i in start..kline.len() {
            let bar = &kline[i];
            let prev_close = if i > 0 { kline[i - 1].close } else { bar.open };
            let change = if prev_close != 0.0 {
                (bar.close - prev_close) / prev_close * 100.0
            } else {
                0.0
            };
            recent_changes.push_str(&format!(
                "{}: 收{:.2} 涨跌{:.2}% 量{}; ",
                bar.date, bar.close, change, bar.volume
            ));
        }

        format!(
            "最近{}日K线: {} | 最新日期: {} 收盘:{:.2} 最高:{:.2} 最低:{:.2}",
            len, recent_changes, last.date, last.close, last.high, last.low
        )
    }

    fn summarize_technical(tech: &TechnicalIndicators, realtime: Option<&Value>) -> String {
        let current_price = realtime
            .and_then(|v| v.get("price"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        format!(
            "当前价: {:.2} | MA5={:.2} MA10={:.2} MA20={:.2} MA60={:.2} | \
             乖离率MA5={:.2}% 乖离率MA10={:.2}% | \
             MACD: DIF={:.4} DEA={:.4} 柱={:.4} | \
             RSI14={:.2} | \
             多头排列={} | 趋势评分={}",
            current_price,
            tech.ma5,
            tech.ma10,
            tech.ma20,
            tech.ma60,
            tech.bias_ma5,
            tech.bias_ma10,
            tech.macd,
            tech.macd_signal,
            tech.macd_hist,
            tech.rsi_14,
            if tech.is_bullish_alignment {
                "是"
            } else {
                "否"
            },
            tech.trend_score,
        )
    }

    fn summarize_realtime(realtime: Option<&Value>) -> String {
        match realtime {
            Some(v) => {
                let name = v.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                let code = v.get("code").and_then(|v| v.as_str()).unwrap_or_default();
                let price = v.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let change_pct = v.get("change_pct").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let volume_ratio = v
                    .get("volume_ratio")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let turnover_rate = v
                    .get("turnover_rate")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let pe = v.get("pe").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let pb = v.get("pb").and_then(|v| v.as_f64()).unwrap_or(0.0);
                format!(
                    "实时行情: {}({}) 现价:{:.2} 涨跌:{:.2}% 量比:{:.2} 换手:{:.2}% PE:{:.2} PB:{:.2}",
                    name, code, price, change_pct, volume_ratio, turnover_rate, pe, pb
                )
            }
            None => "无实时行情数据".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::models::TechnicalIndicators;

    fn make_kline(closes: &[f64]) -> Vec<KlineBar> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| KlineBar {
                date: format!("2024-01-{:02}", i + 1),
                open: c - 0.1,
                high: c + 0.5,
                low: c - 0.5,
                close: c,
                volume: 10000 + i as i64,
                amount: c * 10000.0,
            })
            .collect()
    }

    #[test]
    fn test_summarize_kline_empty() {
        let kline: Vec<KlineBar> = vec![];
        let result = AnalysisContextBuilder::build(
            "000001",
            "test",
            &kline,
            None,
            &TechnicalIndicators {
                ma5: 0.0,
                ma10: 0.0,
                ma20: 0.0,
                ma60: 0.0,
                macd: 0.0,
                macd_signal: 0.0,
                macd_hist: 0.0,
                rsi_14: 50.0,
                bias_ma5: 0.0,
                bias_ma10: 0.0,
                is_bullish_alignment: false,
                trend_score: 50,
            },
            None,
        );
        assert_eq!(result.kline_summary, "无K线数据");
    }

    #[test]
    fn test_summarize_kline_with_data() {
        let closes: Vec<f64> = (0..10).map(|i| 10.0 + i as f64).collect();
        let kline = make_kline(&closes);
        let result = AnalysisContextBuilder::build(
            "600519",
            "茅台",
            &kline,
            None,
            &TechnicalIndicators {
                ma5: 14.0,
                ma10: 13.0,
                ma20: 12.0,
                ma60: 0.0,
                macd: 0.1,
                macd_signal: 0.05,
                macd_hist: 0.1,
                rsi_14: 60.0,
                bias_ma5: 2.0,
                bias_ma10: 3.0,
                is_bullish_alignment: true,
                trend_score: 75,
            },
            Some("大盘上涨"),
        );
        assert!(result.kline_summary.contains("最近10日K线"));
        assert!(result.kline_summary.contains("19.00"));
        assert!(result.market_context.contains("大盘上涨"));
    }

    #[test]
    fn test_build_with_realtime() {
        let kline = make_kline(&[10.0, 11.0, 12.0]);
        let realtime = tube::value!({
            "name": "茅台",
            "code": "600519",
            "price": 1800.0,
            "change_pct": 2.5,
            "volume_ratio": 1.2,
            "turnover_rate": 0.8,
            "pe": 35.0,
            "pb": 12.0,
        });
        let result = AnalysisContextBuilder::build(
            "600519",
            "茅台",
            &kline,
            Some(&realtime),
            &TechnicalIndicators {
                ma5: 11.0,
                ma10: 10.5,
                ma20: 10.0,
                ma60: 0.0,
                macd: 0.0,
                macd_signal: 0.0,
                macd_hist: 0.0,
                rsi_14: 55.0,
                bias_ma5: 1.0,
                bias_ma10: 2.0,
                is_bullish_alignment: false,
                trend_score: 60,
            },
            None,
        );
        assert!(result.realtime_summary.contains("茅台"));
        assert!(result.realtime_summary.contains("1800.00"));
        assert!(result.technical_summary.contains("1800.00"));
    }
}
