use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "screening_results", comment = "筛选结果", primary = "identity")]
pub struct ScreeningResult {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, comment = "策略ID")]
    pub strategy: String,

    #[field(required = true, rename = "stock_code", comment = "股票代码")]
    pub stock_code: String,

    #[field(required = true, rename = "stock_name", comment = "股票名称")]
    pub stock_name: String,

    #[field(comment = "收盘价")]
    pub close: f64,

    #[field(default_value = "0", comment = "MA均线值")]
    pub ma_value: f64,

    #[field(default_value = "0", comment = "DIF值")]
    pub dif: f64,

    #[field(default_value = "0", comment = "DEA值")]
    pub dea: f64,

    #[field(default_value = "0", comment = "MACD柱")]
    pub macd_hist: f64,

    #[field(default_value = "0", rename = "pct_chg", comment = "涨跌幅%")]
    pub pct_chg: f64,

    #[field(default_value = "0", rename = "turnover_rate", comment = "换手率")]
    pub turnover_rate: f64,

    #[field(default_value = "0", rename = "volume_ratio", comment = "量比")]
    pub volume_ratio: f64,

    #[field(default_value = "0", comment = "高于均线百分比")]
    pub above_ma_pct: f64,

    #[field(rename = "trade_date", comment = "交易日期")]
    pub trade_date: Option<chrono::NaiveDateTime>,

    #[field(comment = "筛选参数JSON")]
    pub params_json: String,

    #[field(rename = "batch_id", comment = "批次ID")]
    pub batch_id: String,

    #[field(default_value = "1", comment = "状态")]
    pub status: i8,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
