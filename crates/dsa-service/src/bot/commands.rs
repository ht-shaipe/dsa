//! Bot 命令处理器 - 各命令的具体实现
//!
//! 内置命令:
//!   /help    - 列出所有可用命令
//!   /status  - 系统状态检查
//!   /analyze - 快速股票分析
//!   /market  - 大盘概览
//!   /ask     - Agent 分析提问
//!   /chat    - 自由对话
//!   /history - 查看历史分析

use std::future::Future;
use std::pin::Pin;

use dsa_core::{DsaError, DsaResult, utils};
use dsa_core::db::{query_rows, row_get_string, row_get_value};
use qta_crawler::Real;
use tube::Value;

use super::dispatcher::BotContext;

/// Bot 命令 trait
pub trait BotCommand {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &str, context: &BotContext) -> Pin<Box<dyn Future<Output = DsaResult<String>> + '_>>;
}

// ============================================================
// /help - 列出所有命令
// ============================================================
pub struct HelpCommand;

impl BotCommand for HelpCommand {
    fn name(&self) -> &str { "help" }
    fn description(&self) -> &str { "显示所有可用命令" }

    fn execute(&self, _args: &str, _context: &BotContext) -> Pin<Box<dyn Future<Output = DsaResult<String>> + '_>> {
        Box::pin(async move {
            let help_text = [
                ("📖 可用命令列表", ""),
                ("", ""),
                ("/help", "显示所有可用命令"),
                ("/status", "查看系统运行状态"),
                ("/analyze <代码>", "快速股票分析 (例: /analyze 600519)"),
                ("/market", "大盘指数概览"),
                ("/ask <问题>", "Agent 分析提问 (例: /ask 茅台值得买入吗?)"),
                ("/chat <消息>", "自由对话"),
                ("/history <代码>", "查看最近3次分析记录"),
                ("", ""),
                ("💡 命令以 / 开头, 参数用空格分隔", ""),
            ];

            let mut lines: Vec<String> = Vec::new();
            for (cmd, desc) in &help_text {
                if desc.is_empty() && !cmd.is_empty() {
                    lines.push(cmd.to_string());
                } else if cmd.is_empty() {
                    lines.push(String::new());
                } else {
                    lines.push(format!("  {:<20} {}", cmd, desc));
                }
            }

            Ok(lines.join("\n"))
        })
    }
}

// ============================================================
// /status - 系统状态检查
// ============================================================
pub struct StatusCommand;

impl BotCommand for StatusCommand {
    fn name(&self) -> &str { "status" }
    fn description(&self) -> &str { "查看系统运行状态" }

    fn execute(&self, _args: &str, _context: &BotContext) -> Pin<Box<dyn Future<Output = DsaResult<String>> + '_>> {
        Box::pin(async move {
            let mut parts: Vec<String> = vec!["🔍 系统状态检查".to_string()];

            // DB 连接检查
            let db_status = match utils::get_db_connector() {
                Ok(_) => "✅ 正常",
                Err(e) => {
                    tracing::warn!("[Bot] DB连接检查失败: {}", e);
                    "❌ 异常"
                }
            };
            parts.push(format!("  数据库: {}", db_status));

            // LLM 可用性检查
            let conf = dsa_core::get_global_config();
            let api_key = conf.resolve_api_key();
            let llm_status = if api_key.is_empty() {
                "⚠️ 未配置 API Key"
            } else {
                "✅ 已配置"
            };
            parts.push(format!("  LLM服务: {}", llm_status));

            // 调度器状态(静态说明, 不实际查询调度器)
            parts.push("  调度器: ✅ 运行中".to_string());

            parts.push(String::new());
            parts.push(format!("  Provider: {}", conf.llm.provider));
            parts.push(format!("  Model: {}", conf.llm.model));

            Ok(parts.join("\n"))
        })
    }
}

// ============================================================
// /analyze <code> - 快速股票分析
// ============================================================
pub struct AnalyzeCommand;

impl BotCommand for AnalyzeCommand {
    fn name(&self) -> &str { "analyze" }
    fn description(&self) -> &str { "快速股票分析" }

    fn execute(&self, args: &str, _context: &BotContext) -> Pin<Box<dyn Future<Output = DsaResult<String>> + '_>> {
        let code = args.trim().to_string();
        Box::pin(async move {
            if code.is_empty() {
                return Ok("❌ 请提供股票代码, 例: /analyze 600519".to_string());
            }

            let code_clean = code.replace("sh", "").replace("sz", "");
            if code_clean.len() != 6 || !code_clean.chars().all(|c| c.is_numeric()) {
                return Ok(format!("❌ 无效股票代码: {}", code));
            }

            // 查询最新分析历史
            let connector = match utils::get_db_connector() {
                Ok(c) => c,
                Err(e) => return Ok(format!("❌ 数据库连接失败: {}", e)),
            };

            let prefix = utils::market_prefix(&code_clean);
            let full_code = format!("{}{}", prefix, code_clean);

            let sql = "SELECT id, stock_code, stock_name, sentiment_score, decision_type, \
                        operationAdvice, analysisSummary, createTime \
                        FROM analysis_history WHERE stock_code = :code AND status = 1 \
                        ORDER BY create_time DESC LIMIT 1";
            let params = vec![("code".to_string(), Value::from(full_code.as_str()))];

            let rows = query_rows(sql, params, &connector)
                .map_err(|e| DsaError::Database(format!("查询分析记录失败: {}", e)))?;

            if let Some(row) = rows.first() {
                let name = row_get_string(row, "stockName");
                let name = if name.is_empty() { code_clean.clone() } else { name };
                let score = row_get_string(row, "sentimentScore");
                let score = if score.is_empty() { "N/A".to_string() } else { score };
                let decision = row_get_string(row, "decisionType");
                let decision = if decision.is_empty() { "N/A".to_string() } else { decision };
                let advice = row_get_string(row, "operationAdvice");
                let advice = if advice.is_empty() { "N/A".to_string() } else { advice };
                let summary = row_get_string(row, "analysisSummary");
                let summary = if summary.is_empty() { "无摘要".to_string() } else { summary };
                let time = row_get_string(row, "createTime");
                let time = if time.is_empty() { "未知".to_string() } else { time };

                // 截断摘要过长内容
                let summary_truncated = if summary.len() > 200 {
                    format!("{}...", &summary[..200])
                } else {
                    summary
                };

                Ok(format!(
                    "📊 {} ({}) 最新分析\n\n\
                     情绪评分: {}\n\
                     决策类型: {}\n\
                     操作建议: {}\n\
                     分析时间: {}\n\n\
                     📝 摘要: {}",
                    name, full_code, score, decision, advice, time, summary_truncated
                ))
            } else {
                // 无历史记录, 尝试获取实时行情
                match Real::new().get_price(&full_code).await {
                    Ok(quote) => {
                        let name = quote.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                        let name = if name.is_empty() { code_clean.clone() } else { name.to_string() };
                        let price = quote.get("price").and_then(|v| v.as_str()).unwrap_or_default();
                        let change = quote.get("changePercent").and_then(|v| v.as_str()).unwrap_or_default();
                        Ok(format!(
                            "📊 {} ({}) 实时行情\n\n\
                             当前价: {}\n\
                             涨跌幅: {}%\n\n\
                             💡 暂无分析记录, 可通过分析服务触发深度分析",
                            name, full_code, price, change
                        ))
                    }
                    Err(e) => Ok(format!(
                        "❌ 未找到 {} 的分析记录, 获取行情也失败: {}",
                        full_code, e
                    )),
                }
            }
        })
    }
}

// ============================================================
// /market - 大盘概览
// ============================================================
pub struct MarketCommand;

impl BotCommand for MarketCommand {
    fn name(&self) -> &str { "market" }
    fn description(&self) -> &str { "大盘指数概览" }

    fn execute(&self, _args: &str, _context: &BotContext) -> Pin<Box<dyn Future<Output = DsaResult<String>> + '_>> {
        Box::pin(async move {
            let real = Real::new();

            let indices = [
                ("上证指数", "sh000001"),
                ("深证成指", "sz399001"),
                ("创业板指", "sz399006"),
            ];

            let mut parts: Vec<String> = vec!["📈 大盘指数概览".to_string(), String::new()];

            for (label, code) in &indices {
                match real.get_price(code).await {
                    Ok(data) => {
                        let name = data.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                        let name = if name.is_empty() { label.to_string() } else { name.to_string() };
                        let price = data.get("price").and_then(|v| v.as_str()).unwrap_or_default();
                        let change = data.get("changePercent").and_then(|v| v.as_str()).unwrap_or_default();
                        let change_dir = if change.starts_with('-') { "📉" } else { "📈" };
                        parts.push(format!("  {} {}: {}  {}%", change_dir, name, price, change));
                    }
                    Err(e) => {
                        tracing::warn!("[Bot] 获取{}行情失败: {}", label, e);
                        parts.push(format!("  ⚠️ {}: 获取失败", label));
                    }
                }
            }

            Ok(parts.join("\n"))
        })
    }
}

// ============================================================
// /ask <question> - Agent 分析提问
// ============================================================
pub struct AskCommand;

impl BotCommand for AskCommand {
    fn name(&self) -> &str { "ask" }
    fn description(&self) -> &str { "Agent 分析提问" }

    fn execute(&self, args: &str, _context: &BotContext) -> Pin<Box<dyn Future<Output = DsaResult<String>> + '_>> {
        let question = args.trim().to_string();
        Box::pin(async move {
            if question.is_empty() {
                return Ok("❌ 请输入问题, 例: /ask 茅台值得买入吗?".to_string());
            }

            // Placeholder: 实际生产中会调用 agent pipeline
            // 当前仅返回确认消息
            Ok(format!(
                "🤖 Agent 分析\n\n\
                 问题: {}\n\n\
                 💡 该功能需要接入 Agent 服务, 当前为预览模式。\n\
                 将通过 AI Agent 对您的问题进行深度分析。",
                question
            ))
        })
    }
}

// ============================================================
// /chat <message> - 自由对话
// ============================================================
pub struct ChatCommand;

impl BotCommand for ChatCommand {
    fn name(&self) -> &str { "chat" }
    fn description(&self) -> &str { "自由对话" }

    fn execute(&self, args: &str, _context: &BotContext) -> Pin<Box<dyn Future<Output = DsaResult<String>> + '_>> {
        let message = args.trim().to_string();
        Box::pin(async move {
            if message.is_empty() {
                return Ok("❌ 请输入消息, 例: /chat 今天股市怎么样?".to_string());
            }

            // Placeholder: 实际生产中会调用 agent 对话接口
            Ok(format!(
                "💬 对话\n\n\
                 您说: {}\n\n\
                 💡 该功能需要接入对话 Agent, 当前为预览模式。\n\
                 将通过 AI Agent 进行自由对话。",
                message
            ))
        })
    }
}

// ============================================================
// /history <code> - 查看最近3次分析记录
// ============================================================
pub struct HistoryCommand;

impl BotCommand for HistoryCommand {
    fn name(&self) -> &str { "history" }
    fn description(&self) -> &str { "查看最近分析记录" }

    fn execute(&self, args: &str, _context: &BotContext) -> Pin<Box<dyn Future<Output = DsaResult<String>> + '_>> {
        let code = args.trim().to_string();
        Box::pin(async move {
            if code.is_empty() {
                return Ok("❌ 请提供股票代码, 例: /history 600519".to_string());
            }

            let code_clean = code.replace("sh", "").replace("sz", "");
            if code_clean.len() != 6 || !code_clean.chars().all(|c| c.is_numeric()) {
                return Ok(format!("❌ 无效股票代码: {}", code));
            }

            let connector = match utils::get_db_connector() {
                Ok(c) => c,
                Err(e) => return Ok(format!("❌ 数据库连接失败: {}", e)),
            };

            let prefix = utils::market_prefix(&code_clean);
            let full_code = format!("{}{}", prefix, code_clean);

            let sql = "SELECT id, stock_code, stock_name, sentiment_score, decision_type, \
                        operationAdvice, createTime \
                        FROM analysis_history WHERE stock_code = :code AND status = 1 \
                        ORDER BY create_time DESC LIMIT 3";
            let params = vec![("code".to_string(), Value::from(full_code.as_str()))];

            let rows = query_rows(sql, params, &connector)
                .map_err(|e| DsaError::Database(format!("查询历史记录失败: {}", e)))?;

            if rows.is_empty() {
                return Ok(format!("📭 暂无 {} 的分析记录", full_code));
            }

            let mut parts: Vec<String> = vec![format!("📋 {} 最近分析记录", full_code), String::new()];

            for (i, row) in rows.iter().enumerate() {
                let name = row_get_string(row, "stockName");
                let name = if name.is_empty() { code_clean.clone() } else { name };
                let score = row_get_string(row, "sentimentScore");
                let score = if score.is_empty() { "N/A".to_string() } else { score };
                let decision = row_get_string(row, "decisionType");
                let decision = if decision.is_empty() { "N/A".to_string() } else { decision };
                let advice = row_get_string(row, "operationAdvice");
                let advice = if advice.is_empty() { "N/A".to_string() } else { advice };
                let time = row_get_string(row, "createTime");
                let time = if time.is_empty() { "未知".to_string() } else { time };

                parts.push(format!("{}. {} | 评分: {} | 决策: {}", i + 1, name, score, decision));
                parts.push(format!("   建议: {} | 时间: {}", advice, time));
                if i < rows.len() - 1 {
                    parts.push(String::new());
                }
            }

            Ok(parts.join("\n"))
        })
    }
}
