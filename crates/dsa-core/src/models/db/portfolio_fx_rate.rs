use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "portfolio_fx_rates", comment = "组合汇率缓存", primary = "identity")]
pub struct PortfolioFxRate {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "fromCurrency", required = true, comment = "源币种")]
    pub from_currency: String,

    #[field(rename = "toCurrency", required = true, comment = "目标币种")]
    pub to_currency: String,

    #[field(rename = "rateDate", required = true, comment = "汇率日期")]
    pub rate_date: Option<chrono::NaiveDateTime>,

    #[field(required = true, comment = "汇率")]
    pub rate: f64,

    #[field(default_value = "'manual'", comment = "数据来源")]
    pub source: String,

    #[field(rename = "isStale", default_value = "0", comment = "是否过期")]
    pub is_stale: i8,

    #[field(rename = "updatedTime", default_value = "current_timestamp()")]
    pub updated_time: Option<chrono::NaiveDateTime>,
}
