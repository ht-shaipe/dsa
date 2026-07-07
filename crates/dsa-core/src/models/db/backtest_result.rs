//! 回测结果表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "backtest_results", comment = "回测结果", primary = "identity")]
pub struct BacktestResult {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "analysisId", comment = "分析ID")]
    pub analysis_id: i64,

    #[field(rename = "stockCode", comment = "股票代码")]
    pub stock_code: String,

    #[field(rename = "signalDate", comment = "信号日期")]
    pub signal_date: Option<chrono::NaiveDateTime>,

    #[field(rename = "decisionAction", comment = "决策动作")]
    pub decision_action: String,

    #[field(rename = "simulatedEntry", comment = "模拟入场价")]
    pub simulated_entry: f64,

    #[field(rename = "simulatedExit", comment = "模拟出场价")]
    pub simulated_exit: f64,

    #[field(rename = "exitDate", comment = "出场日期")]
    pub exit_date: Option<chrono::NaiveDateTime>,

    #[field(rename = "returnPct", comment = "回报%")]
    pub return_pct: f64,

    #[field(rename = "maxDrawdown", comment = "最大回撤%")]
    pub max_drawdown: f64,

    #[field(rename = "directionCorrect", comment = "方向正确")]
    pub direction_correct: bool,

    #[field(rename = "scopeType", default_value = "'watchlist'")]
    pub scope_type: String,

    #[field(default_value = "1")]
    pub status: i8,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "evalWindowDays", default_value = "10", comment = "评估窗口天数")]
    pub eval_window_days: i32,

    #[field(rename = "engineVersion", length = 16, default_value = "''", comment = "引擎版本")]
    pub engine_version: String,

    #[field(rename = "evalStatus", length = 16, default_value = "'pending'", comment = "评估状态")]
    pub eval_status: String,

    #[field(rename = "evaluatedAt", comment = "评估时间")]
    pub evaluated_at: Option<chrono::NaiveDateTime>,

    #[field(rename = "operationAdvice", length = 20, default_value = "''", comment = "操作建议")]
    pub operation_advice: String,

    #[field(rename = "positionRecommendation", length = 8, default_value = "''", comment = "仓位建议")]
    pub position_recommendation: String,

    #[field(rename = "startPrice", default_value = "0", comment = "起始价")]
    pub start_price: f64,

    #[field(rename = "endClose", default_value = "0", comment = "结束收盘价")]
    pub end_close: f64,

    #[field(rename = "maxHigh", default_value = "0", comment = "最高价")]
    pub max_high: f64,

    #[field(rename = "minLow", default_value = "0", comment = "最低价")]
    pub min_low: f64,

    #[field(rename = "stockReturnPct", default_value = "0", comment = "股票回报%")]
    pub stock_return_pct: f64,

    #[field(rename = "directionExpected", length = 16, default_value = "''", comment = "预期方向")]
    pub direction_expected: String,

    #[field(length = 16, default_value = "''", comment = "结果")]
    pub outcome: String,

    #[field(rename = "stopLossPrice", default_value = "0", comment = "止损价")]
    pub stop_loss_price: f64,

    #[field(rename = "takeProfitPrice", default_value = "0", comment = "止盈价")]
    pub take_profit_price: f64,

    #[field(rename = "hitStopLoss", default_value = "0", comment = "是否触及止损")]
    pub hit_stop_loss: i8,

    #[field(rename = "hitTakeProfit", default_value = "0", comment = "是否触及止盈")]
    pub hit_take_profit: i8,

    #[field(rename = "firstHit", length = 16, default_value = "''", comment = "首次触及类型")]
    pub first_hit: String,

    #[field(rename = "firstHitDate", comment = "首次触及日期")]
    pub first_hit_date: Option<chrono::NaiveDate>,

    #[field(rename = "firstHitTradingDays", default_value = "0", comment = "首次触及交易天数")]
    pub first_hit_trading_days: i32,

    #[field(rename = "simulatedExitReason", length = 24, default_value = "''", comment = "模拟出场原因")]
    pub simulated_exit_reason: String,

    #[field(rename = "simulatedReturnPct", default_value = "0", comment = "模拟回报%")]
    pub simulated_return_pct: f64,
}
