use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "fundamental_snapshot", comment = "基本面快照", primary = "identity")]
pub struct FundamentalSnapshot {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "query_id", required = true, comment = "查询批次ID")]
    pub query_id: String,

    #[field(rename = "stock_code", required = true, comment = "股票代码")]
    pub stock_code: String,

    #[field(comment = "基本面数据JSON")]
    pub payload: String,

    #[field(rename = "source_chain", comment = "数据来源链")]
    pub source_chain: String,

    #[field(comment = "覆盖率JSON")]
    pub coverage: String,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
