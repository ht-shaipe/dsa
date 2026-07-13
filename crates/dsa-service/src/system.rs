use tube::{Result, Value};
use tube_web::RequestParameter;
use ai_llm_kit::{LlmFactory, LlmProvider, LlmService};

pub struct System { request: RequestParameter }

impl System {
    pub fn new(param: &RequestParameter) -> Self { System { request: param.clone() } }
    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "get" => self.get_config().await,
            "reload" => self.reload_config().await,
            "save" => self.save_config().await,
            "validate" => self.validate_config().await,
            "export_config" => self.export_config().await,
            "import_config" => self.import_config().await,
            "test_llm" => self.test_llm().await,
            "discover_models" => self.discover_models().await,
            "llm_test" => self.llm_test().await,
            "notification_test" => self.notification_test().await,
            "config_export" => self.config_export().await,
            "config_import" => self.config_import().await,
            _ => Err(tube::Error::from(format!(
                "system不支持方法: {}",
                method
            ))),
        }
    }
    fn params(&self) -> &Value { &self.request.value }

    async fn get_config(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let json = serde_json::to_value(&conf)
            .map_err(|e| tube::Error::from(format!("配置序列化失败: {}", e)))?;
        Ok(tube::Value::from(json))
    }

    async fn reload_config(&self) -> Result<Value> {
        let conf_path = std::env::var("DSA_CONFIG_PATH")
            .unwrap_or_else(|_| "conf/config.toml".to_string());
        let path = std::path::Path::new(&conf_path);

        let conf = dsa_core::config::AppConfig::load(path).map_err(|e| tube::Error::msg(e.to_string()))?;
        dsa_core::set_global_config(conf.clone());

        let connector = conf.build_connector();
        connector.set_cache("default");

        Ok(value!({"message": "配置已重新加载"}))
    }

    async fn save_config(&self) -> Result<Value> {
        let params = self.params();
        let patch = params
            .get("config")
            .cloned()
            .unwrap_or(Value::Null);

        let current = dsa_core::get_global_config();
        let mut current_json = serde_json::to_value(&current)
            .map_err(|e| tube::Error::from(format!("当前配置序列化失败: {}", e)))?;

        let patch_json: serde_json::Value = serde_json::to_value(&patch)
            .map_err(|e| tube::Error::from(format!("补丁配置转换失败: {}", e)))?;

        merge_json(&mut current_json, &patch_json);

        let conf: dsa_core::config::AppConfig = serde_json::from_value(current_json)
            .map_err(|e| tube::Error::from(format!("合并后配置解析失败: {}", e)))?;

        let toml_str = toml::to_string_pretty(&conf)
            .map_err(|e| tube::Error::from(format!("配置序列化为TOML失败: {}", e)))?;

        let conf_path = dsa_core::get_config_path();
        std::fs::write(&conf_path, &toml_str)
            .map_err(|e| tube::Error::from(format!("写入配置文件失败 {}: {}", conf_path, e)))?;

        dsa_core::set_global_config(conf);

        Ok(value!({"message": "配置已保存"}))
    }

    async fn validate_config(&self) -> Result<Value> {
        let params = self.params();
        let config_json = params
            .get("config")
            .cloned()
            .unwrap_or(Value::Null);

        let config_str = serde_json::to_string(&config_json)
            .map_err(|e| tube::Error::from(format!("配置转换失败: {}", e)))?;

        match toml::from_str::<dsa_core::config::AppConfig>(&config_str) {
            Ok(_) => Ok(value!({"valid": true})),
            Err(e) => Ok(value!({"valid": false, "error": e.to_string()})),
        }
    }

    async fn export_config(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let toml_str = toml::to_string_pretty(&conf)
            .map_err(|e| tube::Error::from(format!("配置序列化失败: {}", e)))?;
        Ok(Value::from(toml_str))
    }

    async fn import_config(&self) -> Result<Value> {
        let params = self.params();
        let config_str = params
            .get("config")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if config_str.is_empty() {
            return Err(tube::Error::from("请提供配置内容"));
        }

        let conf: dsa_core::config::AppConfig = toml::from_str(&config_str)
            .map_err(|e| tube::Error::from(format!("配置解析失败: {}", e)))?;

        dsa_core::set_global_config(conf);

        Ok(value!({"message": "配置已导入"}))
    }

    async fn test_llm(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();

        if api_key.is_empty() {
            return Ok(value!({
                "connected": false,
                "error": "API Key 未配置",
            }));
        }

        let llm_provider = LlmProvider::instance(&conf.llm.provider);
        match llm_provider {
            Ok(provider) => {
                let llm: Box<dyn LlmService> = LlmFactory::create(provider, &api_key);

                let body = value!({
                    "model": &conf.llm.model,
                    "messages": [{"role": "user", "content": "Hello, test"}],
                    "max_tokens": 10,
                });

                match llm.chat(&body).await {
                    Ok(response) => {
                        let has_content = response
                            .get("choices")
                            .and_then(|c| tube::Value::as_array(&c.clone()))
                            .map(|a| !a.is_empty())
                            .unwrap_or(false);

                        Ok(value!({
                            "connected": true,
                            "provider": conf.llm.provider,
                            "model": conf.llm.model,
                            "hasContent": has_content,
                        }))
                    }
                    Err(e) => Ok(value!({
                        "connected": false,
                        "error": format!("LLM调用失败: {}", e),
                        "provider": conf.llm.provider,
                        "model": conf.llm.model,
                    })),
                }
            }
            Err(e) => Ok(value!({
                "connected": false,
                "error": format!("不支持的provider: {}", e),
            })),
        }
    }

    async fn discover_models(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let provider = &conf.llm.provider;

        let models = match provider.as_str() {
            "deepseek" => vec![
                value!({"id": "deepseek-chat", "name": "DeepSeek Chat"}),
                value!({"id": "deepseek-reasoner", "name": "DeepSeek Reasoner"}),
            ],
            "openai" => vec![
                value!({"id": "gpt-4o", "name": "GPT-4o"}),
                value!({"id": "gpt-4o-mini", "name": "GPT-4o Mini"}),
                value!({"id": "gpt-3.5-turbo", "name": "GPT-3.5 Turbo"}),
            ],
            "qwen" => vec![
                value!({"id": "qwen-max", "name": "Qwen Max"}),
                value!({"id": "qwen-plus", "name": "Qwen Plus"}),
                value!({"id": "qwen-turbo", "name": "Qwen Turbo"}),
            ],
            _ => {
                let model = conf.llm.model.clone();
                vec![
                    value!({"id": model.as_str(), "name": model.as_str()}),
                ]
            }
        };

        Ok(value!({
            "provider": provider,
            "models": models,
        }))
    }

    async fn llm_test(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();

        if api_key.is_empty() {
            return Ok(value!({
                "success": false,
                "error": "API Key 未配置",
                "latency_ms": 0,
            }));
        }

        let llm_provider = LlmProvider::instance(&conf.llm.provider);
        match llm_provider {
            Ok(provider) => {
                let llm: Box<dyn LlmService> = LlmFactory::create(provider, &api_key);

                let body = value!({
                    "model": &conf.llm.model,
                    "messages": [{"role": "user", "content": "Hello"}],
                    "max_tokens": 10,
                });

                let start = std::time::Instant::now();
                match llm.chat(&body).await {
                    Ok(_response) => {
                        let latency_ms = start.elapsed().as_millis() as i64;
                        Ok(value!({
                            "success": true,
                            "provider": conf.llm.provider,
                            "model": conf.llm.model,
                            "latency_ms": latency_ms,
                        }))
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis() as i64;
                        Ok(value!({
                            "success": false,
                            "error": format!("LLM调用失败: {}", e),
                            "provider": conf.llm.provider,
                            "model": conf.llm.model,
                            "latency_ms": latency_ms,
                        }))
                    }
                }
            }
            Err(e) => Ok(value!({
                "success": false,
                "error": format!("不支持的provider: {}", e),
                "latency_ms": 0,
            })),
        }
    }

    async fn notification_test(&self) -> Result<Value> {
        let params = self.params();
        let channel = params
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "log".to_string());

        let notification = crate::Notification::new(&self.request);
        notification.dispatch("send").await
    }

    async fn config_export(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let json = serde_json::to_value(&conf)
            .map_err(|e| tube::Error::from(format!("配置序列化失败: {}", e)))?;
        Ok(tube::Value::from(json))
    }

    async fn config_import(&self) -> Result<Value> {
        let params = self.params();
        let config_str = params
            .get("config")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if config_str.is_empty() {
            return Err(tube::Error::from("请提供配置JSON字符串"));
        }

        let conf: dsa_core::config::AppConfig = serde_json::from_str(&config_str)
            .map_err(|e| tube::Error::from(format!("配置JSON解析失败: {}", e)))?;

        dsa_core::set_global_config(conf);

        Ok(value!({"message": "配置已导入并更新"}))
    }
}

fn merge_json(base: &mut serde_json::Value, patch: &serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(patch_map)) => {
            for (key, patch_val) in patch_map {
                if let Some(base_val) = base_map.get_mut(key) {
                    merge_json(base_val, patch_val);
                } else {
                    base_map.insert(key.clone(), patch_val.clone());
                }
            }
        }
        (base, patch) => {
            *base = patch.clone();
        }
    }
}
