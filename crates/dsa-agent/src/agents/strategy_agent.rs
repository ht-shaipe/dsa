//! 策略Agent - 策略列表和策略路由

use tube::Value;

/// 策略Agent - 管理交易策略列表并根据市场上下文路由最佳策略
pub struct StrategyAgent;

impl StrategyAgent {
    pub fn new() -> Self {
        Self
    }

    /// 列出所有可用策略
    pub fn list_strategies() -> Vec<Value> {
        vec![
            value!({"id": "bull_trend", "name": "多头趋势", "category": "trend", "description": "均线多头排列，趋势向上"}),
            value!({"id": "ma_golden_cross", "name": "均线金叉", "category": "trend", "description": "MA5上穿MA10/MA20"}),
            value!({"id": "volume_breakout", "name": "放量突破", "category": "momentum", "description": "成交量放大配合价格突破"}),
            value!({"id": "shrink_pullback", "name": "缩量回调", "category": "pullback", "description": "上升趋势中缩量回调到支撑位"}),
            value!({"id": "box_oscillation", "name": "箱体震荡", "category": "range", "description": "价格在区间内反复震荡"}),
            value!({"id": "bottom_volume", "name": "底部放量", "category": "reversal", "description": "底部区域出现放量迹象"}),
            value!({"id": "chan_theory", "name": "缠论", "category": "advanced", "description": "基于缠论的分型/笔/线段分析"}),
            value!({"id": "wave_theory", "name": "波浪理论", "category": "advanced", "description": "艾略特波浪周期分析"}),
            value!({"id": "emotion_cycle", "name": "情绪周期", "category": "sentiment", "description": "市场情绪从恐惧到贪婪的周期"}),
            value!({"id": "event_driven", "name": "事件驱动", "category": "fundamental", "description": "政策/财报/重大事件催化"}),
            value!({"id": "hot_theme", "name": "热点题材", "category": "momentum", "description": "追踪市场热点和题材轮动"}),
            value!({"id": "growth_quality", "name": "成长质量", "category": "fundamental", "description": "高ROE+低PB优质成长股"}),
            value!({"id": "dragon_head", "name": "龙头战法", "category": "momentum", "description": "板块龙头股的强者恒强"}),
            value!({"id": "expectation_repricing", "name": "预期重定价", "category": "fundamental", "description": "业绩预期上修带来的估值重定价"}),
            value!({"id": "one_yang_three_yin", "name": "一阳三阴", "category": "reversal", "description": "一根大阳线后三阴回调买入"}),
        ]
    }

    /// 根据市场上下文路由最佳策略
    pub fn route_strategy(context: &Value) -> Value {
        let trend = context
            .get("trend")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let change_pct = context
            .get("changePercent")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let volume_signal = context
            .get("volumeSignal")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let strategy = if trend.contains("up") && change_pct > 3.0 {
            "bull_trend"
        } else if trend.contains("up") && volume_signal.contains("shrink") {
            "shrink_pullback"
        } else if change_pct > 5.0 && volume_signal.contains("surge") {
            "volume_breakout"
        } else if trend.contains("down") && volume_signal.contains("surge") {
            "bottom_volume"
        } else if trend.contains("range") || trend.contains("sideway") {
            "box_oscillation"
        } else if change_pct > 7.0 {
            "dragon_head"
        } else {
            "growth_quality"
        };

        let strategies = Self::list_strategies();
        let matched = strategies
            .iter()
            .find(|s| s.get("id").and_then(|v| v.as_str()).unwrap_or_default() == strategy);

        value!({
            "recommendedStrategy": strategy,
            "strategyDetail": matched.cloned().unwrap_or(value!({})),
            "allStrategies": strategies,
        })
    }
}
