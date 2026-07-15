//! 对话上下文管理 - 支持持久化和序列化

use tube::Value;
use dsa_core::{DsaError, DsaResult, utils};
use ai_llm_kit::{LlmFactory, LlmProvider, LlmService};
use deck_mysql::{DataRow, Helper};

/// 单条消息
pub struct Message {
    pub role: String,
    pub content: String,
}

/// 对话上下文，支持持久化
pub struct Conversation {
    pub session_id: String,
    pub messages: Vec<Message>,
}

impl Conversation {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            messages: Vec::new(),
        }
    }

    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }

    pub fn add_user(&mut self, content: &str) {
        self.messages.push(Message { role: "user".to_string(), content: content.to_string() });
    }

    pub fn add_assistant(&mut self, content: &str) {
        self.messages.push(Message { role: "assistant".to_string(), content: content.to_string() });
    }

    pub fn add_system(&mut self, content: &str) {
        self.messages.push(Message { role: "system".to_string(), content: content.to_string() });
    }

    /// 序列化为 Value
    pub fn to_value(&self) -> Value {
        let msgs: Vec<Value> = self.messages.iter().map(|m| {
            value!({
                "role": m.role.clone(),
                "content": m.content.clone(),
            })
        }).collect();

        value!({
            "session_id": self.session_id.clone(),
            "messages": msgs,
        })
    }

    /// 从 Value 反序列化
    pub fn from_value(session_id: &str, v: &Value) -> Self {
        let msgs = v.get("messages")
            .and_then(|m| Value::as_array(m))
            .unwrap_or_default();

        let messages: Vec<Message> = msgs.iter().map(|item| {
            let role = item.get("role")
                .and_then(|r| r.as_str())
                .unwrap_or_default();
            let content = item.get("content")
                .and_then(|c| c.as_str())
                .unwrap_or_default();
            Message { role, content }
        }).collect();

        Self {
            session_id: session_id.to_string(),
            messages,
        }
    }

    /// 压缩对话摘要 - 当消息超过20条时, 调用LLM压缩为摘要并保存
    pub async fn compress_summary(&self, session_id: &str) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;

        // 读取该session的所有消息
        let sql = "SELECT role, content FROM conversation_messages \
              WHERE session_id = :sid ORDER BY create_time ASC";
        let rows = Helper::query_rows(
            sql,
            vec![("sid".to_string(), Value::from(session_id.to_string()))],
            &connector,
        ).map_err(|e| DsaError::Database(format!("查询对话消息失败: {}", e)))?;

        if rows.len() <= 20 {
            return Ok(value!({
                "status": "ok",
                "data": {
                    "compressed": false,
                    "reason": "消息数未超过20条, 无需压缩",
                    "messageCount": rows.len() as i64,
                }
            }));
        }

        // 构建对话文本
        let conversation_text: String = rows.iter().map(|r| {
            let v = r.to_value2();
            let role = v.get("role").and_then(|v| v.as_str()).unwrap_or_default();
            let content = v.get("content").and_then(|v| v.as_str()).unwrap_or_default();
            format!("[{}]: {}", role, content)
        }).collect::<Vec<String>>().join("\n");

        // 调用LLM生成摘要
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Err(DsaError::LlmAnalysis("API Key 未配置, 无法压缩摘要".to_string()));
        }

        let llm_provider = LlmProvider::instance(&conf.llm.provider)
            .map_err(|e| DsaError::LlmAnalysis(format!("创建LLM Provider失败: {}", e)))?;
        let llm: Box<dyn LlmService> = LlmFactory::create(llm_provider, &api_key);

        let prompt = format!(
            "请将以下对话历史压缩为一段简洁的摘要, 保留关键信息和决策要点:\n\n{}",
            conversation_text
        );

        let body = value!({
            "model": &conf.llm.model,
            "messages": [
                {"role": "system", "content": "你是一个对话摘要助手, 负责将冗长的对话历史压缩为简洁摘要, 保留关键信息和决策要点。"},
                {"role": "user", "content": prompt}
            ],
            "max_tokens": 500,
        });

        let response = llm.chat(&body).await
            .map_err(|e| DsaError::LlmAnalysis(format!("LLM摘要生成失败: {}", e)))?;

        let choices = response
            .get("choices")
            .and_then(|c| tube::Value::as_array(&c.clone()))
            .unwrap_or_default();
        let summary = choices
            .first()
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        // 保存摘要到 conversation_summaries 表
        let insert_sql = "INSERT INTO conversation_summaries \
             (sessionId, summary, originalMessageCount, createTime) \
             VALUES (:sid, :summary, :count, NOW())";
        let _ = dsa_core::db::execute(
            insert_sql,
            vec![
                ("sid".to_string(), Value::from(session_id.to_string())),
                ("summary".to_string(), Value::from(summary.as_str())),
                ("count".to_string(), Value::from(rows.len() as i64)),
            ],
            &connector,
        );

        Ok(value!({
            "status": "ok",
            "data": {
                "compressed": true,
                "summary": summary,
                "originalMessageCount": rows.len() as i64,
            }
        }))
    }
}
