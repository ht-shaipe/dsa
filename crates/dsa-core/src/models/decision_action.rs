//! 决策动作枚举 - 对齐原项目 decision_action.py

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionAction {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "add")]
    Add,
    #[serde(rename = "hold")]
    Hold,
    #[serde(rename = "reduce")]
    Reduce,
    #[serde(rename = "sell")]
    Sell,
    #[serde(rename = "watch")]
    Watch,
    #[serde(rename = "avoid")]
    Avoid,
    #[serde(rename = "alert")]
    Alert,
}

impl DecisionAction {
    pub fn label_zh(&self) -> &str {
        match self {
            DecisionAction::Buy => "买入",
            DecisionAction::Add => "加仓",
            DecisionAction::Hold => "持有",
            DecisionAction::Reduce => "减仓",
            DecisionAction::Sell => "卖出",
            DecisionAction::Watch => "观望",
            DecisionAction::Avoid => "回避",
            DecisionAction::Alert => "预警",
        }
    }

    pub fn label_en(&self) -> &str {
        match self {
            DecisionAction::Buy => "Buy",
            DecisionAction::Add => "Add",
            DecisionAction::Hold => "Hold",
            DecisionAction::Reduce => "Reduce",
            DecisionAction::Sell => "Sell",
            DecisionAction::Watch => "Watch",
            DecisionAction::Avoid => "Avoid",
            DecisionAction::Alert => "Alert",
        }
    }

    pub fn emoji(&self) -> &str {
        match self {
            DecisionAction::Buy | DecisionAction::Add => "🟢",
            DecisionAction::Hold | DecisionAction::Watch => "🟡",
            DecisionAction::Sell | DecisionAction::Reduce => "🔴",
            DecisionAction::Avoid | DecisionAction::Alert => "⚠️",
        }
    }

    pub fn from_score(score: i32) -> Self {
        match score {
            s if s >= 80 => DecisionAction::Buy,
            s if s >= 65 => DecisionAction::Add,
            s if s >= 50 => DecisionAction::Hold,
            s if s >= 40 => DecisionAction::Watch,
            s if s >= 25 => DecisionAction::Reduce,
            s if s >= 15 => DecisionAction::Sell,
            s if s >= 5 => DecisionAction::Avoid,
            _ => DecisionAction::Alert,
        }
    }

    pub fn from_advice(advice: &str) -> Option<Self> {
        let normalized = advice.to_lowercase().replace('_', " ").trim().to_string();
        let mapping = [
            (&["买入", "强烈买入", "布局", "建仓", "strong buy", "buy"][..], DecisionAction::Buy),
            (&["加仓", "增持", "accumulate", "add"], DecisionAction::Add),
            (&["持有", "持有观察", "hold"], DecisionAction::Hold),
            (&["减仓", "trim", "reduce"], DecisionAction::Reduce),
            (&["卖出", "强烈卖出", "清仓", "strong sell", "sell"], DecisionAction::Sell),
            (&["观望", "等待", "wait", "watch"], DecisionAction::Watch),
            (&["回避", "规避", "avoid", "不建议买入"], DecisionAction::Avoid),
            (&["预警", "风险预警", "alert"], DecisionAction::Alert),
        ];

        for (phrases, action) in mapping {
            for phrase in phrases {
                if normalized.contains(phrase) {
                    return Some(action);
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionActionFields {
    pub action: Option<DecisionAction>,
    pub action_label: Option<String>,
}

pub fn build_action_fields(
    operation_advice: Option<&str>,
    explicit_action: Option<&str>,
    sentiment_score: Option<i32>,
) -> DecisionActionFields {
    let action = explicit_action
        .and_then(|a| DecisionAction::from_advice(a))
        .or_else(|| operation_advice.and_then(|a| DecisionAction::from_advice(a)))
        .or_else(|| sentiment_score.map(|s| DecisionAction::from_score(s)));

    DecisionActionFields {
        action_label: action.map(|a| a.label_zh().to_string()),
        action,
    }
}
