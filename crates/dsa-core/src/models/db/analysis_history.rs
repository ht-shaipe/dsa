//! 分析历史表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "analysis_history", comment = "分析历史记录", primary = "identity")]
pub struct AnalysisHistory {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "stockCode", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "stockName", comment = "股票名称")]
    pub stock_name: String,

    #[field(rename = "queryId", comment = "查询批次ID")]
    pub query_id: String,

    #[field(rename = "sentimentScore", comment = "情绪评分")]
    pub sentiment_score: i32,

    #[field(rename = "trendPrediction", comment = "趋势预测")]
    pub trend_prediction: String,

    #[field(rename = "operationAdvice", comment = "操作建议")]
    pub operation_advice: String,

    #[field(rename = "decisionType", comment = "决策类型")]
    pub decision_type: String,

    #[field(rename = "confidenceLevel", comment = "信心水平")]
    pub confidence_level: String,

    #[field(rename = "idealBuy", comment = "理想买入价")]
    pub ideal_buy: f64,

    #[field(rename = "secondaryBuy", comment = "次级买入价")]
    pub secondary_buy: f64,

    #[field(rename = "stopLoss", comment = "止损价")]
    pub stop_loss: f64,

    #[field(rename = "takeProfit", comment = "止盈价")]
    pub take_profit: f64,

    #[field(comment = "分析报告JSON")]
    pub report_json: String,

    #[field(rename = "analysisSummary", comment = "分析摘要")]
    pub analysis_summary: String,

    #[field(rename = "riskWarning", comment = "风险提示")]
    pub risk_warning: String,

    #[field(rename = "marketContext", comment = "大盘上下文")]
    pub market_context: String,

    #[field(rename = "llmProvider", comment = "LLM提供商")]
    pub llm_provider: String,

    #[field(rename = "llmModel", comment = "LLM模型")]
    pub llm_model: String,

    #[field(rename = "scopeType", default_value = "'watchlist'", comment = "范围类型")]
    pub scope_type: String,

    #[field(rename = "scopeValue", default_value = "''", comment = "范围值")]
    pub scope_value: String,

    #[field(default_value = "1", comment = "状态")]
    pub status: i8,

    #[field(rename = "creatorId", default_value = "0")]
    pub creator_id: i64,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modifyTime", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "reportType", length = 16, default_value = "'full'", comment = "报告类型")]
    pub report_type: String,

    #[field(rename = "newsContent", default_value = "''", comment = "新闻内容")]
    pub news_content: String,

    #[field(rename = "contextSnapshot", default_value = "''", comment = "上下文快照")]
    pub context_snapshot: String,
}
