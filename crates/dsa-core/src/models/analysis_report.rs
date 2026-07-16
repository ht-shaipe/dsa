//! 分析报告数据结构 - 对齐原项目 report_schema.py

use serde::{Deserialize, Serialize};

/// 持仓操作建议（按是否已持仓区分）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionAdvice {
    pub no_position: Option<String>,
    pub has_position: Option<String>,
}

/// 核心结论摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConclusion {
    pub one_sentence: Option<String>,
    pub signal_type: Option<String>,
    pub time_sensitivity: Option<String>,
    pub position_advice: Option<PositionAdvice>,
}

/// 趋势状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendStatus {
    pub ma_alignment: Option<String>,
    pub is_bullish: Option<bool>,
    pub trend_score: Option<f64>,
}

/// 价格位置与均线偏离
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePosition {
    pub current_price: Option<f64>,
    pub ma5: Option<f64>,
    pub ma10: Option<f64>,
    pub ma20: Option<f64>,
    pub bias_ma5: Option<f64>,
    pub bias_status: Option<String>,
    pub support_level: Option<f64>,
    pub resistance_level: Option<f64>,
}

/// 成交量分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeAnalysis {
    pub volume_ratio: Option<f64>,
    pub volume_status: Option<String>,
    pub turnover_rate: Option<f64>,
    pub volume_meaning: Option<String>,
}

/// 筹码结构分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipStructure {
    pub profit_ratio: Option<f64>,
    pub avg_cost: Option<f64>,
    pub concentration: Option<f64>,
    pub chip_health: Option<String>,
}

/// 数据视角（趋势/价格/量能/筹码）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPerspective {
    pub trend_status: Option<TrendStatus>,
    pub price_position: Option<PricePosition>,
    pub volume_analysis: Option<VolumeAnalysis>,
    pub chip_structure: Option<ChipStructure>,
}

/// 情报信息（新闻/风险/催化剂/业绩/情绪）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intelligence {
    pub latest_news: Option<String>,
    pub risk_alerts: Option<Vec<String>>,
    pub positive_catalysts: Option<Vec<String>>,
    pub earnings_outlook: Option<String>,
    pub sentiment_summary: Option<String>,
}

/// 狙击点位（买入/止损/止盈）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperPoints {
    pub ideal_buy: Option<f64>,
    pub secondary_buy: Option<f64>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

/// 仓位策略建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionStrategy {
    pub suggested_position: Option<String>,
    pub entry_plan: Option<String>,
    pub risk_control: Option<String>,
}

/// 作战计划（点位/仓位/行动清单）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattlePlan {
    pub sniper_points: Option<SniperPoints>,
    pub position_strategy: Option<PositionStrategy>,
    pub action_checklist: Option<Vec<String>>,
}

/// 阶段决策（行动窗口/观察条件/下次检查时间）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseDecision {
    pub phase_context: Option<serde_json::Value>,
    pub action_window: Option<String>,
    pub immediate_action: Option<String>,
    pub watch_conditions: Vec<String>,
    pub next_check_time: Option<String>,
    pub confidence_reason: Option<String>,
    pub data_limitations: Vec<String>,
}

/// 信号归因（技术/情绪/基本面/市场环境权重）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalAttribution {
    pub technical_indicators: Option<f64>,
    pub news_sentiment: Option<f64>,
    pub fundamentals: Option<f64>,
    pub market_conditions: Option<f64>,
    pub strongest_bullish_signal: Option<String>,
    pub strongest_bearish_signal: Option<String>,
}

/// 仪表盘（核心结论/数据视角/情报/作战计划/阶段决策/信号归因）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub core_conclusion: Option<CoreConclusion>,
    pub data_perspective: Option<DataPerspective>,
    pub intelligence: Option<Intelligence>,
    pub battle_plan: Option<BattlePlan>,
    pub phase_decision: Option<PhaseDecision>,
    pub signal_attribution: Option<SignalAttribution>,
}

/// 完整分析报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub stock_name: Option<String>,
    pub sentiment_score: Option<i32>,
    pub trend_prediction: Option<String>,
    pub operation_advice: Option<String>,
    pub decision_type: Option<String>,
    pub confidence_level: Option<String>,

    pub dashboard: Option<Dashboard>,

    pub analysis_summary: Option<String>,
    pub key_points: Option<String>,
    pub risk_warning: Option<String>,
    pub buy_reason: Option<String>,

    pub trend_analysis: Option<String>,
    pub short_term_outlook: Option<String>,
    pub medium_term_outlook: Option<String>,
    pub technical_analysis: Option<String>,
    pub ma_analysis: Option<String>,
    pub volume_analysis: Option<String>,
    pub pattern_analysis: Option<String>,
    pub fundamental_analysis: Option<String>,
    pub sector_position: Option<String>,
    pub company_highlights: Option<String>,
    pub news_summary: Option<String>,
    pub market_sentiment: Option<String>,
    pub hot_topics: Option<String>,

    pub search_performed: Option<bool>,
    pub data_sources: Option<String>,

    #[serde(rename = "dataAsOf")]
    pub data_as_of: Option<String>,
}

impl AnalysisReport {
    /// 根据决策类型返回对应表情符号
    pub fn action_emoji(&self) -> &str {
        match self.decision_type.as_deref() {
            Some("buy") | Some("add") => "🟢",
            Some("hold") | Some("watch") => "🟡",
            Some("sell") | Some("reduce") => "🔴",
            Some("avoid") | Some("alert") => "⚠️",
            _ => "⚪",
        }
    }

    /// 根据决策类型返回中文操作标签
    pub fn action_label(&self) -> &str {
        match self.decision_type.as_deref() {
            Some("buy") => "买入",
            Some("add") => "加仓",
            Some("hold") => "持有",
            Some("reduce") => "减仓",
            Some("sell") => "卖出",
            Some("watch") => "观望",
            Some("avoid") => "回避",
            Some("alert") => "预警",
            _ => "未知",
        }
    }

    /// 根据情绪分数返回对应表情符号
    pub fn score_emoji(&self) -> &str {
        match self.sentiment_score {
            Some(s) if s >= 70 => "🟢",
            Some(s) if s >= 40 => "🟡",
            Some(_) => "🔴",
            None => "⚪",
        }
    }
}
