//! 通知服务 - 多渠道消息推送
//!
//! 支持渠道: 钉钉、飞书、企业微信、Telegram、Bark、邮件
//!           Discord、Slack、Pushover、PushPlus、ServerChan、ntfy、Gotify、自定义Webhook
//! 所有渠道均已实现真实 HTTP 推送
//!
//! 路由规则:
//!   critical → 所有已配置渠道
//!   warning  → 钉钉/飞书/企微 + 邮件
//!   info     → 仅日志
//!
//! 静默规则:
//!   在静默时段(quiet_hours)内, info/warning 级别通知将被抑制, critical 不受影响

use dsa_core::{DsaError, DsaResult};
use tube::Value;
use chrono::Timelike;

pub struct NotificationService {
    client: reqwest::Client,
}

impl NotificationService {
    /// 创建通知服务实例
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// 请求分发 - 可用方法: send, channels, test, route
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "send" => self.send(params).await,
            "channels" => self.channels().await,
            "test" => self.test_channel(params).await,
            "route" => self.route(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "notification不支持方法: {}",
                method
            ))),
        }
    }

    async fn send(&self, params: &Value) -> DsaResult<Value> {
        let channel = params
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "log".to_string());
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let severity = params
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "info".to_string());

        // 静默时段检查: info/warning 级别在静默时段内被抑制, critical 不受影响
        if self.should_suppress(&severity) {
            tracing::info!("[通知][静默] 抑制发送 severity={} channel={}", severity, channel);
            return Ok(value!({"status": "suppressed", "reason": "quiet_hours", "channel": channel, "severity": severity}));
        }

        tracing::info!("[通知][{}] {} - {}", channel, title, content);

        match channel.as_str() {
            "dingtalk" => self.send_dingtalk(&title, &content).await,
            "feishu" => self.send_feishu(&title, &content).await,
            "wecom" => self.send_wecom(&title, &content).await,
            "telegram" => self.send_telegram(&title, &content).await,
            "bark" => self.send_bark(&title, &content).await,
            "email" => self.send_email(&title, &content).await,
            "discord" => self.send_discord(&title, &content).await,
            "slack" => self.send_slack(&title, &content).await,
            "pushover" => self.send_pushover(&title, &content).await,
            "pushplus" => self.send_pushplus(&title, &content).await,
            "serverchan" => self.send_serverchan(&title, &content).await,
            "ntfy" => self.send_ntfy(&title, &content).await,
            "gotify" => self.send_gotify(&title, &content).await,
            "custom_webhook" => self.send_custom_webhook(&title, &content).await,
            _ => {
                tracing::info!("[通知][默认] 标题: {} 内容: {}", title, content);
                Ok(value!({"status": "ok", "channel": channel, "sent": true}))
            }
        }
    }

    async fn channels(&self) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        Ok(value!([
            {"id": "dingtalk", "name": "钉钉", "enabled": !conf.notification.dingtalk_webhook.is_empty()},
            {"id": "feishu", "name": "飞书", "enabled": !conf.notification.feishu_webhook.is_empty()},
            {"id": "wecom", "name": "企业微信", "enabled": !conf.notification.wecom_webhook.is_empty()},
            {"id": "telegram", "name": "Telegram", "enabled": !conf.notification.telegram_bot_token.is_empty()},
            {"id": "bark", "name": "Bark", "enabled": !conf.notification.bark_url.is_empty()},
            {"id": "email", "name": "邮件", "enabled": !conf.notification.email_smtp_host.is_empty()},
            {"id": "discord", "name": "Discord", "enabled": !conf.notification.discord_webhook.is_empty()},
            {"id": "slack", "name": "Slack", "enabled": !conf.notification.slack_webhook.is_empty()},
            {"id": "pushover", "name": "Pushover", "enabled": !conf.notification.pushover_user_key.is_empty()},
            {"id": "pushplus", "name": "PushPlus", "enabled": !conf.notification.pushplus_token.is_empty()},
            {"id": "serverchan", "name": "ServerChan", "enabled": !conf.notification.serverchan_token.is_empty()},
            {"id": "ntfy", "name": "ntfy", "enabled": !conf.notification.ntfy_topic.is_empty()},
            {"id": "gotify", "name": "Gotify", "enabled": !conf.notification.gotify_app_token.is_empty()},
            {"id": "custom_webhook", "name": "自定义Webhook", "enabled": !conf.notification.custom_webhook_url.is_empty()},
        ]))
    }

    async fn test_channel(&self, params: &Value) -> DsaResult<Value> {
        let channel = params
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "log".to_string());
        tracing::info!("[通知测试] 频道: {}", channel);
        let result = self
            .send(&value!({
                "channel": channel.clone(),
                "title": "DSA通知测试",
                "content": "这是一条测试通知, 如果收到说明配置正确。",
            }))
            .await?;
        Ok(value!({"channel": channel, "test": true, "result": result}))
    }

    // -----------------------------------------------------------------------
    // 钉钉机器人 Webhook
    // -----------------------------------------------------------------------

    async fn send_dingtalk(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let webhook = &conf.notification.dingtalk_webhook;
        if webhook.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "钉钉webhook未配置"}));
        }

        let body = serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "title": title,
                "text": format!("### {}\n\n{}", title, content),
            }
        });

        match self.post_json(webhook, &body).await {
            Ok(resp) => {
                tracing::info!("[钉钉] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "dingtalk", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[钉钉] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "dingtalk", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // 飞书机器人 Webhook
    // -----------------------------------------------------------------------

    async fn send_feishu(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let webhook = &conf.notification.feishu_webhook;
        if webhook.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "飞书webhook未配置"}));
        }

        let body = serde_json::json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": title,
                    },
                    "template": "blue",
                },
                "elements": [
                    {
                        "tag": "markdown",
                        "content": content,
                    }
                ],
            }
        });

        match self.post_json(webhook, &body).await {
            Ok(resp) => {
                tracing::info!("[飞书] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "feishu", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[飞书] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "feishu", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // 企业微信机器人 Webhook
    // -----------------------------------------------------------------------

    async fn send_wecom(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let webhook = &conf.notification.wecom_webhook;
        if webhook.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "企业微信webhook未配置"}));
        }

        let body = serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "content": format!("### {}\n> {}", title, content),
            }
        });

        match self.post_json(webhook, &body).await {
            Ok(resp) => {
                tracing::info!("[企业微信] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "wecom", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[企业微信] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "wecom", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Telegram Bot API
    // -----------------------------------------------------------------------

    async fn send_telegram(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let bot_token = &conf.notification.telegram_bot_token;
        let chat_id = &conf.notification.telegram_chat_id;
        if bot_token.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Telegram bot token未配置"}));
        }
        if chat_id.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Telegram chat_id未配置"}));
        }

        let text = format!("*{}*\n\n{}", title, content);
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            bot_token
        );

        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown",
        });

        match self.post_json(&url, &body).await {
            Ok(resp) => {
                tracing::info!("[Telegram] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "telegram", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[Telegram] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "telegram", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Bark Push (iOS)
    // -----------------------------------------------------------------------

    async fn send_bark(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let bark_url = &conf.notification.bark_url;
        if bark_url.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Bark URL未配置"}));
        }

        let base = bark_url.trim_end_matches('/');
        let url = format!(
            "{}/{}/{}",
            base,
            urlencoding::encode(title),
            urlencoding::encode(content),
        );

        match self.client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                tracing::info!("[Bark] 发送完成 status={}", status);
                Ok(value!({"status": "ok", "channel": "bark", "httpStatus": status as i64, "response": body}))
            }
            Err(e) => {
                tracing::warn!("[Bark] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "bark", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Email (via SMTP)
    // 支持三种模式:
    //   1. email_smtp_host 含 "mailgun" -> Mailgun API
    //   2. email_smtp_host 含 "resend"  -> Resend API
    //   3. 其他 -> 通过 lettre crate 直连 SMTP 服务器
    // -----------------------------------------------------------------------

    async fn send_email(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let smtp_host = &conf.notification.email_smtp_host;
        if smtp_host.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "邮件SMTP未配置"}));
        }

        let from = &conf.notification.email_from;
        let to = &conf.notification.email_to;
        if from.is_empty() || to.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "邮件发件人/收件人未配置"}));
        }

        let host_lower = smtp_host.to_lowercase();

        if host_lower.contains("mailgun") {
            return self.send_mailgun(title, content, &conf).await;
        }

        if host_lower.contains("resend") {
            return self.send_resend(title, content, &conf).await;
        }

        self.send_smtp(title, content, &conf).await
    }

    async fn send_smtp(
        &self,
        title: &str,
        content: &str,
        conf: &dsa_core::config::AppConfig,
    ) -> DsaResult<Value> {
        use lettre::message::header::ContentType;
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

        let smtp_host = &conf.notification.email_smtp_host;
        let smtp_port = conf.notification.email_smtp_port;
        let email_user = &conf.notification.email_user;
        let email_pass = Self::resolve_key(
            &conf.notification.email_pass,
            &conf.notification.email_pass_env,
        );

        let from_addr = conf.notification.email_from.parse::<lettre::Address>();
        let to_addr = conf.notification.email_to.parse::<lettre::Address>();

        let (from_addr, to_addr) = match (from_addr, to_addr) {
            (Ok(f), Ok(t)) => (f, t),
            _ => {
                return Ok(value!({"status": "error", "channel": "email", "error": "邮件地址格式错误"}));
            }
        };

        let email = match Message::builder()
            .from(lettre::message::Mailbox::new(None, from_addr))
            .to(lettre::message::Mailbox::new(None, to_addr))
            .subject(title)
            .header(ContentType::TEXT_PLAIN)
            .body(content.to_string())
        {
            Ok(m) => m,
            Err(e) => {
                return Ok(value!({"status": "error", "channel": "email", "error": format!("构建邮件失败: {}", e)}));
            }
        };

        let mailer_result = if smtp_port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)
                .map(|builder| {
                    let mut b = builder.port(smtp_port);
                    if !email_user.is_empty() && !email_pass.is_empty() {
                        b = b.credentials(Credentials::new(email_user.clone(), email_pass));
                    }
                    b.build()
                })
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_host)
                .map(|builder| {
                    let mut b = builder.port(smtp_port);
                    if !email_user.is_empty() && !email_pass.is_empty() {
                        b = b.credentials(Credentials::new(email_user.clone(), email_pass));
                    }
                    b.build()
                })
        };

        let mailer = match mailer_result {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("[SMTP] 构建传输失败: {}", e);
                return Ok(value!({"status": "error", "channel": "email", "error": format!("SMTP配置失败: {}", e)}));
            }
        };

        match mailer.send(email).await {
            Ok(_) => {
                tracing::info!("[SMTP] 发送成功 to={}", conf.notification.email_to);
                Ok(value!({"status": "ok", "channel": "email", "provider": "smtp"}))
            }
            Err(e) => {
                tracing::warn!("[SMTP] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "email", "error": format!("{}", e)}))
            }
        }
    }

    async fn send_mailgun(
        &self,
        title: &str,
        content: &str,
        conf: &dsa_core::config::AppConfig,
    ) -> DsaResult<Value> {
        let domain = conf.notification.email_smtp_host.replace("mailgun:", "");
        if domain.is_empty() {
            return Ok(value!({"status": "error", "reason": "Mailgun域名未配置, 格式: mailgun:your-domain.com"}));
        }

        let api_key = Self::resolve_key(
            &conf.notification.email_pass,
            &conf.notification.email_pass_env,
        );
        if api_key.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Mailgun API Key未配置"}));
        }

        let url = format!("https://api.mailgun.net/v3/{}/messages", domain);

        let params = [
            ("from", conf.notification.email_from.as_str()),
            ("to", conf.notification.email_to.as_str()),
            ("subject", title),
            ("text", content),
        ];

        let resp = self
            .client
            .post(&url)
            .form(&params)
            .basic_auth("api", Some(&api_key))
            .send()
            .await;

        match resp {
            Ok(response) => {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                if status >= 200 && status < 300 {
                    tracing::info!("[Mailgun] 发送成功");
                    Ok(value!({"status": "ok", "channel": "email", "provider": "mailgun"}))
                } else {
                    tracing::warn!("[Mailgun] 发送失败 status={} body={}", status, body);
                    Ok(value!({"status": "error", "channel": "email", "httpStatus": status as i64}))
                }
            }
            Err(e) => {
                tracing::warn!("[Mailgun] 请求失败: {}", e);
                Ok(value!({"status": "error", "channel": "email", "error": e.to_string()}))
            }
        }
    }

    async fn send_resend(
        &self,
        title: &str,
        content: &str,
        conf: &dsa_core::config::AppConfig,
    ) -> DsaResult<Value> {
        let api_key = Self::resolve_key(
            &conf.notification.email_pass,
            &conf.notification.email_pass_env,
        );
        if api_key.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Resend API Key未配置"}));
        }

        let body = serde_json::json!({
            "from": conf.notification.email_from,
            "to": [conf.notification.email_to],
            "subject": title,
            "text": content,
        });

        let resp = self
            .client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match resp {
            Ok(response) => {
                let status = response.status().as_u16();
                let resp_body = response.text().await.unwrap_or_default();
                if status >= 200 && status < 300 {
                    tracing::info!("[Resend] 发送成功: {}", resp_body);
                    Ok(value!({"status": "ok", "channel": "email", "provider": "resend"}))
                } else {
                    tracing::warn!("[Resend] 发送失败 status={} body={}", status, resp_body);
                    Ok(value!({"status": "error", "channel": "email", "httpStatus": status as i64}))
                }
            }
            Err(e) => {
                tracing::warn!("[Resend] 请求失败: {}", e);
                Ok(value!({"status": "error", "channel": "email", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Discord Webhook
    // -----------------------------------------------------------------------

    async fn send_discord(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let webhook = &conf.notification.discord_webhook;
        if webhook.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Discord webhook未配置"}));
        }

        let body = serde_json::json!({
            "content": format!("{}\n{}", title, content),
        });

        match self.post_json(webhook, &body).await {
            Ok(resp) => {
                tracing::info!("[Discord] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "discord", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[Discord] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "discord", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Slack Webhook
    // -----------------------------------------------------------------------

    async fn send_slack(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let webhook = &conf.notification.slack_webhook;
        if webhook.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Slack webhook未配置"}));
        }

        let body = serde_json::json!({
            "text": format!("{}\n{}", title, content),
        });

        match self.post_json(webhook, &body).await {
            Ok(resp) => {
                tracing::info!("[Slack] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "slack", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[Slack] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "slack", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Pushover
    // -----------------------------------------------------------------------

    async fn send_pushover(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let user_key = &conf.notification.pushover_user_key;
        let api_token = &conf.notification.pushover_api_token;
        if user_key.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Pushover user key未配置"}));
        }
        if api_token.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Pushover API token未配置"}));
        }

        let url = "https://api.pushover.net/1/messages.json";
        let params = [
            ("user", user_key.as_str()),
            ("token", api_token.as_str()),
            ("message", content),
            ("title", title),
        ];

        let resp = self
            .client
            .post(url)
            .form(&params)
            .send()
            .await;

        match resp {
            Ok(response) => {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                if status >= 200 && status < 300 {
                    tracing::info!("[Pushover] 发送成功");
                    Ok(value!({"status": "ok", "channel": "pushover", "response": body}))
                } else {
                    tracing::warn!("[Pushover] 发送失败 status={} body={}", status, body);
                    Ok(value!({"status": "error", "channel": "pushover", "httpStatus": status as i64}))
                }
            }
            Err(e) => {
                tracing::warn!("[Pushover] 请求失败: {}", e);
                Ok(value!({"status": "error", "channel": "pushover", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // PushPlus (微信推送)
    // -----------------------------------------------------------------------

    async fn send_pushplus(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let token = &conf.notification.pushplus_token;
        if token.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "PushPlus token未配置"}));
        }

        let url = "http://www.pushplus.plus/send";
        let body = serde_json::json!({
            "token": token,
            "title": title,
            "content": content,
            "template": "html",
        });

        match self.post_json(url, &body).await {
            Ok(resp) => {
                tracing::info!("[PushPlus] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "pushplus", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[PushPlus] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "pushplus", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // ServerChan (Server酱)
    // -----------------------------------------------------------------------

    async fn send_serverchan(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let token = &conf.notification.serverchan_token;
        if token.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "ServerChan token未配置"}));
        }

        let url = format!("https://sctapi.ftqq.com/{}.send", token);
        let body = serde_json::json!({
            "title": title,
            "desp": content,
        });

        match self.post_json(&url, &body).await {
            Ok(resp) => {
                tracing::info!("[ServerChan] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "serverchan", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[ServerChan] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "serverchan", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // ntfy
    // -----------------------------------------------------------------------

    async fn send_ntfy(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let topic = &conf.notification.ntfy_topic;
        if topic.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "ntfy topic未配置"}));
        }

        let server = if conf.notification.ntfy_server.is_empty() {
            "https://ntfy.sh"
        } else {
            &conf.notification.ntfy_server
        };
        let base = server.trim_end_matches('/');
        let url = format!("{}/{}", base, topic);

        let resp = self
            .client
            .post(&url)
            .header("Title", title)
            .header("Priority", "default")
            .header("Content-Type", "text/plain")
            .body(content.to_string())
            .send()
            .await;

        match resp {
            Ok(response) => {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                if status >= 200 && status < 300 {
                    tracing::info!("[ntfy] 发送成功");
                    Ok(value!({"status": "ok", "channel": "ntfy", "response": body}))
                } else {
                    tracing::warn!("[ntfy] 发送失败 status={} body={}", status, body);
                    Ok(value!({"status": "error", "channel": "ntfy", "httpStatus": status as i64}))
                }
            }
            Err(e) => {
                tracing::warn!("[ntfy] 请求失败: {}", e);
                Ok(value!({"status": "error", "channel": "ntfy", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Gotify
    // -----------------------------------------------------------------------

    async fn send_gotify(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let server = &conf.notification.gotify_server;
        let app_token = &conf.notification.gotify_app_token;
        if server.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Gotify server未配置"}));
        }
        if app_token.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "Gotify app token未配置"}));
        }

        let base = server.trim_end_matches('/');
        let url = format!("{}/message?token={}", base, app_token);

        let body = serde_json::json!({
            "title": title,
            "message": content,
            "priority": 5,
        });

        match self.post_json(&url, &body).await {
            Ok(resp) => {
                tracing::info!("[Gotify] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "gotify", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[Gotify] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "gotify", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // 自定义 Webhook
    // -----------------------------------------------------------------------

    async fn send_custom_webhook(&self, title: &str, content: &str) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let webhook_url = &conf.notification.custom_webhook_url;
        if webhook_url.is_empty() {
            return Ok(value!({"status": "skipped", "reason": "自定义Webhook URL未配置"}));
        }

        let body = serde_json::json!({
            "title": title,
            "content": content,
            "channel": "custom_webhook",
        });

        match self.post_json(webhook_url, &body).await {
            Ok(resp) => {
                tracing::info!("[自定义Webhook] 发送成功: {}", resp);
                Ok(value!({"status": "ok", "channel": "custom_webhook", "response": resp}))
            }
            Err(e) => {
                tracing::warn!("[自定义Webhook] 发送失败: {}", e);
                Ok(value!({"status": "error", "channel": "custom_webhook", "error": e.to_string()}))
            }
        }
    }

    // -----------------------------------------------------------------------
    // 路由 & 静默
    // -----------------------------------------------------------------------

    /// 根据严重程度和类型推荐通知渠道
    async fn route(&self, params: &Value) -> DsaResult<Value> {
        let severity = params
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "info".to_string());
        let ntype = params
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "alert".to_string());

        let conf = dsa_core::get_global_config();

        let recommended = match severity.as_str() {
            "critical" => {
                // critical → 所有已配置渠道
                let mut channels = Vec::new();
                if !conf.notification.dingtalk_webhook.is_empty() { channels.push("dingtalk"); }
                if !conf.notification.feishu_webhook.is_empty() { channels.push("feishu"); }
                if !conf.notification.wecom_webhook.is_empty() { channels.push("wecom"); }
                if !conf.notification.telegram_bot_token.is_empty() { channels.push("telegram"); }
                if !conf.notification.bark_url.is_empty() { channels.push("bark"); }
                if !conf.notification.email_smtp_host.is_empty() { channels.push("email"); }
                if !conf.notification.discord_webhook.is_empty() { channels.push("discord"); }
                if !conf.notification.slack_webhook.is_empty() { channels.push("slack"); }
                if !conf.notification.pushover_user_key.is_empty() { channels.push("pushover"); }
                if !conf.notification.pushplus_token.is_empty() { channels.push("pushplus"); }
                if !conf.notification.serverchan_token.is_empty() { channels.push("serverchan"); }
                if !conf.notification.ntfy_topic.is_empty() { channels.push("ntfy"); }
                if !conf.notification.gotify_app_token.is_empty() { channels.push("gotify"); }
                if !conf.notification.custom_webhook_url.is_empty() { channels.push("custom_webhook"); }
                channels
            }
            "warning" => {
                // warning → 钉钉/飞书/企微 + 邮件
                let mut channels = Vec::new();
                if !conf.notification.dingtalk_webhook.is_empty() { channels.push("dingtalk"); }
                if !conf.notification.feishu_webhook.is_empty() { channels.push("feishu"); }
                if !conf.notification.wecom_webhook.is_empty() { channels.push("wecom"); }
                if !conf.notification.email_smtp_host.is_empty() { channels.push("email"); }
                channels
            }
            _ => {
                // info → 仅日志
                vec!["log"]
            }
        };

        Ok(value!({
            "severity": severity,
            "type": ntype,
            "channels": recommended,
        }))
    }

    /// 检查当前是否处于静默时段, 且该严重程度应被抑制
    fn should_suppress(&self, severity: &str) -> bool {
        // critical 级别永远不抑制
        if severity == "critical" {
            return false;
        }

        let conf = dsa_core::get_global_config();
        let start = conf.notification.quiet_hours_start;
        let end = conf.notification.quiet_hours_end;

        // 如果 start == end, 说明没有静默时段
        if start == end {
            return false;
        }

        let now = chrono::Local::now();
        let hour = now.hour() as i32;

        let in_quiet = if start > end {
            // 跨午夜, 如 23-7 表示 23:00~07:00
            hour >= start || hour < end
        } else {
            // 同日, 如 1-5 表示 01:00~05:00
            hour >= start && hour < end
        };

        if in_quiet {
            // 静默时段内: 抑制 info 和 warning
            tracing::debug!(
                "[通知][静默] 当前处于静默时段 {}-{} 当前小时={}, 抑制 severity={}",
                start, end, hour, severity
            );
            return true;
        }

        false
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    async fn post_json(&self, url: &str, body: &serde_json::Value) -> Result<String, reqwest::Error> {
        let resp = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            tracing::warn!("[HTTP] POST {} 返回 status={} body={}", url, status, text);
        }

        Ok(text)
    }

    fn resolve_key(direct: &str, env_name: &str) -> String {
        if !direct.is_empty() {
            return direct.to_string();
        }
        if !env_name.is_empty() {
            if let Ok(key) = std::env::var(env_name) {
                return key;
            }
        }
        String::new()
    }
}
