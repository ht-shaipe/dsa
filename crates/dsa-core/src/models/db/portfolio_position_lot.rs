use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(
    name = "portfolio_position_lots",
    comment = "组合持仓批次(FIFO)",
    primary = "identity"
)]
pub struct PortfolioPositionLot {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "account_id", required = true, comment = "账户ID")]
    pub account_id: i64,

    #[field(
        rename = "cost_method",
        default_value = "'fifo'",
        comment = "成本方法: fifo/avg"
    )]
    pub cost_method: String,

    #[field(required = true, comment = "股票代码")]
    pub symbol: String,

    #[field(default_value = "'cn'", comment = "市场")]
    pub market: String,

    #[field(rename = "base_currency", default_value = "'CNY'", comment = "币种")]
    pub base_currency: String,

    #[field(rename = "open_date", required = true, comment = "建仓日期")]
    pub open_date: Option<chrono::NaiveDateTime>,

    #[field(
        rename = "remaining_quantity",
        default_value = "0",
        comment = "剩余数量"
    )]
    pub remaining_quantity: f64,

    #[field(rename = "unit_cost", default_value = "0", comment = "单位成本")]
    pub unit_cost: f64,

    #[field(rename = "source_trade_id", comment = "来源交易ID")]
    pub source_trade_id: i64,

    #[field(rename = "updated_time", default_value = "current_timestamp()")]
    pub updated_time: Option<chrono::NaiveDateTime>,
}
