//! 技能路由 - 策略技能评估和路由

use tube::Value;

/// 技能 trait
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn evaluate(&self, context: &Value) -> Value;
}

/// 技能路由器
pub struct SkillRouter {
    pub skills: Vec<Box<dyn Skill>>,
    threshold: u32,
}

impl SkillRouter {
    pub fn new(skills: Vec<Box<dyn Skill>>) -> Self {
        Self { skills, threshold: 40 }
    }

    /// 设置阈值
    pub fn with_threshold(mut self, threshold: u32) -> Self {
        self.threshold = threshold;
        self
    }

    /// 路由：运行所有技能，返回强度超过阈值的结果
    pub fn route(&self, context: &Value) -> Vec<Value> {
        self.skills.iter()
            .map(|skill| skill.evaluate(context))
            .filter(|result| {
                result.get("strength")
                    .and_then(|s| s.as_u64())
                    .unwrap_or(0) as u32 >= self.threshold
            })
            .collect()
    }

    /// 运行所有技能，不管阈值
    pub fn evaluate_all(&self, context: &Value) -> Vec<Value> {
        self.skills.iter()
            .map(|skill| skill.evaluate(context))
            .collect()
    }
}
