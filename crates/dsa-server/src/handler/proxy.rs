//! External API proxy handler
//! Unified proxy layer for all outbound HTTP requests (both frontend-forwarded and server-internal)

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

pub fn is_url_allowed(url: &str) -> bool {
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

pub async fn send_proxy_request(
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
    timeout_secs: u64,
) -> Result<serde_json::Value, String> {
    if !is_url_allowed(url) {
        return Err(format!("Domain not allowed: {}", url));
    }

    tube::log!("[PROXY] {} {} body={:?}", method, url,
        body.map(|b| if b.len() > 200 { &b[..200] } else { b }));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_default();

    let req_method = match method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        _ => reqwest::Method::POST,
    };

    let mut request_builder = client.request(req_method, url);

    for (key, value) in headers {
        request_builder = request_builder.header(key.as_str(), value.as_str());
    }

    if let Some(body_str) = body {
        let content_type = headers.get("Content-Type").map(|s| s.as_str()).unwrap_or("application/json");
        request_builder = request_builder.header("Content-Type", content_type).body(body_str.to_owned());
    }

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();
            let response_text = response.text().await.unwrap_or_default();

            tube::log!("[PROXY] Response status={}, body={:?}", status,
                if response_text.len() > 200 { &response_text[..200] } else { &response_text });

            if status.as_u16() >= 400 {
                return Err(format!("Remote returned error: {} - {}", status, response_text));
            }

            serde_json::from_str(&response_text)
                .map_err(|e| format!("Failed to parse remote response: {}", e))
        }
        Err(e) => {
            Err(format!("Proxy request failed: {}", e))
        }
    }
}

pub async fn proxy_post(_req: HttpRequest, payload: web::Json<ProxyRequest>) -> HttpResponse {
    let proxy_req = payload.into_inner();

    match send_proxy_request(
        &proxy_req.url,
        &proxy_req.method,
        &proxy_req.headers,
        proxy_req.body.as_deref(),
        30,
    ).await {
        Ok(response_json) => {
            HttpResponse::Ok().json(serde_json::json!({
                "code": 200,
                "result": response_json,
                "message": ""
            }))
        }
        Err(err_msg) => {
            let code = if err_msg.starts_with("Domain not allowed") { 403 } else { 500 };
            tube::log!("[PROXY] Request failed: {}", err_msg);
            HttpResponse::Ok().json(serde_json::json!({
                "code": code,
                "result": serde_json::Value::Null,
                "message": err_msg
            }))
        }
    }
}
