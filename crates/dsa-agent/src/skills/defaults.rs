//! 默认技能 - 交易策略技能定义

use super::router::Skill;
use tube::Value;

/// 多头趋势技能 - 在上升趋势中买入
pub struct BullTrendSkill;

impl Skill for BullTrendSkill {
    fn name(&self) -> &str { "bull_trend" }

    fn evaluate(&self, context: &Value) -> Value {
        // 从上下文获取趋势分析结果
        let trend = context.get("trend")
            .and_then(|t| t.as_str())
            .unwrap_or_default();

        let strength = match trend.as_str() {
            "strong_up" => 85,
            "up" => 65,
            "sideways" => 20,
            "down" | "strong_down" => 5,
            _ => 10,
        };

        value!({
            "skill": self.name(),
            "strength": strength,
            "reason": if strength > 50 { "趋势向上，可考虑买入" } else if strength > 20 { "趋势不明朗，观望为主" } else { "趋势向下，不宜追涨" },
        })
    }
}

/// 缩量回调技能 - 缩量回调时买入
pub struct ShrinkPullbackSkill;

impl Skill for ShrinkPullbackSkill {
    fn name(&self) -> &str { "shrink_pullback" }

    fn evaluate(&self, context: &Value) -> Value {
        let vol_signal = context.get("volumeSignal")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let trend = context.get("trend")
            .and_then(|t| t.as_str())
            .unwrap_or_default();

        // 缩量 + 上升趋势回调 = 好买点
        let strength = match (trend.as_str(), vol_signal.as_str()) {
            ("up" | "strong_up", "shrink_volume" | "slight_shrink") => 80,
            ("up" | "strong_up", "normal") => 50,
            ("sideways", "shrink_volume") => 40,
            (_, "huge_volume") => 15, // 放量风险大
            _ => 20,
        };

        value!({
            "skill": self.name(),
            "strength": strength,
            "reason": if strength > 60 { "缩量回调，可能是好的买入时机" } else if strength > 30 { "量价配合不明显" } else { "量价异常，需谨慎" },
        })
    }
}

/// 筹码集中技能 - 关注筹码集中股
pub struct ChipFocusSkill;

impl Skill for ChipFocusSkill {
    fn name(&self) -> &str { "chip_focus" }

    fn evaluate(&self, context: &Value) -> Value {
        // 筹码集中度评估
        let chip_concentration = context.get("chipConcentration")
            .and_then(|c| c.as_f64())
            .unwrap_or(50.0);

        // 筹码越集中，分数越高
        let strength = if chip_concentration > 80.0 {
            80
        } else if chip_concentration > 60.0 {
            60
        } else if chip_concentration > 40.0 {
            40
        } else {
            15
        };

        value!({
            "skill": self.name(),
            "strength": strength,
            "reason": if strength > 60 { "筹码较为集中，主力控盘明显" } else if strength > 30 { "筹码分散，关注为主" } else { "筹码极度分散，不建议介入" },
        })
    }
}

/// 不追高技能 - 避免追高开盘
pub struct NoChaseSkill;

impl Skill for NoChaseSkill {
    fn name(&self) -> &str { "no_chase" }

    fn evaluate(&self, context: &Value) -> Value {
        let change_pct = context.get("changePercent")
            .and_then(|c| c.as_f64())
            .unwrap_or(0.0);

        // 涨幅越大，追高风险越大（strength越低表示越不建议买入）
        let strength = if change_pct > 7.0 {
            5  // 涨幅太大，强烈不建议追
        } else if change_pct > 5.0 {
            15
        } else if change_pct > 3.0 {
            30
        } else if change_pct > 0.0 {
            60
        } else if change_pct > -3.0 {
            75  // 小幅下跌可能是机会
        } else {
            40  // 大幅下跌需观察
        };

        value!({
            "skill": self.name(),
            "strength": strength,
            "reason": if change_pct > 5.0 { "涨幅过大，不宜追高" } else if change_pct > 0.0 { "涨幅适中，可适度参与" } else if change_pct > -3.0 { "小幅回调，可关注买入机会" } else { "跌幅较大，需观察企稳信号" },
        })
    }
}

/// 获取所有默认技能
pub fn default_skills() -> Vec<Box<dyn Skill>> {
    vec![
        Box::new(BullTrendSkill),
        Box::new(ShrinkPullbackSkill),
        Box::new(ChipFocusSkill),
        Box::new(NoChaseSkill),
    ]
}
