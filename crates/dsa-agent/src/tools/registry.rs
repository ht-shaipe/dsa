//! ToolRegistry - 动态工具注册与 OpenAI function-calling schema 生成

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use dsa_core::{DsaError, DsaResult};
use tube::{Map, Value};

/// 工具参数定义
#[derive(Debug, Clone)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

/// 工具处理器 (异步回调)
///
/// 注意: Future 未加 Send 约束, 因为部分底层 HTTP 客户端(awc/actix)
/// 使用 Rc 类型, 不满足 Send。使用时需在本地 task 中执行。
#[derive(Clone)]
pub struct ToolHandler {
    pub callback: fn(Value) -> Pin<Box<dyn Future<Output = DsaResult<Value>>>>,
}

impl std::fmt::Debug for ToolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolHandler")
            .field("callback", &"<fn>")
            .finish()
    }
}

/// 工具完整定义
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub handler: ToolHandler,
}

/// 工具注册中心
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// 创建空的注册中心
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// 注册一个工具
    pub fn register(
        &mut self,
        name: &str,
        description: &str,
        parameters: Vec<ToolParameter>,
        handler: fn(Value) -> Pin<Box<dyn Future<Output = DsaResult<Value>>>>,
    ) {
        self.tools.insert(
            name.to_string(),
            ToolDefinition {
                name: name.to_string(),
                description: description.to_string(),
                parameters,
                handler: ToolHandler { callback: handler },
            },
        );
    }

    /// 执行指定工具
    pub fn execute(&self, name: &str, params: Value) -> Pin<Box<dyn Future<Output = DsaResult<Value>>>> {
        let def = match self.tools.get(name) {
            Some(d) => d,
            None => {
                let err = DsaError::Validation(format!("工具不存在: {}", name));
                return Box::pin(async move { Err(err) });
            }
        };
        (def.handler.callback)(params)
    }

    /// 列出所有工具的基本信息
    pub fn list_tools(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|def| {
                let params: Vec<Value> = def
                    .parameters
                    .iter()
                    .map(|p| {
                        value!({
                            "name": p.name.clone(),
                            "type": p.param_type.clone(),
                            "description": p.description.clone(),
                            "required": p.required,
                            "defaultValue": p.default_value.clone(),
                        })
                    })
                    .collect();
                value!({
                    "name": def.name.clone(),
                    "description": def.description.clone(),
                    "parameters": params,
                })
            })
            .collect()
    }

    /// 生成 OpenAI function-calling 格式的工具 schema
    pub fn to_openai_tools(&self) -> Value {
        let tools: Vec<Value> = self
            .tools
            .values()
            .map(|def| {
                // Build properties object using Map
                let mut props_map = Map::new();
                for p in &def.parameters {
                    let entry = value!({
                        "type": p.param_type.clone(),
                        "description": p.description.clone(),
                    });
                    props_map.insert(&p.name, entry);
                }
                let properties = Value::Object(props_map);

                let required: Vec<Value> = def
                    .parameters
                    .iter()
                    .filter(|p| p.required)
                    .map(|p| Value::from(p.name.as_str()))
                    .collect();

                value!({
                    "type": "function",
                    "function": {
                        "name": def.name.clone(),
                        "description": def.description.clone(),
                        "parameters": {
                            "type": "object",
                            "properties": properties,
                            "required": required,
                        }
                    }
                })
            })
            .collect();

        Value::from(tools)
    }

    /// 返回所有工具名称
    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// 检查工具是否存在
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_handler(_params: Value) -> Pin<Box<dyn Future<Output = DsaResult<Value>>>> {
        Box::pin(async { Ok(value!({"ok": true})) })
    }

    #[test]
    fn test_register_and_has() {
        let mut reg = ToolRegistry::new();
        assert!(!reg.has("test_tool"));

        reg.register(
            "test_tool",
            "A test tool",
            vec![ToolParameter {
                name: "code".into(),
                param_type: "string".into(),
                description: "Stock code".into(),
                required: true,
                default_value: None,
            }],
            dummy_handler,
        );

        assert!(reg.has("test_tool"));
        assert_eq!(reg.names(), vec!["test_tool"]);
    }

    #[test]
    fn test_list_tools() {
        let mut reg = ToolRegistry::new();
        reg.register(
            "t1",
            "Tool one",
            vec![ToolParameter {
                name: "x".into(),
                param_type: "number".into(),
                description: "param x".into(),
                required: false,
                default_value: Some("0".into()),
            }],
            dummy_handler,
        );

        let list = reg.list_tools();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].get("name").and_then(|v| v.as_str()), Some("t1".to_string()));
    }

    #[test]
    fn test_to_openai_tools() {
        let mut reg = ToolRegistry::new();
        reg.register(
            "get_chip_concentration",
            "Analyze chip concentration",
            vec![
                ToolParameter {
                    name: "code".into(),
                    param_type: "string".into(),
                    description: "Stock code".into(),
                    required: true,
                    default_value: None,
                },
            ],
            dummy_handler,
        );

        let schema = reg.to_openai_tools();
        let tools = schema.as_array().unwrap();
        assert_eq!(tools.len(), 1);

        let func = &tools[0];
        assert_eq!(
            func.get("type").and_then(|v| v.as_str()),
            Some("function".to_string())
        );
    }

    #[tokio::test]
    async fn test_execute() {
        let mut reg = ToolRegistry::new();
        reg.register("echo", "Echo tool", vec![], dummy_handler);

        let result = reg.execute("echo", value!({})).await.unwrap();
        assert_eq!(result.get("ok").and_then(|v| v.as_bool()), Some(true));
    }

    #[tokio::test]
    async fn test_execute_not_found() {
        let reg = ToolRegistry::new();
        let result = reg.execute("missing", value!({})).await;
        assert!(result.is_err());
    }
}
