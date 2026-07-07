//! 分析历史表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "analysis_history", comment = "分析历史记录", primary = "identity")]
pub struct AnalysisHistory {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "stock_code", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "stock_name", comment = "股票名称")]
    pub stock_name: String,

    #[field(rename = "query_id", comment = "查询批次ID")]
    pub query_id: String,

    #[field(rename = "sentiment_score", comment = "情绪评分")]
    pub sentiment_score: i32,

    #[field(rename = "trend_prediction", comment = "趋势预测")]
    pub trend_prediction: String,

    #[field(rename = "operation_advice", comment = "操作建议")]
    pub operation_advice: String,

    #[field(rename = "decision_type", comment = "决策类型")]
    pub decision_type: String,

    #[field(rename = "confidence_level", comment = "信心水平")]
    pub confidence_level: String,

    #[field(rename = "ideal_buy", comment = "理想买入价")]
    pub ideal_buy: f64,

    #[field(rename = "secondary_buy", comment = "次级买入价")]
    pub secondary_buy: f64,

    #[field(rename = "stop_loss", comment = "止损价")]
    pub stop_loss: f64,

    #[field(rename = "take_profit", comment = "止盈价")]
    pub take_profit: f64,

    #[field(comment = "分析报告JSON")]
    pub report_json: String,

    #[field(rename = "analysis_summary", comment = "分析摘要")]
    pub analysis_summary: String,

    #[field(rename = "risk_warning", comment = "风险提示")]
    pub risk_warning: String,

    #[field(rename = "market_context", comment = "大盘上下文")]
    pub market_context: String,

    #[field(rename = "llm_provider", comment = "LLM提供商")]
    pub llm_provider: String,

    #[field(rename = "llm_model", comment = "LLM模型")]
    pub llm_model: String,

    #[field(rename = "scope_type", default_value = "'watchlist'", comment = "范围类型")]
    pub scope_type: String,

    #[field(rename = "scope_value", default_value = "''", comment = "范围值")]
    pub scope_value: String,

    #[field(default_value = "1", comment = "状态")]
    pub status: i8,

    #[field(rename = "creator_id", default_value = "0")]
    pub creator_id: i64,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "report_type", length = 16, default_value = "'full'", comment = "报告类型")]
    pub report_type: String,

    #[field(rename = "news_content", default_value = "''", comment = "新闻内容")]
    pub news_content: String,

    #[field(rename = "context_snapshot", default_value = "''", comment = "上下文快照")]
    pub context_snapshot: String,
}
