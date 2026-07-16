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

    #[field(unique = true, required = true, rename = "stock_code", length = 32, comment = "股票代码")]
    pub stock_code: String,

    #[field(required = true, rename = "stock_name", length = 64, default_value = "''", comment = "股票名称")]
    pub stock_name: String,

    #[field(rename = "symbol", length = 32, default_value = "''", comment = "带市场和code的标识")]
    pub symbol: String,

    #[field(rename = "en_name", length = 128, default_value = "''", comment = "英文名称")]
    pub en_name: String,

    #[field(rename = "company_name", length = 128, default_value = "''", comment = "公司名称")]
    pub company_name: String,

    #[field(rename = "address", length = 256, default_value = "''", comment = "注册地址")]
    pub address: String,

    #[field(rename = "a_code", length = 32, default_value = "''", comment = "A股代码")]
    pub a_code: String,

    #[field(rename = "a_name", length = 128, default_value = "''", comment = "A股名称")]
    pub a_name: String,

    #[field(rename = "a_cost", comment = "A股总股本")]
    pub a_cost: Option<f64>,

    #[field(rename = "province", length = 128, default_value = "''", comment = "省")]
    pub province: String,

    #[field(rename = "city", length = 128, default_value = "''", comment = "市")]
    pub city: String,

    #[field(rename = "area", length = 128, default_value = "''", comment = "区域")]
    pub area: String,

    #[field(rename = "market", length = 8, default_value = "'cn'", comment = "市场")]
    pub market: String,

    #[field(rename = "market_id", default_value = "0", comment = "市场ID 0=深圳 1=上海")]
    pub market_id: i8,

    #[field(rename = "industry", length = 128, default_value = "''", comment = "行业")]
    pub industry: String,

    #[field(rename = "market_time", comment = "上市时间")]
    pub market_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "website", length = 256, default_value = "''", comment = "官网")]
    pub website: String,

    #[field(rename = "pe", comment = "市盈率")]
    pub pe: Option<f64>,

    #[field(rename = "outstanding", comment = "流通股本（亿）")]
    pub outstanding: Option<f64>,

    #[field(rename = "total", comment = "总股本（亿）")]
    pub total: Option<f64>,

    #[field(rename = "total_assets", comment = "总资产（亿）")]
    pub total_assets: Option<f64>,

    #[field(rename = "flow_assets", comment = "流动资产")]
    pub flow_assets: Option<f64>,

    #[field(rename = "fixed_assets", comment = "固定资产")]
    pub fixed_assets: Option<f64>,

    #[field(rename = "esp", comment = "每股收益")]
    pub esp: Option<f64>,

    #[field(rename = "per_assets", comment = "每股净资")]
    pub per_assets: Option<f64>,

    #[field(rename = "pb", comment = "市净率")]
    pub pb: Option<f64>,

    #[field(rename = "unassigned_profit", default_value = "0", comment = "未分配利润")]
    pub unassigned_profit: Option<f64>,

    #[field(rename = "per_unassigned", comment = "每股未配")]
    pub per_unassigned: Option<f64>,

    #[field(rename = "rev", comment = "收入同比（%）")]
    pub rev: Option<f64>,

    #[field(rename = "profit", comment = "利润同比（%）")]
    pub profit: Option<f64>,

    #[field(rename = "gpr", comment = "毛利率（%）")]
    pub gpr: Option<f64>,

    #[field(rename = "npr", length = 64, default_value = "''", comment = "净利率（%）")]
    pub npr: String,

    #[field(rename = "holders", comment = "股东人数")]
    pub holders: Option<i64>,

    #[field(default_value = "1", comment = "状态 1=正常 0=禁用")]
    pub status: i8,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
