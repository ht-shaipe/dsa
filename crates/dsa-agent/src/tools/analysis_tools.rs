//! 分析工具 - 技术指标计算

use tube::Value;

pub struct AnalysisTools;

impl AnalysisTools {
    pub fn new() -> Self { Self }

    /// 简单移动平均线
    /// closes: 收盘价序列
    /// period: 周期
    pub fn calculate_ma(closes: &[f64], period: usize) -> f64 {
        if closes.len() < period || period == 0 {
            return 0.0;
        }
        let sum: f64 = closes[closes.len() - period..].iter().sum();
        sum / period as f64
    }

    /// 趋势分析
    /// kline: K线数据数组，每条含 "收盘" 字段
    pub fn analyze_trend(kline: &[Value]) -> Value {
        if kline.len() < 5 {
            return value!({"trend": "insufficient_data", "direction": "unknown", "strength": 0.0});
        }

        // 提取收盘价
        let closes: Vec<f64> = kline.iter()
            .filter_map(|v| v.get("收盘").and_then(|c| c.as_f64()))
            .collect();

        if closes.len() < 5 {
            return value!({"trend": "insufficient_data", "direction": "unknown", "strength": 0.0});
        }

        // 计算短期(5日)和长期(20日)均线
        let ma5 = Self::calculate_ma(&closes, 5.min(closes.len()));
        let ma20 = if closes.len() >= 20 {
            Self::calculate_ma(&closes, 20)
        } else {
            Self::calculate_ma(&closes, closes.len())
        };

        // 计算涨跌次数
        let mut up_count = 0u32;
        let mut down_count = 0u32;
        for i in 1..closes.len() {
            if closes[i] > closes[i - 1] {
                up_count += 1;
            } else if closes[i] < closes[i - 1] {
                down_count += 1;
            }
        }

        let (direction, strength) = if ma5 > ma20 {
            let ratio = (ma5 - ma20) / ma20 * 100.0;
            ("up", ratio.min(100.0))
        } else if ma5 < ma20 {
            let ratio = (ma20 - ma5) / ma20 * 100.0;
            ("down", ratio.min(100.0))
        } else {
            ("sideways", 0.0)
        };

        let trend = if up_count > down_count * 2 {
            "strong_up"
        } else if up_count > down_count {
            "up"
        } else if down_count > up_count * 2 {
            "strong_down"
        } else if down_count > up_count {
            "down"
        } else {
            "sideways"
        };

        value!({
            "trend": trend,
            "direction": direction,
            "strength": strength,
            "ma5": ma5,
            "ma20": ma20,
            "upDays": up_count,
            "downDays": down_count,
        })
    }

    /// 成交量分析
    pub fn analyze_volume(kline: &[Value]) -> Value {
        if kline.len() < 5 {
            return value!({"status": "insufficient_data", "signal": "unknown"});
        }

        // 提取成交量
        let volumes: Vec<f64> = kline.iter()
            .filter_map(|v| v.get("成交量").and_then(|c| c.as_f64()))
            .collect();

        if volumes.len() < 5 {
            return value!({"status": "insufficient_data", "signal": "unknown"});
        }

        // 计算平均成交量(最近5日)
        let avg_vol_5: f64 = volumes[volumes.len() - 5..].iter().sum::<f64>() / 5.0;
        // 最近一天成交量
        let latest_vol = volumes.last().unwrap_or(&0.0);

        // 量比
        let volume_ratio = if avg_vol_5 > 0.0 {
            latest_vol / avg_vol_5
        } else {
            1.0
        };

        // 判断放量/缩量
        let signal = if volume_ratio > 2.0 {
            "huge_volume"
        } else if volume_ratio > 1.5 {
            "volume_increase"
        } else if volume_ratio < 0.5 {
            "shrink_volume"
        } else if volume_ratio < 0.8 {
            "slight_shrink"
        } else {
            "normal"
        };

        // 计算成交量趋势(最近5天)
        let mut vol_increasing = 0u32;
        let vol_slice = &volumes[volumes.len().saturating_sub(5)..];
        for i in 1..vol_slice.len() {
            if vol_slice[i] > vol_slice[i - 1] {
                vol_increasing += 1;
            }
        }

        value!({
            "latestVolume": latest_vol,
            "avgVolume5": avg_vol_5,
            "volume_ratio": volume_ratio,
            "signal": signal,
            "volTrend": if vol_increasing >= 3 { "increasing" } else if vol_increasing <= 1 { "decreasing" } else { "stable" },
        })
    }
}
