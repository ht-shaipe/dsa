//! SSE streaming handler - Agent chat streaming endpoint

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use tube_web::sse_channel;

/// Check auth for stream endpoint (same logic as router::check_auth)
fn check_stream_auth(req: &HttpRequest) -> Result<(), &'static str> {
    // If no password is configured, skip auth
    let has_password = dsa_core::get_password_override().is_some()
        || std::env::var("DSA_PASSWORD").is_ok()
        || {
            let conf = dsa_core::get_global_config();
            !conf.database.password.is_empty()
        };
    if !has_password {
        return Ok(());
    }

    // Check Authorization header
    if let Some(auth_header) = req.headers().get("Authorization").and_then(|v| v.to_str().ok()) {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..];
            let stored = dsa_core::get_auth_token();
            if let Some(ref stored_token) = stored {
                if token == stored_token.as_str() {
                    return Ok(());
                }
            }
        }
    }

    // Also check query param token (for SSE streams)
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some(token_val) = pair.strip_prefix("token=") {
                let stored = dsa_core::get_auth_token();
                if let Some(ref stored_token) = stored {
                    if token_val == stored_token.as_str() {
                        return Ok(());
                    }
                }
            }
        }
    }

    Err("未授权访问")
}

pub async fn chat_stream(req: HttpRequest, payload: web::Payload) -> HttpResponse {
    // Auth check
    if let Err(msg) = check_stream_auth(&req) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": msg
        }));
    }

    let param = tube_web::parse_request(req.clone(), payload).await;
    let (mut sender, receiver) = sse_channel(10);

    let message = param
        .value
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    actix_rt::spawn(async move {
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();

        if api_key.is_empty() {
            let msg = serde_json::json!({"type": "error", "content": "API Key未配置"});
            let _ = sender
                .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                .await;
            let _ = sender.done("{}").await;
            return;
        }

        let llm_provider = match ai_llm_kit::LlmProvider::instance(&conf.llm.provider) {
            Ok(p) => p,
            Err(_) => {
                let msg = serde_json::json!({"type": "error", "content": "Provider错误"});
                let _ = sender
                    .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                    .await;
                let _ = sender.done("{}").await;
                return;
            }
        };
        let llm = ai_llm_kit::LlmFactory::create(llm_provider, &api_key);

        let msg = serde_json::json!({"type": "thinking"});
        let _ = sender.send_data(&serde_json::to_string(&msg).unwrap_or_default()).await;

        let body = value!({
            "model": &conf.llm.model,
            "messages": [{"role": "user", "content": &message}],
        });

        match llm.chat(&body).await {
            Ok(resp) => {
                let content = resp
                    .get("choices")
                    .and_then(|c| tube::Value::as_array(&c.clone()))
                    .and_then(|a| a.first().cloned())
                    .and_then(|first| first.get("message").cloned())
                    .and_then(|msg| msg.get("content").and_then(|c| c.as_str()))
                    .unwrap_or_default();
                let json_content = serde_json::to_string(&content).unwrap_or_default();
                let out = serde_json::json!({"type": "message", "content": serde_json::from_str::<serde_json::Value>(&json_content).unwrap_or_default()});
                let _ = sender
                    .send_data(&serde_json::to_string(&out).unwrap_or_default())
                    .await;
            }
            Err(e) => {
                let out = serde_json::json!({"type": "error", "content": format!("{}", e)});
                let _ = sender
                    .send_data(&serde_json::to_string(&out).unwrap_or_default())
                    .await;
            }
        }
        let _ = sender.done("{}").await;
    });

    receiver.respond_to(&req)
}
