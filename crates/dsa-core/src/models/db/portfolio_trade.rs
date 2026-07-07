//! 组合交易记录表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "portfolio_trades", comment = "组合交易记录", primary = "identity")]
pub struct PortfolioTrade {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "accountId", comment = "账户ID")]
    pub account_id: i64,

    #[field(required = true, rename = "stockCode", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "stockName", comment = "股票名称")]
    pub stock_name: String,

    #[field(required = true, comment = "方向: buy/sell")]
    pub direction: String,

    #[field(required = true, comment = "价格")]
    pub price: f64,

    #[field(required = true, comment = "数量")]
    pub quantity: i64,

    #[field(rename = "tradeDate", comment = "交易日期")]
    pub trade_date: Option<chrono::NaiveDateTime>,

    #[field(rename = "commission", comment = "手续费")]
    pub commission: f64,

    #[field(rename = "tradeCurrency", default_value = "'CNY'")]
    pub trade_currency: String,

    #[field(rename = "dedupHash", comment = "去重哈希")]
    pub dedup_hash: String,

    #[field(comment = "备注")]
    pub remark: String,

    #[field(default_value = "1")]
    pub status: i8,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
