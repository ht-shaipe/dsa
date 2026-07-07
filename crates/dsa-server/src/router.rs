//! API 路由分发 - soma 模式 parse_request → module.method dispatch

use actix_web::{web, Error as ActixError, HttpRequest, HttpResponse};
use tube_web::response::{get_error, get_success};

/// Check auth token for protected endpoints.
/// Returns Ok(()) if the request is allowed, Err with error message if unauthorized.
fn check_auth(req: &HttpRequest) -> Result<(), &'static str> {
    // Parse the module from path: /api/v1/{module} or /api/v1/{module}/{tail}
    let path = req.path();

    // Skip auth for health check
    if path == "/health" {
        return Ok(());
    }

    // Extract module name from path for auth endpoint check
    // Path format: /api/v1/{module} or /api/v1/{module}/...
    let module = path
        .trim_start_matches("/api/v1/")
        .split('/')
        .next()
        .unwrap_or("");

    // Skip auth for auth endpoints (login, status, etc.)
    if module == "auth" {
        return Ok(());
    }

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

    // Also check query param token (for SSE streams and WebSocket)
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

pub async fn api_handler(req: HttpRequest, payload: web::Payload) -> Result<HttpResponse, ActixError> {
    // Auth check
    if let Err(msg) = check_auth(&req) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": msg
        })));
    }

    let param = tube_web::parse_request(req, payload).await;

    let res = match param.module.to_lowercase().as_str() {
        "stock" => crate::handler::stock::distribute(&param).await,
        "analysis" => crate::handler::analysis::distribute(&param).await,
        "market" => crate::handler::market::distribute(&param).await,
        "agent" => crate::handler::agent::distribute(&param).await,
        "backtest" => crate::handler::backtest::distribute(&param).await,
        "scheduler" => crate::handler::scheduler::distribute(&param).await,
        "portfolio" => crate::handler::portfolio::distribute(&param).await,
        "config" => crate::handler::config_handler::distribute(&param).await,
        "decision" => crate::handler::decision::distribute(&param).await,
        "intelligence" => crate::handler::intelligence::distribute(&param).await,
        "alert" => crate::handler::alert::distribute(&param).await,
        "usage" => crate::handler::usage::distribute(&param).await,
        "auth" => crate::handler::auth::distribute(&param).await,
        "system" => crate::handler::system::distribute(&param).await,
        "screening" => crate::handler::screening::distribute(&param).await,
        "notification" => crate::handler::notification::distribute(&param).await,
        "search" => crate::handler::search::distribute(&param).await,
        "social_sentiment" => crate::handler::social_sentiment::distribute(&param).await,
        "backtest_worker" => crate::handler::backtest_worker::distribute(&param).await,
        "alert_worker" => crate::handler::alert_worker::distribute(&param).await,
        "decision_extractor" => crate::handler::decision_extractor::distribute(&param).await,
        "market_context" => crate::handler::market_context::distribute(&param).await,
        "name_resolver" => crate::handler::name_resolver::distribute(&param).await,
        "report" => crate::handler::report::distribute(&param).await,
        "bot" => crate::handler::bot::distribute(&param).await,
        _ => Err(error!("请求方法{}.{}系统未提供。", param.module, param.method)),
    };

    match res {
        Ok(v) => get_success(&v),
        Err(e) => get_error(e),
    }
}

pub async fn health_check() -> Result<HttpResponse, ActixError> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "dsa",
        "version": env!("CARGO_PKG_VERSION"),
    })))
}
