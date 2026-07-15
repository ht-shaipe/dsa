//! 情报条目表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(
    name = "intelligence_items",
    comment = "情报条目(去重)",
    primary = "identity"
)]
pub struct IntelligenceItem {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "source_id", comment = "源ID")]
    pub source_id: i64,

    #[field(comment = "标题")]
    pub title: String,

    #[field(comment = "内容")]
    pub content: String,

    #[field(rename = "source_url", unique = true, comment = "原始URL")]
    pub source_url: String,

    #[field(rename = "scope_type")]
    pub scope_type: String,

    #[field(rename = "scope_value")]
    pub scope_value: String,

    #[field(rename = "market", default_value = "'cn'")]
    pub market: String,

    #[field(rename = "published_at")]
    pub published_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "fetched_at", default_value = "current_timestamp()")]
    pub fetched_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(
        rename = "source_name",
        length = 100,
        default_value = "''",
        comment = "来源名称"
    )]
    pub source_name: String,

    #[field(
        rename = "source_type",
        length = 32,
        default_value = "''",
        comment = "来源类型"
    )]
    pub source_type: String,

    #[field(length = 100, default_value = "''", comment = "来源")]
    pub source: String,

    #[field(default_value = "''", comment = "摘要")]
    pub summary: String,

    #[field(rename = "raw_payload", default_value = "''", comment = "原始载荷")]
    pub raw_payload: String,
}
