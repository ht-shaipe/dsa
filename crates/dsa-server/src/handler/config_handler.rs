//! Config handler - 读写配置

use tube::{Result, Value};
use tube_web::RequestParameter;

pub async fn distribute(param: &RequestParameter) -> Result<Value> {
    match param.method.as_str() {
        "get" => {
            let conf = dsa_core::get_global_config();
            let json = serde_json::to_value(&conf)
                .map_err(|e| error!("配置序列化失败: {}", e))?;
            Ok(json.into())
        }
        "reload" => {
            let conf_path = dsa_core::get_config_path();
            let abs_path = tube_web::utils::get_abs_path(&conf_path);
            let path = std::path::Path::new(&abs_path);
            match dsa_core::config::AppConfig::load(path) {
                Ok(conf) => {
                    dsa_core::set_global_config(conf);
                    Ok(value!({"status": "ok", "message": "配置已重新加载"}))
                }
                Err(e) => Err(error!("配置重载失败: {}", e)),
            }
        }
        _ => Err(error!("config不支持方法: {}", param.method)),
    }
}
