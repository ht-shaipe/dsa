use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "backtest_summaries", comment = "回测汇总", primary = "identity")]
pub struct BacktestSummary {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, comment = "范围: overall/stock")]
    pub scope: String,

    #[field(comment = "股票代码(stock scope时)")]
    pub code: String,

    #[field(rename = "eval_window_days", default_value = "10", comment = "评估窗口天数")]
    pub eval_window_days: i32,

    #[field(rename = "engine_version", default_value = "'v1'", comment = "引擎版本")]
    pub engine_version: String,

    #[field(rename = "computed_at", default_value = "current_timestamp()")]
    pub computed_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "total_evaluations", default_value = "0")]
    pub total_evaluations: i32,

    #[field(rename = "completed_count", default_value = "0")]
    pub completed_count: i32,

    #[field(rename = "insufficient_count", default_value = "0")]
    pub insufficient_count: i32,

    #[field(rename = "long_count", default_value = "0")]
    pub long_count: i32,

    #[field(rename = "cash_count", default_value = "0")]
    pub cash_count: i32,

    #[field(rename = "win_count", default_value = "0")]
    pub win_count: i32,

    #[field(rename = "loss_count", default_value = "0")]
    pub loss_count: i32,

    #[field(rename = "neutral_count", default_value = "0")]
    pub neutral_count: i32,

    #[field(rename = "direction_accuracy_pct", comment = "方向准确率%")]
    pub direction_accuracy_pct: f64,

    #[field(rename = "win_rate_pct", comment = "胜率%")]
    pub win_rate_pct: f64,

    #[field(rename = "neutral_rate_pct", comment = "中性率%")]
    pub neutral_rate_pct: f64,

    #[field(rename = "avg_stock_return_pct", comment = "平均股票收益%")]
    pub avg_stock_return_pct: f64,

    #[field(rename = "avg_simulated_return_pct", comment = "平均模拟收益%")]
    pub avg_simulated_return_pct: f64,

    #[field(rename = "stop_loss_trigger_rate", comment = "止损触发率")]
    pub stop_loss_trigger_rate: f64,

    #[field(rename = "take_profit_trigger_rate", comment = "止盈触发率")]
    pub take_profit_trigger_rate: f64,

    #[field(rename = "ambiguous_rate", comment = "模糊率")]
    pub ambiguous_rate: f64,

    #[field(rename = "avg_days_to_first_hit", comment = "首次命中平均天数")]
    pub avg_days_to_first_hit: f64,

    #[field(rename = "advice_breakdown_json", comment = "建议分布JSON")]
    pub advice_breakdown_json: String,

    #[field(rename = "diagnostics_json", comment = "诊断JSON")]
    pub diagnostics_json: String,
}
