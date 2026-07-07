//! 情报条目表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "intelligence_items", comment = "情报条目(去重)", primary = "identity")]
pub struct IntelligenceItem {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "sourceId", comment = "源ID")]
    pub source_id: i64,

    #[field(comment = "标题")]
    pub title: String,

    #[field(comment = "内容")]
    pub content: String,

    #[field(rename = "sourceUrl", unique = true, comment = "原始URL")]
    pub source_url: String,

    #[field(rename = "scopeType")]
    pub scope_type: String,

    #[field(rename = "scopeValue")]
    pub scope_value: String,

    #[field(rename = "market", default_value = "'cn'")]
    pub market: String,

    #[field(rename = "publishedAt")]
    pub published_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "fetchedAt", default_value = "current_timestamp()")]
    pub fetched_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "sourceName", length = 100, default_value = "''", comment = "来源名称")]
    pub source_name: String,

    #[field(rename = "sourceType", length = 32, default_value = "''", comment = "来源类型")]
    pub source_type: String,

    #[field(length = 100, default_value = "''", comment = "来源")]
    pub source: String,

    #[field(default_value = "''", comment = "摘要")]
    pub summary: String,

    #[field(rename = "rawPayload", default_value = "''", comment = "原始载荷")]
    pub raw_payload: String,
}
