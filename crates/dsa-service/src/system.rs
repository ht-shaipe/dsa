use tube::{Result, Value};
use tube_web::RequestParameter;
use ai_llm_kit::{LlmFactory, LlmProvider, LlmService};

lazy_static::lazy_static! {
    static ref DATA_SYNC_STATUS: std::sync::Mutex<DataSyncStatus> = std::sync::Mutex::new(DataSyncStatus::default());
}

#[derive(Default)]
struct DataSyncStatus {
    running: bool,
    total: u32,
    done: u32,
    failed: u32,
    phase: String,
}

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
            "init_daily_data" => self.init_daily_data().await,
            "sync_status" => self.sync_status().await,
            "clean_daily_data" => self.clean_daily_data().await,
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

    async fn init_daily_data(&self) -> Result<Value> {
        {
            let st = DATA_SYNC_STATUS.lock().unwrap();
            if st.running {
                return Ok(value!({"message": "同步已在进行中", "progress": st.done, "total": st.total}));
            }
        }

        let conf = dsa_core::get_global_config();
        let sync_conf = &conf.data_sync;
        let retention_days = sync_conf.retention_days as i64;

        let em = qta_crawler::EastMoney::new();
        let spot = em.stock_zh_a_spot().await
            .map_err(|e| tube::Error::from(format!("获取行情失败: {}", e)))?;

        let codes: Vec<(String, String)> = spot.iter()
            .filter_map(|s| {
                let code: String = s.get("代码").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let name: String = s.get("名称").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                if code.is_empty() { return None; }
                if sync_conf.should_include_code(&code, &name) {
                    Some((code, name))
                } else {
                    None
                }
            })
            .collect();

        let total = codes.len() as u32;
        {
            let mut st = DATA_SYNC_STATUS.lock().unwrap();
            st.running = true;
            st.total = total;
            st.done = 0;
            st.failed = 0;
            st.phase = "fetching".to_string();
        }

        let codes_clone = codes.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime for init_daily_data");

            rt.block_on(async {
                let em = qta_crawler::EastMoney::new();
                let conf = dsa_core::get_global_config();
                let is_sqlite = conf.database.is_sqlite();
                let retention_date = {
                    let now = chrono::Local::now();
                    let cutoff = now - chrono::Duration::days(retention_days);
                    cutoff.format("%Y-%m-%d").to_string()
                };

                for (code, _name) in &codes_clone {
                    if !DATA_SYNC_STATUS.lock().unwrap().running { break; }

                    let start_date = if is_sqlite {
                        format!(" AND trade_date >= '{}'", retention_date)
                    } else {
                        String::new()
                    };

                    match em.stock_zh_a_hist(code, Some("daily"), None, None, Some("qfq")).await {
                        Ok(raw) => {
                            let bars: Vec<dsa_core::models::KlineBar> = raw.iter()
                                .filter_map(|item| {
                                    let date = item.get("日期").and_then(|v| v.as_str()).unwrap_or_default();
                                    let keep = !is_sqlite || date.as_str() >= retention_date.as_str();
                                    if keep {
                                        Some(dsa_core::models::KlineBar {
                                            date: date.to_string(),
                                            open: item.get("开盘").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                            high: item.get("最高").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                            low: item.get("最低").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                            close: item.get("收盘").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                            volume: item.get("成交量").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
                                            amount: item.get("成交额").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            if !bars.is_empty() {
                                dsa_core::utils::save_all_kline_to_db(code, &bars);
                            }
                        }
                        Err(_) => {
                            DATA_SYNC_STATUS.lock().unwrap().failed += 1;
                        }
                    }

                    {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.done += 1;
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                }

                {
                    let mut st = DATA_SYNC_STATUS.lock().unwrap();
                    st.phase = "calculating_indicators".to_string();
                }

                let connector = match dsa_core::db::get_db_connector() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("指标计算DB连接失败: {}", e);
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.phase = "done".to_string();
                        return;
                    }
                };

                let sql = "SELECT DISTINCT stock_code FROM stock_daily WHERE status = 1 ORDER BY stock_code";
                let rows = match dsa_core::db::query_rows(sql, vec![], &connector) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("查询股票列表失败: {}", e);
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.phase = "done".to_string();
                        return;
                    }
                };

                let analyzer = dsa_pipeline::technical::TechnicalAnalyzer::new();
                let mut indicator_done = 0u32;
                let indicator_total = rows.len() as u32;

                for row in &rows {
                    if !DATA_SYNC_STATUS.lock().unwrap().running { break; }

                    let code = dsa_core::db::row_get_string(row, "stockCode");
                    if code.is_empty() { continue; }

                    let hist_sql = "SELECT close, trade_date FROM stock_daily \
                         WHERE stock_code = :code AND status >= 1 ORDER BY trade_date ASC";
                    let hist_rows = match dsa_core::db::query_rows(
                        hist_sql,
                        vec![("code".to_string(), Value::from(code.clone()))],
                        &connector,
                    ) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };

                    let closes: Vec<f64> = hist_rows.iter()
                        .map(|r| dsa_core::db::row_get_f64(r, "close"))
                        .collect();

                    if closes.len() < 60 { continue; }

                    let ma5 = analyzer.sma(&closes, 5);
                    let ma10 = analyzer.sma(&closes, 10);
                    let ma20 = analyzer.sma(&closes, 20);
                    let ma60 = analyzer.sma(&closes, 60);
                    let (dif, dea, macd_hist) = analyzer.macd(&closes, 12, 26, 9);

                    let last_date_row = hist_rows.last().unwrap();
                    let last_date = dsa_core::db::row_get_string(last_date_row, "tradeDate");

                    let _ = dsa_core::db::execute(
                        "UPDATE stock_daily SET \
                         ma5 = :ma5, ma10 = :ma10, ma20 = :ma20, ma60 = :ma60, \
                         dif = :dif, dea = :dea, macd_hist = :macd_hist \
                         WHERE stock_code = :code AND trade_date = :date AND status >= 1",
                        vec![
                            ("ma5".to_string(), Value::from(ma5)),
                            ("ma10".to_string(), Value::from(ma10)),
                            ("ma20".to_string(), Value::from(ma20)),
                            ("ma60".to_string(), Value::from(ma60)),
                            ("dif".to_string(), Value::from(dif)),
                            ("dea".to_string(), Value::from(dea)),
                            ("macd_hist".to_string(), Value::from(macd_hist)),
                            ("code".to_string(), Value::from(code.clone())),
                            ("date".to_string(), Value::from(last_date.clone())),
                        ],
                        &connector,
                    );

                    indicator_done += 1;
                    if indicator_done % 50 == 0 {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.phase = format!("calculating_indicators ({}/{})", indicator_done, indicator_total);
                    }
                }

                {
                    let mut st = DATA_SYNC_STATUS.lock().unwrap();
                    st.running = false;
                    st.phase = "done".to_string();
                }

                tracing::info!("日线数据同步完成: 获取{}只, 失败{}, 指标计算{}", 
                    total, DATA_SYNC_STATUS.lock().unwrap().failed, indicator_done);
            });
        });

        Ok(value!({
            "message": "日线数据同步已启动",
            "total": total,
            "config": {
                "boards": sync_conf.boards.clone(),
                "excludeSt": sync_conf.exclude_st,
                "excludeNewStock": sync_conf.exclude_new_stock,
                "retentionDays": sync_conf.retention_days,
            }
        }))
    }

    async fn sync_status(&self) -> Result<Value> {
        let st = DATA_SYNC_STATUS.lock().unwrap();
        let conf = dsa_core::get_global_config();
        Ok(value!({
            "running": st.running,
            "total": st.total,
            "done": st.done,
            "failed": st.failed,
            "phase": st.phase.clone(),
            "config": {
                "boards": conf.data_sync.boards.clone(),
                "excludeSt": conf.data_sync.exclude_st,
                "excludeNewStock": conf.data_sync.exclude_new_stock,
                "excludeDelistingRisk": conf.data_sync.exclude_delisting_risk,
                "retentionDays": conf.data_sync.retention_days,
            }
        }))
    }

    async fn clean_daily_data(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let retention_days = conf.data_sync.retention_days;
        let connector = dsa_core::db::get_db_connector()
            .map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let is_sqlite = conf.database.is_sqlite();
        let cutoff_date = {
            let now = chrono::Local::now();
            let cutoff = now - chrono::Duration::days(retention_days as i64);
            cutoff.format("%Y-%m-%d").to_string()
        };

        let (clean_sql, count_sql) = if is_sqlite {
            (
                format!("DELETE FROM stock_daily WHERE trade_date < '{}'", cutoff_date),
                format!("SELECT COUNT(*) as cnt FROM stock_daily WHERE trade_date < '{}'", cutoff_date),
            )
        } else {
            (
                format!("DELETE FROM stock_daily WHERE trade_date < DATE_SUB(CURDATE(), INTERVAL {} DAY)", retention_days),
                format!("SELECT COUNT(*) as cnt FROM stock_daily WHERE trade_date < DATE_SUB(CURDATE(), INTERVAL {} DAY)", retention_days),
            )
        };

        let count_before = dsa_core::db::query_rows(&count_sql, vec![], &connector)
            .ok()
            .and_then(|rows| rows.first().map(|r| dsa_core::db::row_get_f64(r, "cnt") as i64))
            .unwrap_or(-1);

        let result = dsa_core::db::execute(&clean_sql, vec![], &connector)
            .map_err(|e| tube::Error::from(format!("清理数据失败: {}", e)))?;

        Ok(value!({
            "deleted": result as i64,
            "wouldDelete": count_before,
            "retentionDays": retention_days,
            "cutoffDate": cutoff_date,
        }))
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
