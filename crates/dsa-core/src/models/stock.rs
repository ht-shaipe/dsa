//! 股票相关数据结构

use serde::{Deserialize, Serialize};

/// 市场类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Market {
    /// 中国A股
    #[serde(rename = "cn")]
    CN,
    /// 港股
    #[serde(rename = "hk")]
    HK,
    /// 美股
    #[serde(rename = "us")]
    US,
    /// 日股
    #[serde(rename = "jp")]
    JP,
    /// 韩股
    #[serde(rename = "kr")]
    KR,
    /// 台股
    #[serde(rename = "tw")]
    TW,
}

impl Market {
    /// 根据股票代码推断所属市场
    pub fn from_code(code: &str) -> Self {
        let code = code.to_uppercase();
        if code.starts_with("HK") {
            Market::HK
        } else if code.contains('.') {
            let suffix = code.split('.').last().unwrap_or("");
            match suffix {
                "HK" => Market::HK,
                "US" | "N" | "O" | "P" | "Q" => Market::US,
                "T" => Market::JP,
                "KS" => Market::KR,
                "TW" => Market::TW,
                _ => Market::CN,
            }
        } else if code.len() == 6 {
            match &code[..3] {
                "000" | "001" | "002" | "003" => Market::CN,
                "300" => Market::CN,
                "600" | "601" | "603" => Market::CN,
                "688" => Market::CN,
                _ => Market::CN,
            }
        } else {
            Market::US
        }
    }

    /// 返回市场的中文标签
    pub fn label(&self) -> &str {
        match self {
            Market::CN => "A股",
            Market::HK => "港股",
            Market::US => "美股",
            Market::JP => "日股",
            Market::KR => "韩股",
            Market::TW => "台股",
        }
    }
}

/// 股票基本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfo {
    pub code: String,
    pub name: String,
    pub market: Market,
}

impl StockInfo {
    /// 根据代码和名称创建股票信息
    pub fn new(code: &str, name: &str) -> Self {
        Self {
            code: code.to_string(),
            name: name.to_string(),
            market: Market::from_code(code),
        }
    }
}

/// 实时行情报价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeQuote {
    pub code: String,
    pub name: String,
    pub price: f64,
    pub change_pct: f64,
    pub change_amt: f64,
    pub volume: i64,
    pub amount: f64,
    pub turnover_rate: f64,
    pub volume_ratio: f64,
    pub pe: f64,
    pub pb: f64,
    pub high: f64,
    pub low: f64,
    pub open: f64,
    pub prev_close: f64,
    pub timestamp: String,
}

/// K线数据条
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineBar {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub amount: f64,
}

/// 技术指标集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    pub ma5: f64,
    pub ma10: f64,
    pub ma20: f64,
    pub ma60: f64,
    pub macd: f64,
    pub macd_signal: f64,
    pub macd_hist: f64,
    pub rsi_14: f64,
    pub bias_ma5: f64,
    pub bias_ma10: f64,
    pub is_bullish_alignment: bool,
    pub trend_score: i32,
}

/// 筹码分布结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipDistribution {
    pub profit_ratio: f64,
    pub avg_cost: f64,
    pub concentration: f64,
    pub chip_health: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_from_code_cn_sh() {
        assert_eq!(Market::from_code("600519"), Market::CN);
    }

    #[test]
    fn test_market_from_code_cn_sz() {
        assert_eq!(Market::from_code("000001"), Market::CN);
        assert_eq!(Market::from_code("300750"), Market::CN);
    }

    #[test]
    fn test_market_from_code_hk() {
        assert_eq!(Market::from_code("HK00700"), Market::HK);
        assert_eq!(Market::from_code("00700.HK"), Market::HK);
    }

    #[test]
    fn test_market_from_code_us() {
        assert_eq!(Market::from_code("AAPL.US"), Market::US);
        assert_eq!(Market::from_code("AAPL"), Market::US);
    }

    #[test]
    fn test_market_label() {
        assert_eq!(Market::CN.label(), "A股");
        assert_eq!(Market::HK.label(), "港股");
        assert_eq!(Market::US.label(), "美股");
    }

    #[test]
    fn test_kline_bar_serialization() {
        let bar = KlineBar {
            date: "2024-01-15".to_string(),
            open: 10.5,
            high: 11.0,
            low: 10.2,
            close: 10.8,
            volume: 100000,
            amount: 1080000.0,
        };
        let json = serde_json::to_string(&bar).unwrap();
        let parsed: KlineBar = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.date, "2024-01-15");
        assert_eq!(parsed.close, 10.8);
        assert_eq!(parsed.volume, 100000);
    }

    #[test]
    fn test_technical_indicators_default_fields() {
        let ti = TechnicalIndicators {
            ma5: 10.0,
            ma10: 9.5,
            ma20: 9.0,
            ma60: 8.5,
            macd: 0.1,
            macd_signal: 0.05,
            macd_hist: 0.1,
            rsi_14: 55.0,
            bias_ma5: 2.0,
            bias_ma10: 3.0,
            is_bullish_alignment: true,
            trend_score: 75,
        };
        assert!(ti.is_bullish_alignment);
        assert_eq!(ti.trend_score, 75);
    }
}
