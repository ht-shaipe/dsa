use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "portfolio_corporate_actions", comment = "组合公司行动(分红/拆股)", primary = "identity")]
pub struct PortfolioCorporateAction {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "account_id", required = true, comment = "账户ID")]
    pub account_id: i64,

    #[field(required = true, comment = "股票代码")]
    pub symbol: String,

    #[field(default_value = "'cn'", comment = "市场")]
    pub market: String,

    #[field(rename = "base_currency", default_value = "'CNY'", comment = "币种")]
    pub base_currency: String,

    #[field(rename = "effective_date", required = true, comment = "生效日期")]
    pub effective_date: Option<chrono::NaiveDateTime>,

    #[field(rename = "action_type", required = true, comment = "类型: cash_dividend/split_adjustment")]
    pub action_type: String,

    #[field(rename = "cash_dividend_per_share", comment = "每股现金分红")]
    pub cash_dividend_per_share: f64,

    #[field(rename = "split_ratio", comment = "拆股比率")]
    pub split_ratio: f64,

    #[field(comment = "备注")]
    pub note: String,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
