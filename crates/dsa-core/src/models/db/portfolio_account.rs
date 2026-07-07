//! 组合账户表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "portfolio_accounts", comment = "组合账户", primary = "identity")]
pub struct PortfolioAccount {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, comment = "账户名称")]
    pub name: String,

    #[field(required = true, comment = "市场: cn/hk/us")]
    pub market: String,

    #[field(comment = "券商")]
    pub broker: String,

    #[field(rename = "base_currency", default_value = "'CNY'")]
    pub base_currency: String,

    #[field(rename = "initial_capital", comment = "初始资金")]
    pub initial_capital: f64,

    #[field(comment = "备注")]
    pub remark: String,

    #[field(default_value = "1")]
    pub status: i8,

    #[field(rename = "creator_id", default_value = "0")]
    pub creator_id: i64,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
