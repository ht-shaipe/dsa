//! 自选股表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(
    name = "watchlist_stocks",
    comment = "自选股列表",
    primary = "identity"
)]
pub struct WatchlistStock {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(
        required = true,
        rename = "stock_code",
        length = 16,
        comment = "股票代码"
    )]
    pub stock_code: String,

    #[field(
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

    #[field(
        rename = "group_name",
        length = 32,
        default_value = "'default'",
        comment = "分组"
    )]
    pub group_name: String,

    #[field(rename = "sort_order", default_value = "1", comment = "排序权重")]
    pub sort_order: i32,

    #[field(default_value = "1", comment = "启用")]
    pub enabled: i8,

    #[field(default_value = "''", comment = "备注")]
    pub remark: String,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
