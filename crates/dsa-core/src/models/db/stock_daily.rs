//! 日K线数据表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "stock_daily", comment = "日K线数据", primary = "identity")]
pub struct StockDaily {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "stock_code", comment = "股票代码")]
    pub stock_code: String,

    #[field(required = true, rename = "stock_name", comment = "股票名称")]
    pub stock_name: String,

    #[field(required = true, rename = "trade_date", comment = "交易日期")]
    pub trade_date: Option<chrono::NaiveDateTime>,

    #[field(comment = "开盘价")]
    pub open: f64,

    #[field(comment = "收盘价")]
    pub close: f64,

    #[field(comment = "最高价")]
    pub high: f64,

    #[field(comment = "最低价")]
    pub low: f64,

    #[field(comment = "成交量")]
    pub volume: i64,

    #[field(comment = "成交额")]
    pub amount: f64,

    #[field(rename = "pct_chg", comment = "涨跌幅%")]
    pub pct_chg: f64,

    #[field(rename = "ma5", comment = "5日均线")]
    pub ma5: f64,

    #[field(rename = "ma10", comment = "10日均线")]
    pub ma10: f64,

    #[field(rename = "ma20", comment = "20日均线")]
    pub ma20: f64,

    #[field(rename = "volume_ratio", comment = "量比")]
    pub volume_ratio: f64,

    #[field(rename = "turnover_rate", comment = "换手率")]
    pub turnover_rate: f64,

    #[field(default_value = "1", comment = "状态")]
    pub status: i8,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
