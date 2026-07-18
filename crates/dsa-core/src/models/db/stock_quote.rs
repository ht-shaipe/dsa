use deck::Model;
use serde::{Deserialize, Serialize};

/// 每日行情快照，每个交易日一条
#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(
    name = "stock_quote",
    comment = "每日行情快照",
    primary = "identity"
)]
pub struct StockQuote {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "stock_code", length = 32, comment = "股票代码")]
    pub stock_code: String,

    #[field(required = true, rename = "trade_date", length = 16, comment = "交易日期 YYYY-MM-DD")]
    pub trade_date: String,

    #[field(rename = "open", comment = "开盘价")]
    pub open: Option<f64>,

    #[field(rename = "high", comment = "最高价")]
    pub high: Option<f64>,

    #[field(rename = "low", comment = "最低价")]
    pub low: Option<f64>,

    #[field(rename = "close", comment = "当前价格")]
    pub close: Option<f64>,

    #[field(rename = "previous_close", comment = "昨日收盘价")]
    pub previous_close: Option<f64>,

    #[field(rename = "change_price", comment = "涨跌额")]
    pub change_price: Option<f64>,

    #[field(rename = "change_percent", comment = "涨跌幅(%)")]
    pub change_percent: Option<f64>,

    #[field(rename = "volume", comment = "成交量")]
    pub volume: Option<f64>,

    #[field(rename = "amount", comment = "成交额")]
    pub amount: Option<f64>,

    #[field(rename = "amplitude", comment = "振幅(%)")]
    pub amplitude: Option<f64>,

    #[field(rename = "turnover_ratio", comment = "换手率(%)")]
    pub turnover_ratio: Option<f64>,

    #[field(rename = "volume_ratio", comment = "量比")]
    pub volume_ratio: Option<f64>,

    #[field(rename = "total_market_cap", comment = "总市值(亿)")]
    pub total_market_cap: Option<f64>,

    #[field(rename = "liquid_market_cap", comment = "流通市值(亿)")]
    pub liquid_market_cap: Option<f64>,

    #[field(rename = "pe", comment = "市盈率")]
    pub pe: Option<f64>,

    #[field(rename = "pb", comment = "市净率")]
    pub pb: Option<f64>,

    #[field(rename = "speed_up", comment = "涨速(%)")]
    pub speed_up: Option<f64>,

    #[field(rename = "change_percent_60", comment = "60日涨跌幅(%)")]
    pub change_percent_60: Option<f64>,

    #[field(rename = "year_change_percent", comment = "年初至今涨跌幅(%)")]
    pub year_change_percent: Option<f64>,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
