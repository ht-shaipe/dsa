//! External API proxy handler
//! Forwards requests from frontend to external services, bypassing Tauri CSP restrictions

use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxyRequest {
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxyResponse {
    pub code: i32,
    pub result: serde_json::Value,
    pub message: String,
}

const ALLOWED_DOMAINS: &[&str] = &[
    "auth.htui.cc",
];

fn is_url_allowed(url: &str) -> bool {
    let host = if url.starts_with("https://") {
        url.trim_start_matches("https://").split('/').next()
    } else if url.starts_with("http://") {
        url.trim_start_matches("http://").split('/').next()
    } else {
        None
    };
    if let Some(h) = host {
        return ALLOWED_DOMAINS.iter().any(|d| h == *d || h.ends_with(&format!(".{}", d)));
    }
    false
}

pub async fn proxy_post(_req: HttpRequest, payload: web::Json<ProxyRequest>) -> HttpResponse {
    let proxy_req = payload.into_inner();

    if !is_url_allowed(&proxy_req.url) {
        tube::log!("[PROXY] Blocked disallowed URL: {}", proxy_req.url);
        return HttpResponse::Ok().json(serde_json::json!({
            "code": 403,
            "result": serde_json::Value::Null,
            "message": format!("Domain not allowed: {}", proxy_req.url)
        }));
    }

    tube::log!("[PROXY] {} {} body={:?}", proxy_req.method, proxy_req.url,
        proxy_req.body.as_ref().map(|b| if b.len() > 200 { &b[..200] } else { b as &str }));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let method = match proxy_req.method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::POST,
    };

    let mut request_builder = client.request(method, &proxy_req.url);

    for (key, value) in &proxy_req.headers {
        request_builder = request_builder.header(key.as_str(), value.as_str());
    }

    if let Some(body_str) = &proxy_req.body {
        let content_type = proxy_req.headers.get("Content-Type").map(|s| s.as_str()).unwrap_or("application/json");
        request_builder = request_builder.header("Content-Type", content_type).body(body_str.clone());
    }

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();
            let response_text = response.text().await.unwrap_or_default();

            tube::log!("[PROXY] Response status={}, body={:?}", status,
                if response_text.len() > 200 { &response_text[..200] } else { &response_text });

            let response_json: serde_json::Value = match serde_json::from_str(&response_text) {
                Ok(v) => v,
                Err(_) => serde_json::Value::String(response_text),
            };

            HttpResponse::Ok().json(serde_json::json!({
                "code": 200,
                "result": response_json,
                "message": ""
            }))
        }
        Err(e) => {
            tube::log!("[PROXY] Request failed: {}", e);
            HttpResponse::Ok().json(serde_json::json!({
                "code": 500,
                "result": serde_json::Value::Null,
                "message": format!("Proxy request failed: {}", e)
            }))
        }
    }
}
