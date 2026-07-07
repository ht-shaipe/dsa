//! 基础认证服务

use dsa_core::{DsaError, DsaResult, utils};
use tube::Value;

/// 常量时间字符串比较 - 防止时序攻击
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

/// 获取期望密码 - 依次检查内存覆盖、环境变量、配置文件
fn get_expected_password() -> String {
    // Check in-memory override first, then env var, then config
    if let Some(pwd) = dsa_core::get_password_override() {
        return pwd;
    }
    std::env::var("DSA_PASSWORD").unwrap_or_else(|_| {
        let conf = dsa_core::get_global_config();
        conf.database.password.clone()
    })
}

/// 基础认证服务
pub struct AuthService {}

impl AuthService {
    /// 创建认证服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: status, login, change_password, logout, settings
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "status" => self.status().await,
            "login" => self.login(params).await,
            "change_password" => self.change_password(params).await,
            "logout" => self.logout().await,
            "settings" => self.settings().await,
            _ => Err(DsaError::ApiRouting(format!(
                "auth不支持方法: {}",
                method
            ))),
        }
    }

    /// 认证状态
    async fn status(&self) -> DsaResult<Value> {
        let has_password = dsa_core::get_password_override().is_some()
            || std::env::var("DSA_PASSWORD").is_ok()
            || {
                let conf = dsa_core::get_global_config();
                !conf.database.password.is_empty()
            };

        Ok(value!({
            "status": "ok",
            "data": {
                "authEnabled": has_password,
                "requireLogin": has_password,
            }
        }))
    }

    /// 登录验证
    async fn login(&self, params: &Value) -> DsaResult<Value> {
        let password = utils::param_string(params, "password");

        let expected = get_expected_password();

        if expected.is_empty() {
            // 无密码配置，免登录
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
            // 使用UUID token代替时间戳
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

    /// 修改密码
    async fn change_password(&self, params: &Value) -> DsaResult<Value> {
        let old_password = utils::param_string(params, "oldPassword");
        let new_password = utils::param_string(params, "newPassword");

        if new_password.is_empty() {
            return Err(DsaError::Validation("请提供新密码".to_string()));
        }

        let expected = get_expected_password();

        if !expected.is_empty() && !constant_time_eq(&old_password, &expected) {
            return Err(DsaError::Validation("原密码错误".to_string()));
        }

        // 使用in-memory方式存储密码，而非set_var
        dsa_core::set_password_override(new_password);

        Ok(value!({"status": "ok", "message": "密码已更新"}))
    }

    /// 登出
    async fn logout(&self) -> DsaResult<Value> {
        dsa_core::clear_auth_token();
        Ok(value!({"status": "ok", "message": "已登出"}))
    }

    /// 认证设置
    async fn settings(&self) -> DsaResult<Value> {
        Ok(value!({
            "status": "ok",
            "data": {
                "authMode": "password",
                "sessionTimeout": 3600,
            }
        }))
    }
}
