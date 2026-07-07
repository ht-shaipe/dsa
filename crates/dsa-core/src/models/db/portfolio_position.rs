//! 组合持仓表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "portfolio_positions", comment = "组合持仓", primary = "identity")]
pub struct PortfolioPosition {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "accountId", comment = "账户ID")]
    pub account_id: i64,

    #[field(required = true, rename = "stockCode", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "stockName", comment = "股票名称")]
    pub stock_name: String,

    #[field(comment = "持仓数量")]
    pub quantity: i64,

    #[field(rename = "avgCost", comment = "平均成本")]
    pub avg_cost: f64,

    #[field(rename = "currentPrice", comment = "当前价")]
    pub current_price: f64,

    #[field(rename = "marketValue", comment = "市值")]
    pub market_value: f64,

    #[field(rename = "unrealizedPnl", comment = "未实现盈亏")]
    pub unrealized_pnl: f64,

    #[field(rename = "unrealizedPnlPct", comment = "未实现盈亏%")]
    pub unrealized_pnl_pct: f64,

    #[field(rename = "snapshotDate", comment = "快照日期")]
    pub snapshot_date: Option<chrono::NaiveDateTime>,

    #[field(default_value = "1")]
    pub status: i8,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modifyTime", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
