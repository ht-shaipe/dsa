use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tube_jwt::{Authorizer, Provider, Visitor};

use super::proxy::send_proxy_request;

const REMOTE_AUTH_BASE: &str = "https://auth.htui.cc/api/pas";

#[derive(Deserialize)]
pub struct LoginRequest {
    pub mobile: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub mobile: String,
    pub password: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct ProfileUpdateRequest {
    pub token: String,
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub token: String,
    pub old_password: String,
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct TokenOnlyRequest {
    pub token: String,
}

#[derive(Serialize)]
struct LoginResult {
    remote_token: String,
    local_token: String,
    user: serde_json::Value,
}

async fn proxy_post(url: &str, body: &str) -> Result<serde_json::Value, String> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    send_proxy_request(url, "POST", &headers, Some(body), 15).await
}

async fn proxy_get_with_token(url: &str, token: &str) -> Result<serde_json::Value, String> {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), token.to_string());
    send_proxy_request(url, "GET", &headers, None, 15).await
}

async fn proxy_put_with_token(
    url: &str,
    token: &str,
    body: &str,
) -> Result<serde_json::Value, String> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Authorization".to_string(), token.to_string());
    send_proxy_request(url, "PUT", &headers, Some(body), 15).await
}

async fn proxy_post_with_token(
    url: &str,
    token: &str,
    body: &str,
) -> Result<serde_json::Value, String> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Authorization".to_string(), token.to_string());
    send_proxy_request(url, "POST", &headers, Some(body), 15).await
}

fn generate_local_token(
    _remote_token: &str,
    user_data: &serde_json::Value,
) -> Result<String, String> {
    let user_id = user_data.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
    let mobile = user_data
        .get("mobile")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let name = user_data
        .get("name")
        .or_else(|| user_data.get("nickname"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let avatar = user_data
        .get("avatar")
        .or_else(|| user_data.get("avatar_url"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let visitor = Visitor {
        id: user_id,
        platform: 9,
        mobile: mobile.to_owned(),
        name: name.to_owned(),
        avatar: avatar.to_owned(),
        ..Default::default()
    };

    let provider = Provider::new_provider(9, user_id);

    let expire = {
        use chrono::{Duration, Local};
        (Local::now() + Duration::try_days(30).unwrap_or_default()).timestamp_millis()
    };

    let auth = Authorizer {
        visitor,
        provider,
        rent: 0,
        rent_id: 0,
        expire,
        ip: String::new(),
        terminal: String::new(),
        permissions: vec![],
    };

    auth.get_token()
        .map_err(|e| format!("生成本地Token失败: {}", e.get_message()))
}

pub async fn login(req: web::Json<LoginRequest>) -> HttpResponse {
    let body = serde_json::json!({
        "mobile": req.mobile,
        "password": req.password,
    });

    let url = format!("{}/user/login", REMOTE_AUTH_BASE);

    match proxy_post(&url, &body.to_string()).await {
        Ok(data) => {
            let result = data
                .get("result")
                .or_else(|| data.get("data"))
                .unwrap_or(&data);
            let remote_token = result.get("token").and_then(|v| v.as_str()).unwrap_or("");

            if remote_token.is_empty() {
                let msg = data
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("登录失败");
                return HttpResponse::Ok().json(serde_json::json!({
                    "code": 4001,
                    "result": null,
                    "message": msg
                }));
            }

            let user_data = result.get("user").cloned().unwrap_or(serde_json::json!({}));

            match generate_local_token(remote_token, &user_data) {
                Ok(local_token) => {
                    let login_result = LoginResult {
                        remote_token: remote_token.to_owned(),
                        local_token,
                        user: user_data,
                    };
                    HttpResponse::Ok().json(serde_json::json!({
                        "code": 200,
                        "result": login_result,
                        "message": ""
                    }))
                }
                Err(err) => HttpResponse::Ok().json(serde_json::json!({
                    "code": 500,
                    "result": null,
                    "message": err
                })),
            }
        }
        Err(err) => HttpResponse::Ok().json(serde_json::json!({
            "code": 500,
            "result": null,
            "message": err
        })),
    }
}

pub async fn register(req: web::Json<RegisterRequest>) -> HttpResponse {
    let mut body = serde_json::json!({
        "mobile": req.mobile,
        "password": req.password,
    });
    if let Some(name) = &req.name {
        body["name"] = serde_json::Value::String(name.clone());
    }

    let url = format!("{}/user/register", REMOTE_AUTH_BASE);

    match proxy_post(&url, &body.to_string()).await {
        Ok(data) => {
            let result = data
                .get("result")
                .or_else(|| data.get("data"))
                .unwrap_or(&data);
            HttpResponse::Ok().json(serde_json::json!({
                "code": 200,
                "result": result,
                "message": ""
            }))
        }
        Err(err) => HttpResponse::Ok().json(serde_json::json!({
            "code": 500,
            "result": null,
            "message": err
        })),
    }
}

pub async fn get_profile(req: web::Json<TokenOnlyRequest>) -> HttpResponse {
    let url = format!("{}/user/profile", REMOTE_AUTH_BASE);

    match proxy_get_with_token(&url, &req.token).await {
        Ok(data) => {
            let result = data
                .get("result")
                .or_else(|| data.get("data"))
                .unwrap_or(&data);
            HttpResponse::Ok().json(serde_json::json!({
                "code": 200,
                "result": result,
                "message": ""
            }))
        }
        Err(err) => HttpResponse::Ok().json(serde_json::json!({
            "code": 500,
            "result": null,
            "message": err
        })),
    }
}

pub async fn update_profile(req: web::Json<ProfileUpdateRequest>) -> HttpResponse {
    let mut body = serde_json::json!({
        "name": req.name,
    });
    if let Some(avatar) = &req.avatar {
        body["avatar"] = serde_json::Value::String(avatar.clone());
    }

    let url = format!("{}/user/profile", REMOTE_AUTH_BASE);

    match proxy_put_with_token(&url, &req.token, &body.to_string()).await {
        Ok(data) => {
            let result = data
                .get("result")
                .or_else(|| data.get("data"))
                .unwrap_or(&data);
            HttpResponse::Ok().json(serde_json::json!({
                "code": 200,
                "result": result,
                "message": ""
            }))
        }
        Err(err) => HttpResponse::Ok().json(serde_json::json!({
            "code": 500,
            "result": null,
            "message": err
        })),
    }
}

pub async fn change_password(req: web::Json<ChangePasswordRequest>) -> HttpResponse {
    let body = serde_json::json!({
        "oldPassword": req.old_password,
        "newPassword": req.new_password,
    });

    let url = format!("{}/user/change-password", REMOTE_AUTH_BASE);

    match proxy_post_with_token(&url, &req.token, &body.to_string()).await {
        Ok(data) => {
            let result = data
                .get("result")
                .or_else(|| data.get("data"))
                .unwrap_or(&data);
            HttpResponse::Ok().json(serde_json::json!({
                "code": 200,
                "result": result,
                "message": ""
            }))
        }
        Err(err) => HttpResponse::Ok().json(serde_json::json!({
            "code": 500,
            "result": null,
            "message": err
        })),
    }
}
