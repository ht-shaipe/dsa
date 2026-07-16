use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(
    name = "stock_pool",
    comment = "本地股票池",
    primary = "identity"
)]
pub struct StockPool {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(
        unique = true,
        required = true,
        rename = "stock_code",
        length = 16,
        comment = "股票代码"
    )]
    pub stock_code: String,

    #[field(
        required = true,
        rename = "stock_name",
        length = 64,
        default_value = "''",
        comment = "股票名称"
    )]
    pub stock_name: String,

    #[field(
        rename = "market",
        length = 8,
        default_value = "'cn'",
        comment = "市场"
    )]
    pub market: String,

    #[field(rename = "market_id", default_value = "0", comment = "市场ID 0=深圳 1=上海")]
    pub market_id: i8,

    #[field(
        rename = "industry",
        length = 32,
        default_value = "''",
        comment = "行业"
    )]
    pub industry: String,

    #[field(rename = "list_date", comment = "上市日期")]
    pub list_date: Option<chrono::NaiveDateTime>,

    #[field(default_value = "1", comment = "状态 1=正常 0=禁用")]
    pub status: i8,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
