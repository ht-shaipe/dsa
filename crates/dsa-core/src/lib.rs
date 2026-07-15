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
}

/// 设置全局应用配置
pub fn set_global_config(conf: AppConfig) {
    apply_proxy_env(&conf.proxy);
    let mut cfg = GLOBAL_CONFIG.lock().unwrap_or_else(|e| e.into_inner());
    *cfg = Some(conf);
}

fn apply_proxy_env(proxy: &config::ProxyConfig) {
    if !proxy.http_proxy.is_empty() && std::env::var("http_proxy").unwrap_or_default().is_empty() {
        std::env::set_var("http_proxy", &proxy.http_proxy);
    }
    if !proxy.https_proxy.is_empty() && std::env::var("https_proxy").unwrap_or_default().is_empty()
    {
        std::env::set_var("https_proxy", &proxy.https_proxy);
    }
    if !proxy.no_proxy.is_empty() {
        let existing = std::env::var("no_proxy").unwrap_or_default();
        let merged = if existing.is_empty() {
            proxy.no_proxy.clone()
        } else {
            format!("{},{}", existing, proxy.no_proxy)
        };
        std::env::set_var("no_proxy", &merged);
        std::env::set_var("NO_PROXY", &merged);
    }
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
