use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "decision_signal_feedback", comment = "决策信号用户反馈", primary = "identity")]
pub struct DecisionSignalFeedback {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(rename = "signal_id", required = true, comment = "信号ID")]
    pub signal_id: i64,

    #[field(rename = "feedback_value", required = true, comment = "反馈值: agree/disagree/partial")]
    pub feedback_value: String,

    #[field(rename = "reason_code", comment = "原因码")]
    pub reason_code: String,

    #[field(comment = "备注")]
    pub note: String,

    #[field(default_value = "'api'", comment = "来源")]
    pub source: String,

    #[field(rename = "create_time", default_value = "current_timestamp()")]
    pub create_time: Option<chrono::NaiveDateTime>,

    #[field(rename = "modify_time", default_value = "current_timestamp()")]
    pub modify_time: Option<chrono::NaiveDateTime>,
}
