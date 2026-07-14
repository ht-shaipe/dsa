//! API 路由分发 - soma 模式 parse_request → module.method dispatch

use actix_web::{web, Error as ActixError, HttpRequest, HttpResponse};
use tube_web::response::{get_error, get_success};

pub async fn api_handler(req: HttpRequest, payload: web::Payload) -> Result<HttpResponse, ActixError> {
    api_handler_inner(req, payload).await
}

async fn api_handler_inner(req: HttpRequest, payload: web::Payload) -> Result<HttpResponse, ActixError> {
    let mut param = tube_web::parse_request(req, payload).await;

    if param.method == "get" && !param.path.is_empty() {
        param.method = param.path.replace("/", ".");
    }

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
        "indicator" => crate::handler::indicator::distribute(&param).await,
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
