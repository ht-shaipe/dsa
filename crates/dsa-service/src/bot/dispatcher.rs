//! Bot 命令分发器 - 解析消息、路由命令、速率限制

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use dsa_core::DsaResult;
use tube::Value;

use super::commands::{
    AnalyzeCommand, AskCommand, BotCommand, ChatCommand, HelpCommand, HistoryCommand,
    MarketCommand, StatusCommand,
};

/// Bot 上下文 - 携带消息来源平台和用户信息
#[derive(Debug, Clone)]
pub struct BotContext {
    pub platform: String,
    pub user_id: String,
    pub chat_id: String,
    pub is_admin: bool,
}

/// Bot 命令分发器
pub struct BotDispatcher {
    commands: HashMap<String, Box<dyn BotCommand + Send + Sync>>,
    rate_limits: Mutex<HashMap<String, u64>>,
}

/// 速率限制窗口(秒) - 同一用户两次命令最小间隔
const RATE_LIMIT_WINDOW_SECS: u64 = 2;

impl BotDispatcher {
    /// 创建分发器并注册所有内置命令
    pub fn new() -> Self {
        let mut commands: HashMap<String, Box<dyn BotCommand + Send + Sync>> = HashMap::new();

        let builtins: Vec<Box<dyn BotCommand + Send + Sync>> = vec![
            Box::new(HelpCommand),
            Box::new(StatusCommand),
            Box::new(AnalyzeCommand),
            Box::new(MarketCommand),
            Box::new(AskCommand),
            Box::new(ChatCommand),
            Box::new(HistoryCommand),
        ];

        for cmd in builtins {
            commands.insert(cmd.name().to_string(), cmd);
        }

        Self {
            commands,
            rate_limits: Mutex::new(HashMap::new()),
        }
    }

    /// 分发消息: 解析命令前缀, 路由到对应处理器
    pub async fn dispatch(&self, raw_message: &str, context: &BotContext) -> DsaResult<String> {
        let trimmed = raw_message.trim();

        // 必须以 "/" 开头才是命令
        if !trimmed.starts_with('/') {
            return Ok(Self::nl_fallback_message());
        }

        // 解析命令名和参数: "/analyze 600519" -> ("analyze", "600519")
        let (cmd_name, args) = Self::parse_command(trimmed);

        // 速率限制检查
        if !self.check_rate_limit(&context.user_id) {
            return Ok("⏳ 操作过快，请稍后再试".to_string());
        }

        // 查找并执行命令
        match self.commands.get(cmd_name) {
            Some(handler) => handler.execute(args, context).await,
            None => Ok(format!(
                "❓ 未知命令: /{}\n\n{}",
                cmd_name,
                Self::nl_fallback_message()
            )),
        }
    }

    /// 列出所有已注册命令(name + description)
    pub fn list_commands(&self) -> Vec<Value> {
        self.commands
            .values()
            .map(|cmd| {
                value!({
                    "name": cmd.name(),
                    "description": cmd.description(),
                })
            })
            .collect()
    }

    /// 速率限制检查 - 返回 true 表示允许通过
    pub fn check_rate_limit(&self, user_id: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut limits = self.rate_limits.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(last_time) = limits.get(user_id) {
            if now.saturating_sub(*last_time) < RATE_LIMIT_WINDOW_SECS {
                return false;
            }
        }

        limits.insert(user_id.to_string(), now);
        true
    }

    /// 解析命令字符串
    fn parse_command(input: &str) -> (&str, &str) {
        let without_slash = &input[1..]; // 去掉开头的 "/"
        let mut parts = without_slash.splitn(2, char::is_whitespace);
        let cmd = parts.next().unwrap_or("");
        let args = parts.next().unwrap_or("").trim();
        (cmd, args)
    }

    /// 自然语言回退提示
    fn nl_fallback_message() -> String {
        "💡 输入 /help 查看可用命令列表".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        assert_eq!(
            BotDispatcher::parse_command("/analyze 600519"),
            ("analyze", "600519")
        );
        assert_eq!(BotDispatcher::parse_command("/help"), ("help", ""));
        assert_eq!(BotDispatcher::parse_command("/market  "), ("market", ""));
        assert_eq!(
            BotDispatcher::parse_command("/ask how is AAPL?"),
            ("ask", "how is AAPL?")
        );
    }

    #[test]
    fn test_nl_fallback() {
        let msg = BotDispatcher::nl_fallback_message();
        assert!(msg.contains("/help"));
    }

    #[tokio::test]
    async fn test_dispatch_unknown_command() {
        let dispatcher = BotDispatcher::new();
        let ctx = BotContext {
            platform: "test".to_string(),
            user_id: "u1".to_string(),
            chat_id: "c1".to_string(),
            is_admin: false,
        };
        let result = dispatcher.dispatch("/unknown", &ctx).await.unwrap();
        assert!(result.contains("未知命令"));
    }

    #[tokio::test]
    async fn test_dispatch_non_command() {
        let dispatcher = BotDispatcher::new();
        let ctx = BotContext {
            platform: "test".to_string(),
            user_id: "u1".to_string(),
            chat_id: "c1".to_string(),
            is_admin: false,
        };
        let result = dispatcher.dispatch("hello world", &ctx).await.unwrap();
        assert!(result.contains("/help"));
    }

    #[test]
    fn test_list_commands() {
        let dispatcher = BotDispatcher::new();
        let cmds = dispatcher.list_commands();
        assert!(!cmds.is_empty());
        let names: Vec<String> = cmds
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"help".to_string()));
        assert!(names.contains(&"analyze".to_string()));
        assert!(names.contains(&"market".to_string()));
    }

    #[test]
    fn test_rate_limit() {
        let dispatcher = BotDispatcher::new();
        assert!(dispatcher.check_rate_limit("user1"));
        // Second call within window should be rejected
        assert!(!dispatcher.check_rate_limit("user1"));
        // Different user should pass
        assert!(dispatcher.check_rate_limit("user2"));
    }
}
