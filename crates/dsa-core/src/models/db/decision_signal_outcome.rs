//! 决策信号结果表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(
    name = "decision_signal_outcomes",
    comment = "决策信号回测结果",
    primary = "identity"
)]
pub struct DecisionSignalOutcome {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, rename = "signal_id", comment = "信号ID")]
    pub signal_id: i64,

    #[field(rename = "stock_code", comment = "股票代码")]
    pub stock_code: String,

    #[field(required = true, rename = "eval_horizon", comment = "评估天数")]
    pub eval_horizon: i32,

    #[field(rename = "eval_date", comment = "评估日期")]
    pub eval_date: Option<chrono::NaiveDateTime>,

    #[field(rename = "actual_return", comment = "实际回报%")]
    pub actual_return: f64,

    #[field(rename = "max_drawdown", comment = "最大回撤%")]
    pub max_drawdown: f64,

    #[field(rename = "direction_correct", comment = "方向正确")]
    pub direction_correct: bool,

    #[field(rename = "hit_target", comment = "是否达到目标")]
    pub hit_target: bool,

    #[field(rename = "hit_stop_loss", comment = "是否触及止损")]
    pub hit_stop_loss: bool,

    #[field(default_value = "1")]
    pub status: i8,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,
}
