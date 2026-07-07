//! 情报源表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "intelligence_sources", comment = "情报数据源配置", primary = "identity")]
pub struct IntelligenceSource {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, comment = "名称")]
    pub name: String,

    #[field(required = true, rename = "source_type", comment = "类型")]
    pub source_type: String,

    #[field(rename = "url_template", comment = "URL模板")]
    pub url_template: String,

    #[field(rename = "config_json", comment = "配置JSON")]
    pub config_json: String,

    #[field(rename = "scope_type", default_value = "'all'")]
    pub scope_type: String,

    #[field(rename = "scope_value", default_value = "''")]
    pub scope_value: String,

    #[field(rename = "market", default_value = "'cn'")]
    pub market: String,

    #[field(default_value = "1", comment = "启用状态")]
    pub enabled: i8,

    #[field(rename = "fetch_interval", default_value = "60")]
    pub fetch_interval: i32,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,

    #[field(default_value = "''", comment = "URL")]
    pub url: String,

    #[field(default_value = "''", comment = "描述")]
    pub description: String,

    #[field(rename = "last_status", length = 32, default_value = "''", comment = "最后状态")]
    pub last_status: String,

    #[field(rename = "last_error", default_value = "''", comment = "最后错误")]
    pub last_error: String,

    #[field(rename = "last_fetched_at", comment = "最后抓取时间")]
    pub last_fetched_at: Option<chrono::NaiveDateTime>,
}
