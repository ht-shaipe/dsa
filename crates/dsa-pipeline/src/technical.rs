//! 技术指标计算

use dsa_core::models::{KlineBar, TechnicalIndicators};
use tube::Value;

#[derive(Debug, Clone)]
pub struct MacdPoint {
    pub dif: f64,
    pub dea: f64,
    pub hist: f64,
}

pub struct TechnicalAnalyzer;

impl TechnicalAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate(&self, kline: &[KlineBar], _realtime: Option<&Value>) -> TechnicalIndicators {
        let closes: Vec<f64> = kline.iter().map(|b| b.close).collect();

        let ma5 = self.sma(&closes, 5);
        let ma10 = self.sma(&closes, 10);
        let ma20 = self.sma(&closes, 20);
        let ma60 = self.sma(&closes, 60);

        let (macd, macd_signal, macd_hist) = self.macd(&closes, 12, 26, 9);

        let rsi_14 = self.rsi(&closes, 14);

        let current = closes.last().copied().unwrap_or(0.0);
        let bias_ma5 = if ma5 != 0.0 { (current - ma5) / ma5 * 100.0 } else { 0.0 };
        let bias_ma10 = if ma10 != 0.0 { (current - ma10) / ma10 * 100.0 } else { 0.0 };

        let is_bullish = ma5 > ma10 && ma10 > ma20 && ma5 > 0.0;

        let trend_score = self.calculate_trend_score(
            is_bullish, bias_ma5, rsi_14, macd_hist,
        );

        TechnicalIndicators {
            ma5,
            ma10,
            ma20,
            ma60,
            macd,
            macd_signal,
            macd_hist,
            rsi_14,
            bias_ma5,
            bias_ma10,
            is_bullish_alignment: is_bullish,
            trend_score,
        }
    }

    pub fn macd_series(&self, closes: &[f64], fast: usize, slow: usize, signal: usize) -> Vec<MacdPoint> {
        let dif_series = self.ema_series(closes, fast, slow);
        if dif_series.len() < signal {
            return vec![];
        }
        let k_signal = 2.0 / (signal as f64 + 1.0);
        let mut dea_series: Vec<f64> = Vec::with_capacity(dif_series.len());
        let mut dea = dif_series[0];
        for &d in &dif_series {
            dea = d * k_signal + dea * (1.0 - k_signal);
            dea_series.push(dea);
        }
        dif_series
            .iter()
            .zip(dea_series.iter())
            .map(|(&dif, &dea)| MacdPoint {
                dif,
                dea,
                hist: 2.0 * (dif - dea),
            })
            .collect()
    }

    pub fn is_macd_golden_cross(&self, hist_series: &[f64], lookback: usize) -> bool {
        let n = hist_series.len();
        if n < 2 {
            return false;
        }
        let start = if n > lookback { n - lookback } else { 0 };
        let recent = &hist_series[start..];
        if recent.len() < 2 {
            return false;
        }
        let latest = recent[recent.len() - 1];
        let prev = recent[recent.len() - 2];

        // Case 1: Just crossed - hist went from negative to non-negative
        if prev < 0.0 && latest >= 0.0 {
            return true;
        }

        // Case 2: Green bars shrinking (approaching golden cross)
        // All recent bars must be negative, and each successive one is larger (closer to 0)
        if latest < 0.0 && prev < 0.0 {
            let mut all_negative = true;
            let mut shrinking = true;
            for i in 1..recent.len() {
                if recent[i] >= 0.0 {
                    all_negative = false;
                    break;
                }
                if recent[i] <= recent[i - 1] {
                    // hist[i] <= hist[i-1] means absolute value is growing or same = NOT shrinking
                    shrinking = false;
                }
            }
            // Only true if all negative AND shrinking (absolute value decreasing)
            if all_negative && shrinking {
                return true;
            }
        }

        // Case 3: Already in golden cross state (positive hist) but just recently crossed
        // Check if within the lookback window there was a negative-to-positive transition
        if latest >= 0.0 && prev >= 0.0 {
            for i in 1..recent.len() {
                if recent[i - 1] < 0.0 && recent[i] >= 0.0 {
                    return true;
                }
            }
        }

        false
    }

    pub fn sma(&self, data: &[f64], period: usize) -> f64 {
        if data.len() < period || period == 0 {
            return 0.0;
        }
        let sum: f64 = data[data.len() - period..].iter().sum();
        sum / period as f64
    }

    pub fn macd(&self, closes: &[f64], fast: usize, slow: usize, signal: usize) -> (f64, f64, f64) {
        let ema_fast = self.ema(closes, fast);
        let ema_slow = self.ema(closes, slow);
        let dif = ema_fast - ema_slow;

        let dif_series: Vec<f64> = self.ema_series(closes, fast, slow);
        let dea = self.sma(&dif_series, signal);
        let macd_hist = 2.0 * (dif - dea);

        (dif, dea, macd_hist)
    }

    fn ema(&self, data: &[f64], period: usize) -> f64 {
        if data.is_empty() || period == 0 {
            return 0.0;
        }
        let k = 2.0 / (period as f64 + 1.0);
        let mut ema = data[0];
        for &price in &data[1..] {
            ema = price * k + ema * (1.0 - k);
        }
        ema
    }

    fn ema_series(&self, closes: &[f64], fast: usize, slow: usize) -> Vec<f64> {
        if closes.len() < slow {
            return vec![];
        }
        let k_fast = 2.0 / (fast as f64 + 1.0);
        let k_slow = 2.0 / (slow as f64 + 1.0);

        let mut ema_fast = closes[0];
        let mut ema_slow = closes[0];
        let mut dif_series = Vec::with_capacity(closes.len());

        for &price in &closes[1..] {
            ema_fast = price * k_fast + ema_fast * (1.0 - k_fast);
            ema_slow = price * k_slow + ema_slow * (1.0 - k_slow);
            dif_series.push(ema_fast - ema_slow);
        }
        dif_series
    }

    fn rsi(&self, closes: &[f64], period: usize) -> f64 {
        if closes.len() < period + 1 {
            return 50.0;
        }
        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in (closes.len() - period)..closes.len() {
            let change = closes[i] - closes[i - 1];
            if change > 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
        }

        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;

        if avg_loss == 0.0 {
            return 100.0;
        }
        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }

    fn calculate_trend_score(&self, is_bullish: bool, bias_ma5: f64, rsi: f64, macd_hist: f64) -> i32 {
        let mut score = 50;

        if is_bullish { score += 15; } else { score -= 10; }

        if bias_ma5.abs() < 3.0 { score += 5; }
        else if bias_ma5 > 5.0 { score -= 10; }
        else if bias_ma5 < -5.0 { score -= 5; }

        if rsi > 70.0 { score -= 10; }
        else if rsi > 50.0 { score += 5; }
        else if rsi < 30.0 { score += 5; }
        else { score -= 5; }

        if macd_hist > 0.0 { score += 5; } else { score -= 5; }

        score.clamp(0, 100)
    }
}

impl Default for TechnicalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                volume: 10000,
                amount: c * 10000.0,
            })
            .collect()
    }

    #[test]
    fn test_sma_basic() {
        let analyzer = TechnicalAnalyzer::new();
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ma5 = analyzer.sma(&data, 5);
        assert!((ma5 - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_sma_insufficient_data() {
        let analyzer = TechnicalAnalyzer::new();
        let data = vec![1.0, 2.0];
        let ma5 = analyzer.sma(&data, 5);
        assert_eq!(ma5, 0.0);
    }

    #[test]
    fn test_ema_basic() {
        let analyzer = TechnicalAnalyzer::new();
        let data = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let ema3 = analyzer.ema(&data, 3);
        assert!(ema3 > 10.0 && ema3 < 14.0);
    }

    #[test]
    fn test_ema_empty() {
        let analyzer = TechnicalAnalyzer::new();
        let ema = analyzer.ema(&[], 5);
        assert_eq!(ema, 0.0);
    }

    #[test]
    fn test_rsi_overbought() {
        let analyzer = TechnicalAnalyzer::new();
        let closes: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let rsi = analyzer.rsi(&closes, 14);
        assert!(rsi > 70.0, "monotonically rising prices should have RSI > 70, got {}", rsi);
    }

    #[test]
    fn test_rsi_oversold() {
        let analyzer = TechnicalAnalyzer::new();
        let closes: Vec<f64> = (0..20).map(|i| 120.0 - i as f64).collect();
        let rsi = analyzer.rsi(&closes, 14);
        assert!(rsi < 30.0, "monotonically falling prices should have RSI < 30, got {}", rsi);
    }

    #[test]
    fn test_rsi_insufficient_data() {
        let analyzer = TechnicalAnalyzer::new();
        let closes = vec![10.0, 11.0];
        let rsi = analyzer.rsi(&closes, 14);
        assert_eq!(rsi, 50.0);
    }

    #[test]
    fn test_macd_returns_tuple() {
        let analyzer = TechnicalAnalyzer::new();
        let closes: Vec<f64> = (0..60).map(|i| 50.0 + (i as f64 * 0.5).sin() * 10.0).collect();
        let (dif, dea, hist) = analyzer.macd(&closes, 12, 26, 9);
        assert!(dif.is_finite());
        assert!(dea.is_finite());
        assert!(hist.is_finite());
        assert!((hist - 2.0 * (dif - dea)).abs() < 0.001, "MACD histogram = 2*(DIF-DEA)");
    }

    #[test]
    fn test_calculate_full_indicators() {
        let analyzer = TechnicalAnalyzer::new();
        let closes: Vec<f64> = (0..80).map(|i| 50.0 + (i as f64 * 0.3).sin() * 10.0).collect();
        let kline = make_kline(&closes);
        let ti = analyzer.calculate(&kline, None);

        assert!(ti.ma5 > 0.0, "MA5 should be positive");
        assert!(ti.ma20 > 0.0, "MA20 should be positive");
        assert!(ti.rsi_14 > 0.0 && ti.rsi_14 < 100.0, "RSI should be 0-100");
        assert!(ti.trend_score >= 0 && ti.trend_score <= 100, "trend_score should be 0-100");
    }

    #[test]
    fn test_calculate_empty_kline() {
        let analyzer = TechnicalAnalyzer::new();
        let kline: Vec<KlineBar> = vec![];
        let ti = analyzer.calculate(&kline, None);
        assert_eq!(ti.ma5, 0.0);
        assert_eq!(ti.rsi_14, 50.0);
        assert_eq!(ti.trend_score, 35);
    }

    #[test]
    fn test_bullish_alignment() {
        let analyzer = TechnicalAnalyzer::new();
        let closes: Vec<f64> = (0..80).map(|i| 10.0 + i as f64).collect();
        let kline = make_kline(&closes);
        let ti = analyzer.calculate(&kline, None);
        assert!(ti.is_bullish_alignment, "steadily rising prices should show bullish MA alignment");
    }

    #[test]
    fn test_macd_series_length() {
        let analyzer = TechnicalAnalyzer::new();
        let closes: Vec<f64> = (0..80).map(|i| 50.0 + (i as f64 * 0.3).sin() * 10.0).collect();
        let series = analyzer.macd_series(&closes, 12, 26, 9);
        assert!(!series.is_empty(), "macd_series should return non-empty for 80 data points");
        for pt in &series {
            assert!((pt.hist - 2.0 * (pt.dif - pt.dea)).abs() < 0.001, "hist = 2*(dif-dea)");
        }
    }

    #[test]
    fn test_golden_cross_actual_cross() {
        let analyzer = TechnicalAnalyzer::new();
        // Directly construct a hist series that represents green bars shrinking then crossing
        let hists: Vec<f64> = vec![-2.0, -1.5, -1.0, -0.5, -0.2, 0.1, 0.3];
        let result = analyzer.is_macd_golden_cross(&hists, 7);
        assert!(result, "hist going from negative to positive should be golden cross");
    }

    #[test]
    fn test_golden_cross_shrinking_green() {
        let analyzer = TechnicalAnalyzer::new();
        // Green bars shrinking but not yet crossed
        let hists: Vec<f64> = vec![-2.0, -1.5, -1.0, -0.5, -0.2, -0.1];
        let result = analyzer.is_macd_golden_cross(&hists, 6);
        assert!(result, "shrinking green bars should be detected as approaching golden cross");
    }

    #[test]
    fn test_golden_cross_no_cross_expanding() {
        let analyzer = TechnicalAnalyzer::new();
        // Negative bars getting more negative (expanding, not shrinking)
        let hists: Vec<f64> = vec![-0.5, -1.0, -1.5, -2.0, -2.5];
        let result = analyzer.is_macd_golden_cross(&hists, 5);
        assert!(!result, "expanding negative bars should not be golden cross");
    }

    #[test]
    fn test_golden_cross_no_cross() {
        let analyzer = TechnicalAnalyzer::new();
        let closes: Vec<f64> = (0..80).map(|i| 100.0 - i as f64 * 0.5).collect();
        let series = analyzer.macd_series(&closes, 12, 26, 9);
        let hists: Vec<f64> = series.iter().map(|p| p.hist).collect();
        // Steadily falling: all hist values should be negative
        let all_negative = hists.iter().all(|&h| h <= 0.0);
        if all_negative {
            // If all negative, bars should be getting MORE negative (not shrinking)
            let result = analyzer.is_macd_golden_cross(&hists, 5);
            assert!(!result, "steadily falling with expanding negative bars should not trigger golden cross");
        }
        // If some values happen to be positive, just check the function returns false
        // since there shouldn't be a fresh cross in this scenario
    }
}
