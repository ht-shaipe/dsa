//! 批量注册所有工具到 ToolRegistry

use super::registry::ToolRegistry;
use super::{chip_tools, history_tools, pattern_tools, portfolio_tools};

/// 创建 ToolRegistry 并注册所有内置工具
pub fn register_all() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    chip_tools::register(&mut registry);
    portfolio_tools::register(&mut registry);
    history_tools::register(&mut registry);
    pattern_tools::register(&mut registry);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_all() {
        let registry = register_all();

        // Should have 5 tools: chip(1) + portfolio(2) + history(1) + pattern(1)
        assert!(registry.has("get_chip_concentration"));
        assert!(registry.has("get_portfolio_snapshot"));
        assert!(registry.has("get_capital_flow"));
        assert!(registry.has("get_analysis_context"));
        assert!(registry.has("detect_patterns"));

        let names = registry.names();
        assert_eq!(names.len(), 5);
    }

    #[test]
    fn test_openai_schema_generation() {
        let registry = register_all();
        let schema = registry.to_openai_tools();
        let tools = schema.as_array().expect("should be array");
        assert_eq!(tools.len(), 5);

        // Each tool should have type:"function"
        for tool in tools {
            assert_eq!(
                tool.get("type").and_then(|v| v.as_str()),
                Some("function".to_string())
            );
            assert!(tool.get("function").is_some());
        }
    }
}
