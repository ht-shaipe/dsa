//! 组合持仓表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(
    name = "portfolio_positions",
    comment = "组合持仓",
    primary = "identity"
)]
pub struct PortfolioPosition {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "account_id", comment = "账户ID")]
    pub account_id: i64,

    #[field(required = true, rename = "stock_code", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "stock_name", comment = "股票名称")]
    pub stock_name: String,

    #[field(comment = "持仓数量")]
    pub quantity: i64,

    #[field(rename = "avg_cost", comment = "平均成本")]
    pub avg_cost: f64,

    #[field(rename = "current_price", comment = "当前价")]
    pub current_price: f64,

    #[field(rename = "market_value", comment = "市值")]
    pub market_value: f64,

    #[field(rename = "unrealized_pnl", comment = "未实现盈亏")]
    pub unrealized_pnl: f64,

    #[field(rename = "unrealized_pnl_pct", comment = "未实现盈亏%")]
    pub unrealized_pnl_pct: f64,

    #[field(rename = "realized_pnl", default_value = "0", comment = "已实现盈亏")]
    pub realized_pnl: f64,

    #[field(rename = "total_commission", default_value = "0", comment = "累计佣金")]
    pub total_commission: f64,

    #[field(rename = "total_buy_amount", default_value = "0", comment = "累计买入金额(含佣金)")]
    pub total_buy_amount: f64,

    #[field(rename = "total_sell_amount", default_value = "0", comment = "累计卖出金额(扣佣金)")]
    pub total_sell_amount: f64,

    #[field(rename = "snapshot_date", comment = "快照日期")]
    pub snapshot_date: Option<chrono::NaiveDateTime>,

    #[field(default_value = "1")]
    pub status: i8,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
