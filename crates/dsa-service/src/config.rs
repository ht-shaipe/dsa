//! 全局配置缓存 - 委托给 dsa-core 全局配置

use dsa_core::config::AppConfig;

/// 设置全局应用配置
pub fn set_app_config(conf: AppConfig) {
    dsa_core::set_global_config(conf);
}

/// 获取全局应用配置
pub fn get_app_config() -> AppConfig {
    dsa_core::get_global_config()
}
