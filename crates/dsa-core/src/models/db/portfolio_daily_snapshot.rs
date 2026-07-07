//! 组合每日快照表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "portfolio_daily_snapshots", comment = "组合每日快照", primary = "identity")]
pub struct PortfolioDailySnapshot {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "account_id", comment = "账户ID")]
    pub account_id: i64,

    #[field(required = true, rename = "snapshot_date", comment = "快照日期")]
    pub snapshot_date: Option<chrono::NaiveDateTime>,

    #[field(rename = "total_equity", comment = "总权益")]
    pub total_equity: f64,

    #[field(rename = "cash_balance", comment = "现金余额")]
    pub cash_balance: f64,

    #[field(rename = "market_value", comment = "持仓市值")]
    pub market_value: f64,

    #[field(rename = "daily_pnl", comment = "当日盈亏")]
    pub daily_pnl: f64,

    #[field(rename = "daily_pnl_pct", comment = "当日盈亏%")]
    pub daily_pnl_pct: f64,

    #[field(rename = "total_pnl", comment = "累计盈亏")]
    pub total_pnl: f64,

    #[field(rename = "total_pnl_pct", comment = "累计盈亏%")]
    pub total_pnl_pct: f64,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
