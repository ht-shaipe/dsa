//! 新闻情报表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "news_intel", comment = "新闻情报", primary = "identity")]
pub struct NewsIntel {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "stockCode", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "queryId", comment = "查询批次ID")]
    pub query_id: String,

    #[field(comment = "标题")]
    pub title: String,

    #[field(comment = "摘要")]
    pub summary: String,

    #[field(rename = "sourceUrl", comment = "来源URL")]
    pub source_url: String,

    #[field(comment = "来源")]
    pub source: String,

    #[field(rename = "publishedAt", comment = "发布时间")]
    pub published_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "sentimentLabel", comment = "情感标签")]
    pub sentiment_label: String,

    #[field(rename = "sentimentScore", comment = "情感评分")]
    pub sentiment_score: f64,

    #[field(default_value = "1")]
    pub status: i8,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(length = 50, default_value = "''", comment = "名称")]
    pub name: String,

    #[field(length = 32, default_value = "'latest_news'", comment = "维度")]
    pub dimension: String,

    #[field(length = 255, default_value = "''", comment = "查询")]
    pub query: String,

    #[field(length = 32, default_value = "''", comment = "提供者")]
    pub provider: String,

    #[field(default_value = "''", comment = "摘要片段")]
    pub snippet: String,

    #[field(rename = "querySource", length = 32, default_value = "'system'", comment = "查询来源")]
    pub query_source: String,
}
