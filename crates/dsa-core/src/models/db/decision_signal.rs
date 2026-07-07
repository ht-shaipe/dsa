//! 决策信号表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "decision_signals", comment = "决策信号", primary = "identity")]
pub struct DecisionSignal {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "stockCode", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "stockName", comment = "股票名称")]
    pub stock_name: String,

    #[field(required = true, rename = "signalDate", comment = "信号日期")]
    pub signal_date: Option<chrono::NaiveDateTime>,

    #[field(required = true, comment = "动作: buy/add/hold/reduce/sell/watch/avoid/alert")]
    pub action: String,

    #[field(rename = "sentimentScore", comment = "情绪评分")]
    pub sentiment_score: i32,

    #[field(rename = "confidenceLevel", comment = "信心水平")]
    pub confidence_level: String,

    #[field(rename = "entryPrice", comment = "入场价")]
    pub entry_price: f64,

    #[field(rename = "stopLoss", comment = "止损价")]
    pub stop_loss: f64,

    #[field(rename = "targetPrice", comment = "目标价")]
    pub target_price: f64,

    #[field(comment = "理由")]
    pub reasoning: String,

    #[field(comment = "证据JSON")]
    pub evidence: String,

    #[field(rename = "scopeType", default_value = "'watchlist'")]
    pub scope_type: String,

    #[field(rename = "scopeValue", default_value = "''")]
    pub scope_value: String,

    #[field(rename = "analysisId", comment = "关联分析ID")]
    pub analysis_id: i64,

    #[field(rename = "planQuality", comment = "计划质量评分")]
    pub plan_quality: f64,

    #[field(default_value = "1", comment = "状态")]
    pub status: i8,

    #[field(rename = "creatorId", default_value = "0")]
    pub creator_id: i64,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modifyTime", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "market", length = 8, default_value = "'A'", comment = "市场")]
    pub market: String,

    #[field(rename = "sourceType", length = 32, default_value = "''", comment = "来源类型")]
    pub source_type: String,

    #[field(rename = "sourceAgent", length = 64, default_value = "''", comment = "来源Agent")]
    pub source_agent: String,

    #[field(rename = "sourceReportId", default_value = "0", comment = "来源报告ID")]
    pub source_report_id: i32,

    #[field(rename = "traceId", length = 64, default_value = "''", comment = "追踪ID")]
    pub trace_id: String,

    #[field(rename = "marketPhase", length = 24, default_value = "''", comment = "市场阶段")]
    pub market_phase: String,

    #[field(rename = "triggerSource", length = 64, default_value = "''", comment = "触发源")]
    pub trigger_source: String,

    #[field(default_value = "0", comment = "置信度")]
    pub confidence: f64,

    #[field(default_value = "0", comment = "评分")]
    pub score: i32,

    #[field(length = 16, default_value = "'medium'", comment = "时间跨度")]
    pub horizon: String,

    #[field(rename = "entryLow", default_value = "0", comment = "入场低价")]
    pub entry_low: f64,

    #[field(rename = "entryHigh", default_value = "0", comment = "入场高价")]
    pub entry_high: f64,

    #[field(default_value = "''", comment = "失效条件")]
    pub invalidation: String,

    #[field(rename = "watchConditions", default_value = "''", comment = "观察条件")]
    pub watch_conditions: String,

    #[field(default_value = "''", comment = "理由")]
    pub reason: String,

    #[field(rename = "riskSummary", default_value = "''", comment = "风险摘要")]
    pub risk_summary: String,

    #[field(rename = "catalystSummary", default_value = "''", comment = "催化摘要")]
    pub catalyst_summary: String,

    #[field(rename = "evidenceJson", default_value = "''", comment = "证据JSON")]
    pub evidence_json: String,

    #[field(rename = "planQualityLabel", length = 16, default_value = "''", comment = "计划质量标签")]
    pub plan_quality_label: String,

    #[field(rename = "signalStatus", length = 16, default_value = "'active'", comment = "信号状态")]
    pub signal_status: String,

    #[field(rename = "expiresAt", comment = "过期时间")]
    pub expires_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "metadataJson", default_value = "''", comment = "元数据JSON")]
    pub metadata_json: String,
}
