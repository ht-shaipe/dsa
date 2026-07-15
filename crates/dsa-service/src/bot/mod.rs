//! Bot 平台服务 - 命令分发与平台适配
//!
//! 支持: 钉钉、飞书、Discord 等平台接入
//! 内置命令: /help, /status, /analyze, /market, /ask, /chat, /history

pub mod commands;
pub mod dispatcher;
pub mod platforms;
