use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(
    name = "portfolio_cash_ledger",
    comment = "组合现金流水",
    primary = "identity"
)]
pub struct PortfolioCashLedger {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "account_id", required = true, comment = "账户ID")]
    pub account_id: i64,

    #[field(rename = "event_date", required = true, comment = "事件日期")]
    pub event_date: Option<chrono::NaiveDateTime>,

    #[field(required = true, comment = "方向: in/out")]
    pub direction: String,

    #[field(required = true, comment = "金额")]
    pub amount: f64,

    #[field(rename = "base_currency", default_value = "'CNY'", comment = "币种")]
    pub base_currency: String,

    #[field(comment = "备注")]
    pub note: String,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
