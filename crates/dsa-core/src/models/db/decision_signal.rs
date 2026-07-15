//! 决策信号表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "decision_signals", comment = "决策信号", primary = "identity")]
pub struct DecisionSignal {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "stock_code", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "stock_name", comment = "股票名称")]
    pub stock_name: String,

    #[field(required = true, rename = "signal_date", comment = "信号日期")]
    pub signal_date: Option<chrono::NaiveDateTime>,

    #[field(
        required = true,
        comment = "动作: buy/add/hold/reduce/sell/watch/avoid/alert"
    )]
    pub action: String,

    #[field(rename = "sentiment_score", comment = "情绪评分")]
    pub sentiment_score: i32,

    #[field(rename = "confidence_level", comment = "信心水平")]
    pub confidence_level: String,

    #[field(rename = "entry_price", comment = "入场价")]
    pub entry_price: f64,

    #[field(rename = "stop_loss", comment = "止损价")]
    pub stop_loss: f64,

    #[field(rename = "target_price", comment = "目标价")]
    pub target_price: f64,

    #[field(comment = "理由")]
    pub reasoning: String,

    #[field(comment = "证据JSON")]
    pub evidence: String,

    #[field(rename = "scope_type", default_value = "'watchlist'")]
    pub scope_type: String,

    #[field(rename = "scope_value", default_value = "''")]
    pub scope_value: String,

    #[field(rename = "analysis_id", comment = "关联分析ID")]
    pub analysis_id: i64,

    #[field(rename = "plan_quality", comment = "计划质量评分")]
    pub plan_quality: f64,

    #[field(default_value = "1", comment = "状态")]
    pub status: i8,

    #[field(rename = "creator_id", default_value = "0")]
    pub creator_id: i64,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "market", length = 8, default_value = "'A'", comment = "市场")]
    pub market: String,

    #[field(
        rename = "source_type",
        length = 32,
        default_value = "''",
        comment = "来源类型"
    )]
    pub source_type: String,

    #[field(
        rename = "source_agent",
        length = 64,
        default_value = "''",
        comment = "来源Agent"
    )]
    pub source_agent: String,

    #[field(
        rename = "source_report_id",
        default_value = "0",
        comment = "来源报告ID"
    )]
    pub source_report_id: i32,

    #[field(
        rename = "trace_id",
        length = 64,
        default_value = "''",
        comment = "追踪ID"
    )]
    pub trace_id: String,

    #[field(
        rename = "market_phase",
        length = 24,
        default_value = "''",
        comment = "市场阶段"
    )]
    pub market_phase: String,

    #[field(
        rename = "trigger_source",
        length = 64,
        default_value = "''",
        comment = "触发源"
    )]
    pub trigger_source: String,

    #[field(default_value = "0", comment = "置信度")]
    pub confidence: f64,

    #[field(default_value = "0", comment = "评分")]
    pub score: i32,

    #[field(length = 16, default_value = "'medium'", comment = "时间跨度")]
    pub horizon: String,

    #[field(rename = "entry_low", default_value = "0", comment = "入场低价")]
    pub entry_low: f64,

    #[field(rename = "entry_high", default_value = "0", comment = "入场高价")]
    pub entry_high: f64,

    #[field(default_value = "''", comment = "失效条件")]
    pub invalidation: String,

    #[field(
        rename = "watch_conditions",
        default_value = "''",
        comment = "观察条件"
    )]
    pub watch_conditions: String,

    #[field(default_value = "''", comment = "理由")]
    pub reason: String,

    #[field(rename = "risk_summary", default_value = "''", comment = "风险摘要")]
    pub risk_summary: String,

    #[field(
        rename = "catalyst_summary",
        default_value = "''",
        comment = "催化摘要"
    )]
    pub catalyst_summary: String,

    #[field(rename = "evidence_json", default_value = "''", comment = "证据JSON")]
    pub evidence_json: String,

    #[field(
        rename = "plan_quality_label",
        length = 16,
        default_value = "''",
        comment = "计划质量标签"
    )]
    pub plan_quality_label: String,

    #[field(
        rename = "signal_status",
        length = 16,
        default_value = "'active'",
        comment = "信号状态"
    )]
    pub signal_status: String,

    #[field(rename = "expires_at", comment = "过期时间")]
    pub expires_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "metadata_json", default_value = "''", comment = "元数据JSON")]
    pub metadata_json: String,
}
