//! Bot 平台适配器 - 各平台的消息收发与格式转换
//!
//! 支持: 钉钉(DingTalk)、飞书(Feishu)、Discord

use std::future::Future;
use std::pin::Pin;

use dsa_core::{DsaError, DsaResult};
use tube::Value;

use super::dispatcher::BotContext;

/// 平台消息统一模型
#[derive(Debug, Clone)]
pub struct BotMessage {
    pub text: String,
    pub user_id: String,
    pub chat_id: String,
    pub platform: String,
    pub is_admin: bool,
}

/// 平台适配器 trait
///
/// send_response 使用 Pin<Box<dyn Future>> 以支持 async 在 trait 中(edition 2021)
pub trait BotPlatform: Send + Sync {
    /// 平台名称
    fn name(&self) -> &str;

    /// 验证请求合法性(签名/Token检查)
    fn verify_request(&self, headers: &Value, body: &Value) -> DsaResult<bool>;

    /// 从请求体解析出统一消息
    fn parse_message(&self, body: &Value) -> DsaResult<BotMessage>;

    /// 将响应文本格式化为平台特定的 JSON
    fn format_response(&self, text: &str, context: &BotContext) -> Value;

    /// 发送响应到平台(异步)
    fn send_response<'a>(&'a self, response: &'a Value) -> Pin<Box<dyn Future<Output = DsaResult<()>> + Send + 'a>>;
}

// ============================================================
// 钉钉适配器
// ============================================================

/// 钉钉平台适配器
///
/// - 出站: 通过 webhook_url POST markdown 消息
/// - 入站: 验证 timestamp + sign 签名
pub struct DingTalkAdapter {
    pub webhook_url: String,
    pub app_secret: String,
    client: reqwest::Client,
}

impl DingTalkAdapter {
    pub fn new(webhook_url: &str, app_secret: &str) -> Self {
        Self {
            webhook_url: webhook_url.to_string(),
            app_secret: app_secret.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// 计算钉钉签名
    ///
    /// 简化实现: 使用 app_secret + timestamp 的 FNV-1a 哈希。
    /// 生产环境应替换为 HMAC-SHA256 (需引入 hmac+sha2 crate)。
    fn compute_sign(&self, timestamp: &str) -> String {
        let message = format!("{}\n{}", timestamp, self.app_secret);
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in message.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:x}", hash)
    }
}

impl BotPlatform for DingTalkAdapter {
    fn name(&self) -> &str { "dingtalk" }

    fn verify_request(&self, headers: &Value, _body: &Value) -> DsaResult<bool> {
        // 钉钉回调验证: 检查 timestamp + sign 头
        let timestamp = headers
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let sign = headers
            .get("sign")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if timestamp.is_empty() || sign.is_empty() {
            return Ok(false);
        }

        let expected_sign = self.compute_sign(&timestamp);
        Ok(expected_sign == sign)
    }

    fn parse_message(&self, body: &Value) -> DsaResult<BotMessage> {
        // 钉钉消息格式:
        // { "msgtype": "text", "text": { "content": "消息内容" },
        //   "senderStaffId": "user123", "conversationId": "cid", "isAdmin": true }
        let text = body
            .get("text")
            .and_then(|t| t.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let user_id = body
            .get("senderStaffId")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "unknown".to_string());

        let chat_id = body
            .get("conversationId")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "unknown".to_string());

        let is_admin = body
            .get("isAdmin")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(BotMessage {
            text,
            user_id,
            chat_id,
            platform: "dingtalk".to_string(),
            is_admin,
        })
    }

    fn format_response(&self, text: &str, _context: &BotContext) -> Value {
        // 钉钉 Markdown 格式
        value!({
            "msgtype": "markdown",
            "markdown": {
                "title": "DSA Bot",
                "text": text,
            }
        })
    }

    fn send_response<'a>(&'a self, response: &'a Value) -> Pin<Box<dyn Future<Output = DsaResult<()>> + Send + 'a>> {
        Box::pin(async move {
            let body = serde_json::to_string(response)
                .map_err(|e| DsaError::Internal(format!("序列化钉钉响应失败: {}", e)))?;

            let resp = self.client
                .post(&self.webhook_url)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await
                .map_err(|e| DsaError::Internal(format!("钉钉消息发送失败: {}", e)))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                tracing::warn!("[DingTalk] 发送失败 status={}, body={}", status, text);
                return Err(DsaError::Internal(format!(
                    "钉钉消息发送失败: HTTP {}", status
                )));
            }

            Ok(())
        })
    }
}

// ============================================================
// 飞书适配器
// ============================================================

/// 飞书平台适配器
///
/// - 出站: 通过 webhook_url POST interactive card 消息
/// - 入站: 验证 verification_token
pub struct FeishuAdapter {
    pub webhook_url: String,
    pub app_id: String,
    pub app_secret: String,
    pub verification_token: String,
    client: reqwest::Client,
}

impl FeishuAdapter {
    pub fn new(webhook_url: &str, app_id: &str, app_secret: &str, verification_token: &str) -> Self {
        Self {
            webhook_url: webhook_url.to_string(),
            app_id: app_id.to_string(),
            app_secret: app_secret.to_string(),
            verification_token: verification_token.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl BotPlatform for FeishuAdapter {
    fn name(&self) -> &str { "feishu" }

    fn verify_request(&self, headers: &Value, _body: &Value) -> DsaResult<bool> {
        // 飞书验证: 检查 verification_token
        let token = headers
            .get("X-Lark-Signature")
            .or_else(|| headers.get("verification_token"))
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if self.verification_token.is_empty() {
            // 未配置 token 则跳过验证
            return Ok(true);
        }

        Ok(token == self.verification_token)
    }

    fn parse_message(&self, body: &Value) -> DsaResult<BotMessage> {
        // 飞书事件格式:
        // { "event": { "message": { "content": "{\"text\":\"消息\"}" },
        //   "sender": { "sender_id": { "user_id": "u1" } } },
        //   "header": { ... } }
        let event = body.get("event").cloned().unwrap_or(Value::Null);

        // 飞书消息 content 是 JSON 字符串, 需要二次解析
        let content_str = event
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let content: Value = serde_json::from_str(&content_str).unwrap_or(Value::Null);
        let text = content
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let user_id = event
            .get("sender")
            .and_then(|s| s.get("sender_id"))
            .and_then(|sid| sid.get("user_id"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "unknown".to_string());

        let chat_id = event
            .get("message")
            .and_then(|m| m.get("chat_id"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(BotMessage {
            text,
            user_id,
            chat_id,
            platform: "feishu".to_string(),
            is_admin: false, // 飞书需要额外 API 查询角色, 简化处理
        })
    }

    fn format_response(&self, text: &str, _context: &BotContext) -> Value {
        // 飞书 Interactive Card 格式
        value!({
            "msg_type": "interactive",
            "card": {
                "elements": [
                    {
                        "tag": "markdown",
                        "content": text,
                    }
                ],
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": "DSA Bot",
                    }
                }
            }
        })
    }

    fn send_response<'a>(&'a self, response: &'a Value) -> Pin<Box<dyn Future<Output = DsaResult<()>> + Send + 'a>> {
        Box::pin(async move {
            let body = serde_json::to_string(response)
                .map_err(|e| DsaError::Internal(format!("序列化飞书响应失败: {}", e)))?;

            let resp = self.client
                .post(&self.webhook_url)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await
                .map_err(|e| DsaError::Internal(format!("飞书消息发送失败: {}", e)))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                tracing::warn!("[Feishu] 发送失败 status={}, body={}", status, text);
                return Err(DsaError::Internal(format!(
                    "飞书消息发送失败: HTTP {}", status
                )));
            }

            Ok(())
        })
    }
}

// ============================================================
// Discord 适配器
// ============================================================

/// Discord 平台适配器
///
/// - 出站: 通过 webhook_url POST 消息
/// - 入站: 验证 Ed25519 签名(简化: 检查必要头是否存在)
pub struct DiscordAdapter {
    pub webhook_url: String,
    pub bot_token: String,
    pub public_key: String,
    client: reqwest::Client,
}

impl DiscordAdapter {
    pub fn new(webhook_url: &str, bot_token: &str, public_key: &str) -> Self {
        Self {
            webhook_url: webhook_url.to_string(),
            bot_token: bot_token.to_string(),
            public_key: public_key.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl BotPlatform for DiscordAdapter {
    fn name(&self) -> &str { "discord" }

    fn verify_request(&self, headers: &Value, _body: &Value) -> DsaResult<bool> {
        // Discord 使用 Ed25519 签名验证
        // 简化实现: 检查 X-Signature-Ed25519 和 X-Signature-Timestamp 头是否存在
        let has_sig = headers
            .get("X-Signature-Ed25519")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        let has_ts = headers
            .get("X-Signature-Timestamp")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        // 如果都存在, 认为请求合法(完整实现需 Ed25519 验签)
        Ok(has_sig && has_ts)
    }

    fn parse_message(&self, body: &Value) -> DsaResult<BotMessage> {
        // Discord Interaction 格式:
        // { "type": 1, "data": { "content": "消息内容" },
        //   "member": { "user": { "id": "uid" } },
        //   "channel_id": "cid" }
        let text = body
            .get("data")
            .and_then(|d| d.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let user_id = body
            .get("member")
            .and_then(|m| m.get("user"))
            .and_then(|u| u.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "unknown".to_string());

        let chat_id = body
            .get("channel_id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "unknown".to_string());

        // Discord 管理员判断需要权限位检查, 简化处理
        let is_admin = body
            .get("member")
            .and_then(|m| m.get("permissions"))
            .and_then(|v| v.as_str())
            .map(|p| {
                // 管理员权限位 0x8 (ADMINISTRATOR)
                u64::from_str_radix(&p, 16).map(|bits| bits & 0x8 != 0).unwrap_or(false)
            })
            .unwrap_or(false);

        Ok(BotMessage {
            text,
            user_id,
            chat_id,
            platform: "discord".to_string(),
            is_admin,
        })
    }

    fn format_response(&self, text: &str, _context: &BotContext) -> Value {
        // Discord Interaction Response 格式
        value!({
            "type": 4,
            "data": {
                "content": text,
            }
        })
    }

    fn send_response<'a>(&'a self, response: &'a Value) -> Pin<Box<dyn Future<Output = DsaResult<()>> + Send + 'a>> {
        Box::pin(async move {
            let body = serde_json::to_string(response)
                .map_err(|e| DsaError::Internal(format!("序列化Discord响应失败: {}", e)))?;

            let resp = self.client
                .post(&self.webhook_url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bot {}", self.bot_token))
                .body(body)
                .send()
                .await
                .map_err(|e| DsaError::Internal(format!("Discord消息发送失败: {}", e)))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                tracing::warn!("[Discord] 发送失败 status={}, body={}", status, text);
                return Err(DsaError::Internal(format!(
                    "Discord消息发送失败: HTTP {}", status
                )));
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dingtalk_parse_message() {
        let adapter = DingTalkAdapter::new("https://example.com/webhook", "secret123");
        let body = value!({
            "msgtype": "text",
            "text": { "content": "/help" },
            "senderStaffId": "user001",
            "conversationId": "conv001",
            "isAdmin": true,
        });

        let msg = adapter.parse_message(&body).unwrap();
        assert_eq!(msg.text, "/help");
        assert_eq!(msg.user_id, "user001");
        assert_eq!(msg.chat_id, "conv001");
        assert!(msg.is_admin);
        assert_eq!(msg.platform, "dingtalk");
    }

    #[test]
    fn test_dingtalk_format_response() {
        let adapter = DingTalkAdapter::new("https://example.com/webhook", "secret123");
        let ctx = BotContext {
            platform: "dingtalk".to_string(),
            user_id: "u1".to_string(),
            chat_id: "c1".to_string(),
            is_admin: false,
        };
        let resp = adapter.format_response("Hello", &ctx);
        assert_eq!(resp.get("msgtype").and_then(|v| v.as_str()), Some("markdown".to_string()));
    }

    #[test]
    fn test_dingtalk_compute_sign_deterministic() {
        let adapter = DingTalkAdapter::new("https://example.com/webhook", "secret123");
        let sign1 = adapter.compute_sign("1609459200000");
        let sign2 = adapter.compute_sign("1609459200000");
        assert_eq!(sign1, sign2, "相同输入应产生相同签名");
    }

    #[test]
    fn test_feishu_parse_message() {
        let adapter = FeishuAdapter::new("https://example.com/webhook", "app_id", "app_secret", "token123");
        let body = value!({
            "event": {
                "message": {
                    "content": "{\"text\":\"/market\"}",
                    "chat_id": "oc_xxx",
                },
                "sender": {
                    "sender_id": { "user_id": "ou_xxx" }
                }
            }
        });

        let msg = adapter.parse_message(&body).unwrap();
        assert_eq!(msg.text, "/market");
        assert_eq!(msg.user_id, "ou_xxx");
        assert_eq!(msg.chat_id, "oc_xxx");
        assert_eq!(msg.platform, "feishu");
    }

    #[test]
    fn test_feishu_verify_request_with_token() {
        let adapter = FeishuAdapter::new("https://example.com/webhook", "app_id", "app_secret", "token123");
        let headers = value!({ "verification_token": "token123" });
        assert!(adapter.verify_request(&headers, &Value::Null).unwrap());
    }

    #[test]
    fn test_feishu_verify_request_wrong_token() {
        let adapter = FeishuAdapter::new("https://example.com/webhook", "app_id", "app_secret", "token123");
        let headers = value!({ "verification_token": "wrong_token" });
        assert!(!adapter.verify_request(&headers, &Value::Null).unwrap());
    }

    #[test]
    fn test_feishu_verify_request_empty_config() {
        let adapter = FeishuAdapter::new("https://example.com/webhook", "app_id", "app_secret", "");
        let headers = value!({});
        // 未配置 token 应跳过验证(返回 true)
        assert!(adapter.verify_request(&headers, &Value::Null).unwrap());
    }

    #[test]
    fn test_discord_parse_message() {
        let adapter = DiscordAdapter::new("https://example.com/webhook", "bot_token", "public_key");
        let body = value!({
            "type": 1,
            "data": { "content": "/analyze 600519" },
            "member": {
                "user": { "id": "123456789" },
                "permissions": "8",
            },
            "channel_id": "987654321",
        });

        let msg = adapter.parse_message(&body).unwrap();
        assert_eq!(msg.text, "/analyze 600519");
        assert_eq!(msg.user_id, "123456789");
        assert_eq!(msg.chat_id, "987654321");
        assert!(msg.is_admin); // 0x8 = ADMINISTRATOR
    }

    #[test]
    fn test_discord_parse_message_non_admin() {
        let adapter = DiscordAdapter::new("https://example.com/webhook", "bot_token", "public_key");
        let body = value!({
            "type": 1,
            "data": { "content": "/help" },
            "member": {
                "user": { "id": "111" },
                "permissions": "4",
            },
            "channel_id": "222",
        });

        let msg = adapter.parse_message(&body).unwrap();
        assert!(!msg.is_admin);
    }

    #[test]
    fn test_discord_verify_request_valid_headers() {
        let adapter = DiscordAdapter::new("https://example.com/webhook", "bot_token", "public_key");
        let headers = value!({
            "X-Signature-Ed25519": "somesig",
            "X-Signature-Timestamp": "1609459200",
        });
        assert!(adapter.verify_request(&headers, &Value::Null).unwrap());
    }

    #[test]
    fn test_discord_verify_request_missing_headers() {
        let adapter = DiscordAdapter::new("https://example.com/webhook", "bot_token", "public_key");
        let headers = value!({});
        assert!(!adapter.verify_request(&headers, &Value::Null).unwrap());
    }

    #[test]
    fn test_discord_format_response() {
        let adapter = DiscordAdapter::new("https://example.com/webhook", "bot_token", "public_key");
        let ctx = BotContext {
            platform: "discord".to_string(),
            user_id: "u1".to_string(),
            chat_id: "c1".to_string(),
            is_admin: false,
        };
        let resp = adapter.format_response("Hello Discord", &ctx);
        // type 4 = CHANNEL_MESSAGE_WITH_SOURCE
        assert_eq!(resp.get("type").and_then(|v| v.as_f64()), Some(4.0));
        let content = resp.get("data").and_then(|d| d.get("content")).and_then(|v| v.as_str());
        assert_eq!(content, Some("Hello Discord".to_string()));
    }
}
