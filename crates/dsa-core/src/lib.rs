//! DSA Core - 数据模型、配置、错误定义

pub mod config;
pub mod db;
pub mod errors;
pub mod models;
pub mod utils;

pub use config::AppConfig;
pub use errors::DsaError;
pub use errors::DsaResult;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref GLOBAL_CONFIG: Mutex<Option<AppConfig>> = Mutex::new(None);
    static ref CONFIG_PATH: Mutex<String> = Mutex::new(String::new());
    static ref DSA_PASSWORD_OVERRIDE: Mutex<Option<String>> = Mutex::new(None);
    static ref DSA_TOKEN: Mutex<Option<String>> = Mutex::new(None);
}

/// 设置全局密码覆盖值
pub fn set_password_override(pwd: String) {
    let mut p = DSA_PASSWORD_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner());
    *p = Some(pwd);
}

/// 获取全局密码覆盖值
pub fn get_password_override() -> Option<String> {
    let p = DSA_PASSWORD_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner());
    p.clone()
}

/// 设置全局应用配置
pub fn set_global_config(conf: AppConfig) {
    let mut cfg = GLOBAL_CONFIG.lock().unwrap_or_else(|e| e.into_inner());
    *cfg = Some(conf);
}

/// 获取全局应用配置，未设置则返回默认值
pub fn get_global_config() -> AppConfig {
    let cfg = GLOBAL_CONFIG.lock().unwrap_or_else(|e| e.into_inner());
    cfg.clone().unwrap_or_default()
}

/// 设置配置文件路径
pub fn set_config_path(path: String) {
    let mut p = CONFIG_PATH.lock().unwrap_or_else(|e| e.into_inner());
    *p = path;
}

/// 获取配置文件路径，默认为 conf/config.toml
pub fn get_config_path() -> String {
    let p = CONFIG_PATH.lock().unwrap_or_else(|e| e.into_inner());
    if p.is_empty() {
        "conf/config.toml".to_string()
    } else {
        p.clone()
    }
}

/// 设置认证令牌
pub fn set_auth_token(token: String) {
    let mut t = DSA_TOKEN.lock().unwrap_or_else(|e| e.into_inner());
    *t = Some(token);
}

/// 获取当前认证令牌
pub fn get_auth_token() -> Option<String> {
    let t = DSA_TOKEN.lock().unwrap_or_else(|e| e.into_inner());
    t.clone()
}

/// 清除认证令牌
pub fn clear_auth_token() {
    let mut t = DSA_TOKEN.lock().unwrap_or_else(|e| e.into_inner());
    *t = None;
}
