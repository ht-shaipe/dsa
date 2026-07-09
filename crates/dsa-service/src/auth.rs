use dsa_core::utils;
use tube::{Result, Value};
use tube_web::RequestParameter;

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

fn get_expected_password() -> String {
    if let Some(pwd) = dsa_core::get_password_override() {
        return pwd;
    }
    if !std::env::var("DSA_PASSWORD").is_err() {
        return std::env::var("DSA_PASSWORD").unwrap();
    }
    let conf = dsa_core::get_global_config();
    if !conf.server.auth_password_env.is_empty() {
        if let Ok(pwd) = std::env::var(&conf.server.auth_password_env) {
            return pwd;
        }
    }
    conf.server.auth_password.clone()
}

pub struct Auth { request: RequestParameter }

impl Auth {
    pub fn new(param: &RequestParameter) -> Self { Auth { request: param.clone() } }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "status" => self.status().await,
            "login" => self.login().await,
            "change_password" => self.change_password().await,
            "logout" => self.logout().await,
            "settings" => self.settings().await,
            _ => Err(error!("auth不支持方法: {}", method)),
        }
    }

    async fn status(&self) -> Result<Value> {
        let expected = get_expected_password();
        let has_password = dsa_core::get_password_override().is_some()
            || std::env::var("DSA_PASSWORD").is_ok()
            || !expected.is_empty();

        Ok(value!({
            "status": "ok",
            "data": {
                "authEnabled": has_password,
                "requireLogin": has_password,
            }
        }))
    }

    async fn login(&self) -> Result<Value> {
        let password = utils::param_string(&self.request.value, "password");
        let expected = get_expected_password();

        if expected.is_empty() {
            return Ok(value!({
                "status": "ok",
                "data": {
                    "authenticated": true,
                    "token": "no-auth-required",
                    "message": "未配置密码，免登录",
                }
            }));
        }

        if constant_time_eq(&password, &expected) {
            let token = uuid::Uuid::new_v4().to_string();
            dsa_core::set_auth_token(token.clone());
            Ok(value!({
                "status": "ok",
                "data": {
                    "authenticated": true,
                    "token": token,
                }
            }))
        } else {
            Ok(value!({
                "status": "ok",
                "data": {
                    "authenticated": false,
                    "message": "密码错误",
                }
            }))
        }
    }

    async fn change_password(&self) -> Result<Value> {
        let old_password = utils::param_string(&self.request.value, "oldPassword");
        let new_password = utils::param_string(&self.request.value, "newPassword");

        if new_password.is_empty() {
            return Err(error!("请提供新密码"));
        }

        let expected = get_expected_password();

        if !expected.is_empty() && !constant_time_eq(&old_password, &expected) {
            return Err(error!("原密码错误"));
        }

        dsa_core::set_password_override(new_password);

        Ok(value!({"status": "ok", "message": "密码已更新"}))
    }

    async fn logout(&self) -> Result<Value> {
        dsa_core::clear_auth_token();
        Ok(value!({"status": "ok", "message": "已登出"}))
    }

    async fn settings(&self) -> Result<Value> {
        Ok(value!({
            "status": "ok",
            "data": {
                "authMode": "password",
                "sessionTimeout": 3600,
            }
        }))
    }
}
