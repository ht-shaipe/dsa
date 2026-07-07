//! 决策信号结果表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "decision_signal_outcomes", comment = "决策信号回测结果", primary = "identity")]
pub struct DecisionSignalOutcome {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "signalId", comment = "信号ID")]
    pub signal_id: i64,

    #[field(rename = "stockCode", comment = "股票代码")]
    pub stock_code: String,

    #[field(required = true, rename = "evalHorizon", comment = "评估天数")]
    pub eval_horizon: i32,

    #[field(rename = "evalDate", comment = "评估日期")]
    pub eval_date: Option<chrono::NaiveDateTime>,

    #[field(rename = "actualReturn", comment = "实际回报%")]
    pub actual_return: f64,

    #[field(rename = "maxDrawdown", comment = "最大回撤%")]
    pub max_drawdown: f64,

    #[field(rename = "directionCorrect", comment = "方向正确")]
    pub direction_correct: bool,

    #[field(rename = "hitTarget", comment = "是否达到目标")]
    pub hit_target: bool,

    #[field(rename = "hitStopLoss", comment = "是否触及止损")]
    pub hit_stop_loss: bool,

    #[field(default_value = "1")]
    pub status: i8,

    #[field(rename = "createTime", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
