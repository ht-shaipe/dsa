use ai_llm_kit::{LlmFactory, LlmProvider, LlmService};
use log::{debug as log_debug, warn as log_warn, error as log_error, info as log_info};
use tube::{Result, Value};
use tube_web::RequestParameter;

lazy_static::lazy_static! {
    pub static ref DATA_SYNC_STATUS: std::sync::Mutex<DataSyncStatus> = std::sync::Mutex::new(DataSyncStatus::default());
    pub static ref TASK_BROADCAST: tokio::sync::broadcast::Sender<Value> = {
        let (tx, _) = tokio::sync::broadcast::channel(256);
        tx
    };
}

#[derive(Default, Clone)]
pub struct DataSyncStatus {
    pub running: bool,
    pub paused: bool,
    pub total: u32,
    pub done: u32,
    pub failed: u32,
    pub phase: String,
    pub task_name: String,
    pub current_code: String,
    pub current_name: String,
}

impl DataSyncStatus {
    pub fn to_value(&self) -> Value {
        value!({
            "task": if self.task_name.is_empty() { "init_daily_data" } else { self.task_name.as_str() },
            "running": self.running,
            "paused": self.paused,
            "total": self.total,
            "done": self.done,
            "failed": self.failed,
            "phase": self.phase.clone(),
            "current_code": self.current_code.clone(),
            "current_name": self.current_name.clone(),
        })
    }
}

pub fn broadcast_task_status() {
    let st = DATA_SYNC_STATUS.lock().unwrap();
    let val = st.to_value();
    let _ = TASK_BROADCAST.send(val);
}

pub fn parse_kline_response(
    text: &str,
    is_sqlite: bool,
    retention_date: &str,
) -> Result<Vec<dsa_core::models::KlineBar>> {
    let data_json: Value = tube::Value::from_str(text)
        .map_err(|e| tube::Error::from(format!("解析JSON失败: {}", e)))?;

    let data_obj = data_json
        .get("data")
        .ok_or_else(|| tube::Error::from("响应中缺少data字段"))?;

    let klines = data_obj
        .get("klines")
        .and_then(|v| v.as_array())
        .ok_or_else(|| tube::Error::from("响应中缺少klines字段"))?;

    let bars: Vec<dsa_core::models::KlineBar> = klines
        .iter()
        .filter_map(|v| v.as_str())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 7 {
                return None;
            }
            let date = parts[0];
            if is_sqlite && date < retention_date {
                return None;
            }
            Some(dsa_core::models::KlineBar {
                date: date.to_string(),
                open: parts[1].parse::<f64>().unwrap_or(0.0),
                high: parts[3].parse::<f64>().unwrap_or(0.0),
                low: parts[4].parse::<f64>().unwrap_or(0.0),
                close: parts[2].parse::<f64>().unwrap_or(0.0),
                volume: parts[5].parse::<f64>().unwrap_or(0.0) as i64,
                amount: parts[6].parse::<f64>().unwrap_or(0.0),
            })
        })
        .collect();

    Ok(bars)
}

pub fn wait_if_paused() -> bool {
    loop {
        {
            let st = DATA_SYNC_STATUS.lock().unwrap();
            if !st.running { return false; }
            if !st.paused { return true; }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

pub struct System {
    request: RequestParameter,
}

impl System {
    pub fn new(param: &RequestParameter) -> Self {
        System {
            request: param.clone(),
        }
    }
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
            "quick_init" => self.quick_init().await,
            "sync_status" => self.sync_status().await,
            "clean_daily_data" => self.clean_daily_data().await,
            "daily_data_stats" => self.daily_data_stats().await,
            "dashboard_stats" => self.dashboard_stats().await,
            "export_daily_data" => self.export_daily_data().await,
            "import_daily_data" => self.import_daily_data().await,
            "pause_sync" => self.pause_sync().await,
            "resume_sync" => self.resume_sync().await,
            "stop_sync" => self.stop_sync().await,
            _ => Err(tube::Error::from(format!("system不支持方法: {}", method))),
        }
    }
    fn params(&self) -> &Value {
        &self.request.value
    }

    async fn get_config(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let json = serde_json::to_value(&conf)
            .map_err(|e| tube::Error::from(format!("配置序列化失败: {}", e)))?;
        Ok(tube::Value::from(json))
    }

    async fn reload_config(&self) -> Result<Value> {
        let conf_path =
            std::env::var("DSA_CONFIG_PATH").unwrap_or_else(|_| "conf/config.toml".to_string());
        let path = std::path::Path::new(&conf_path);

        let conf =
            dsa_core::config::AppConfig::load(path).map_err(|e| tube::Error::msg(e.to_string()))?;
        dsa_core::set_global_config(conf.clone());

        let connector = conf.build_connector();
        connector.set_cache("default");

        Ok(value!({"message": "配置已重新加载"}))
    }

    async fn save_config(&self) -> Result<Value> {
        let params = self.params();
        let patch = params.get("config").cloned().unwrap_or(Value::Null);

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
        let config_json = params.get("config").cloned().unwrap_or(Value::Null);

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
        let params = self.request.value.clone();
        let conf = dsa_core::get_global_config();

        let override_provider = dsa_core::utils::param_string(&params, "provider");
        let override_api_key = dsa_core::utils::param_string(&params, "api_key");

        let provider_name = if override_provider.is_empty() {
            &conf.llm.provider
        } else {
            &override_provider
        };
        let api_key = if !override_api_key.is_empty() {
            override_api_key
        } else if provider_name == &conf.llm.provider || provider_name.is_empty() {
            conf.resolve_api_key()
        } else {
            String::new()
        };

        if api_key.is_empty() {
            return Ok(value!({
                "provider": provider_name,
                "models": [],
                "error": "API Key 未配置，无法发现模型",
            }));
        }

        let llm_provider = LlmProvider::instance(provider_name);
        match llm_provider {
            Ok(provider) => {
                let llm: Box<dyn LlmService> = LlmFactory::create(provider, &api_key);
                match llm.models().await {
                    Ok(raw) => {
                        let models = match raw.as_array() {
                            Some(arr) => arr.clone(),
                            None => {
                                if let Some(data) = raw.get("data").and_then(|v| v.as_array()) {
                                    data.clone()
                                } else {
                                    vec![]
                                }
                            }
                        };
                        let list: Vec<Value> = models.iter().filter_map(|m| {
                            let id = m.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                            if id.is_empty() {
                                return None;
                            }
                            Some(value!({
                                "id": id.clone(),
                                "name": id,
                            }))
                        }).collect();
                        Ok(value!({
                            "provider": provider_name,
                            "models": list,
                        }))
                    }
                    Err(e) => {
                        log_warn!("发现模型API调用失败: {}", e);
                        Ok(value!({
                            "provider": provider_name,
                            "models": [],
                            "error": format!("调用模型列表API失败: {}", e),
                        }))
                    }
                }
            }
            Err(e) => {
                Ok(value!({
                    "provider": provider_name,
                    "models": [],
                    "error": format!("不支持的供应商: {}", e),
                }))
            }
        }
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
        let _channel = params
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
                return Ok(
                    value!({"message": "同步已在进行中", "progress": st.done, "total": st.total}),
                );
            }
        }

        {
            let mut st = DATA_SYNC_STATUS.lock().unwrap();
            st.running = true;
            st.paused = false;
            st.total = 0;
            st.done = 0;
            st.failed = 0;
            st.phase = "preparing".to_string();
            st.task_name = "init_daily_data".to_string();
        }
        broadcast_task_status();

        let conf = dsa_core::get_global_config();
        let sync_conf = &conf.data_sync;
        let retention_days = sync_conf.retention_days as i64;

        let codes: Vec<(String, String, i8)> = {
            let connector = dsa_core::db::get_db_connector().ok();
            let pool_loaded = if let Some(ref conn) = connector {
                let sql = "SELECT stock_code, stock_name, market_id FROM stock_pool WHERE status = 1 ORDER BY market_id DESC, stock_code ASC";
                if let Ok(rows) = dsa_core::db::query_rows(sql, vec![], conn) {
                    let pool: Vec<(String, String, i8)> = rows
                        .iter()
                        .filter_map(|r| {
                            let code = dsa_core::db::row_get_string(r, "stockCode");
                            let name = dsa_core::db::row_get_string(r, "stockName");
                            let market_id = dsa_core::db::row_get_i64(r, "marketId") as i8;
                            if code.is_empty() { return None; }
                            Some((code, name, market_id))
                        })
                        .collect();
                    if !pool.is_empty() {
                        Some(pool)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            match pool_loaded {
                Some(pool) => pool,
                None => {
                    let em = qta_crawler::EastMoney::new();
                    let spot = em
                        .stock_zh_a_spot()
                        .await
                        .map_err(|e| {
                            let mut st = DATA_SYNC_STATUS.lock().unwrap();
                            st.running = false;
                            st.phase = "done".to_string();
                            broadcast_task_status();
                            tube::Error::from(format!("获取行情失败: {}", e))
                        })?;

                    let api_codes: Vec<(String, String, i8)> = spot
                        .iter()
                        .filter_map(|s| {
                            let code: String = s
                                .get("代码")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let name: String = s
                                .get("名称")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            if code.is_empty() {
                                return None;
                            }
                            let market_id = if code.starts_with('6') { 1 } else { 0 };
                            if sync_conf.should_include_code(&code, &name) {
                                Some((code, name, market_id))
                            } else {
                                None
                            }
                        })
                        .collect();

                    if let Some(ref conn) = connector {
                        let is_sqlite = conf.database.is_sqlite();
                        for (code, name, market_id) in &api_codes {
                            if is_sqlite {
                                let sql = "INSERT OR IGNORE INTO stock_pool (stock_code, stock_name, market, market_id, industry, status) \
                                           VALUES (:code, :name, 'cn', :market_id, '', 1)";
                                let params = vec![
                                    ("code".to_string(), Value::from(code.clone())),
                                    ("name".to_string(), Value::from(name.clone())),
                                    ("market_id".to_string(), Value::from(*market_id as i64)),
                                ];
                                let _ = dsa_core::db::execute(sql, params, conn);
                            } else {
                                let sql = "INSERT INTO stock_pool (stock_code, stock_name, market, market_id, industry, status) \
                                           VALUES (:code, :name, 'cn', :market_id, '', 1) \
                                           ON DUPLICATE KEY UPDATE stock_name = VALUES(stock_name), market_id = VALUES(market_id), status = 1";
                                let params = vec![
                                    ("code".to_string(), Value::from(code.clone())),
                                    ("name".to_string(), Value::from(name.clone())),
                                    ("market_id".to_string(), Value::from(*market_id as i64)),
                                ];
                                let _ = dsa_core::db::execute(sql, params, conn);
                            }
                        }
                    }

                    api_codes
                }
            }
        };

        let total = codes.len() as u32;
        {
            let mut st = DATA_SYNC_STATUS.lock().unwrap();
            st.running = true;
            st.paused = false;
            st.total = total;
            st.done = 0;
            st.failed = 0;
            st.phase = "fetching".to_string();
            st.task_name = "init_daily_data".to_string();
        }
        broadcast_task_status();

        let codes_clone = codes.clone();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime for init_daily_data");

                rt.block_on(async {
                let conf = dsa_core::get_global_config();
                let is_sqlite = conf.database.is_sqlite();
                let retention_date = {
                    let now = chrono::Local::now();
                    let cutoff = now - chrono::Duration::days(retention_days);
                    cutoff.format("%Y-%m-%d").to_string()
                };

                let kline_base_url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";

                for (code, name, market_id) in &codes_clone {
                    if !wait_if_paused() { break; }

                    {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.current_code = code.clone();
                        st.current_name = name.clone();
                    }
                    broadcast_task_status();

                    let secid = format!("{}.{}", market_id, code);
                    let params = format!(
                        "fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116&ut=7eea3edcaed734bea9cbfc24409ed989&klt=101&fqt=1&secid={}&beg=19700101&end=20500101",
                        urlencoding::encode(&secid)
                    );
                    let full_url = format!("{}?{}", kline_base_url, params);

                    let mut headers = std::collections::HashMap::new();
                    headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36".to_string());
                    headers.insert("Referer".to_string(), "https://quote.eastmoney.com/".to_string());
                    let resp = tube_net::AsyncClient::new("")
                        .add_headers(headers)
                        .timeout(15000)
                        .get(&full_url)
                        .await;

                    match resp {
                        Ok(response_text) => {
                            match parse_kline_response(&response_text, is_sqlite, &retention_date) {
                                Ok(bars) => {
                                    if bars.is_empty() {
                                            log_debug!("股票 {} {} 无K线数据", code, name);
                                    } else {
                                        log_debug!("股票 {} {} 获取到{}条K线，写入DB", code, name, bars.len());
                                        dsa_core::utils::save_all_kline_to_db(code, &bars);
                                    }
                                }
                                Err(e) => {
                                    log_warn!("解析日线数据失败 {} {}: {}", code, name, e);
                                    DATA_SYNC_STATUS.lock().unwrap().failed += 1;
                                }
                            }
                        }
                        Err(e) => {
                            log_warn!("获取日线失败 {} {}: {}", code, name, e);
                            DATA_SYNC_STATUS.lock().unwrap().failed += 1;
                        }
                    }

                    {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.done += 1;
                    }
                    broadcast_task_status();

                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                }

                {
                    let mut st = DATA_SYNC_STATUS.lock().unwrap();
                    st.phase = "calculating_indicators".to_string();
                }
                broadcast_task_status();

                let connector = match dsa_core::db::get_db_connector() {
                    Ok(c) => c,
                    Err(e) => {
                        log_error!("指标计算DB连接失败: {}", e);
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
                        log_error!("查询股票列表失败: {}", e);
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
                    if !wait_if_paused() { break; }

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
                broadcast_task_status();

                log_info!("日线数据同步完成: 获取{}只, 失败{}, 指标计算{}", 
                    total, DATA_SYNC_STATUS.lock().unwrap().failed, indicator_done);
                    }
                }

                {
                    let mut st = DATA_SYNC_STATUS.lock().unwrap();
                    st.running = false;
                    st.paused = false;
                    st.phase = "done".to_string();
                }
                broadcast_task_status();
            });
            }));

            if let Err(_) = result {
                log_error!("日线数据同步线程 panic, 已自动恢复状态");
                let mut st = DATA_SYNC_STATUS.lock().unwrap();
                st.running = false;
                st.paused = false;
                st.phase = "error".to_string();
                broadcast_task_status();
            }
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

    async fn quick_init(&self) -> Result<Value> {
        {
            let st = DATA_SYNC_STATUS.lock().unwrap();
            if st.running {
                return Ok(value!({"message": "已有任务在运行中，请等待完成后再试"}));
            }
        }

        let pool_count: i64 = {
            let connector = dsa_core::db::get_db_connector().ok();
            connector
                .and_then(|c| dsa_core::db::query_rows("SELECT COUNT(*) as cnt FROM stock_pool WHERE status = 1", vec![], &c).ok())
                .and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt")))
                .unwrap_or(0)
        };

        if pool_count > 0 {
            return Ok(value!({"message": "股票池已有数据，如需重新初始化请先清空", "poolCount": pool_count}));
        }

        {
            let mut st = DATA_SYNC_STATUS.lock().unwrap();
            st.running = true;
            st.paused = false;
            st.total = 0;
            st.done = 0;
            st.failed = 0;
            st.phase = "init_pool".to_string();
            st.task_name = "quick_init".to_string();
            st.current_code = String::new();
            st.current_name = String::new();
        }
        broadcast_task_status();

        let conf = dsa_core::get_global_config();
        let sync_conf = conf.data_sync.clone();
        let retention_days = sync_conf.retention_days as i64;

        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime for quick_init");

                rt.block_on(async {
                    // ===== Phase 1: Init Stock Pool =====
                    log::info!("快速初始化 [Phase 1]: 开始获取A股列表");

                    let mut spot_codes: Vec<crate::stock_pool::StockSpot> = Vec::new();

                    match crate::stock_pool::StockPool::fetch_stock_list_simple_pub().await {
                        Ok(items) if !items.is_empty() => {
                            log::info!("新浪API获取到 {} 条A股", items.len());
                            spot_codes = items;
                        }
                        Ok(_) => { log::warn!("新浪API返回空列表"); }
                        Err(e) => { log::warn!("新浪API失败: {}", e); }
                    }

                    if spot_codes.is_empty() {
                        log::info!("尝试东方财富爬虫作为 fallback");
                        let em = qta_crawler::EastMoney::new();
                        match tokio::time::timeout(std::time::Duration::from_secs(180), em.stock_zh_a_spot()).await {
                            Ok(Ok(full_list)) if !full_list.is_empty() => {
                                spot_codes = full_list.iter().filter_map(|s| {
                                    let code: String = s.get("代码").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let name: String = s.get("名称").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    if code.is_empty() { return None; }
                                    let market_id: i8 = if code.starts_with('6') { 1 } else { 0 };
                                    let prefix = if market_id == 1 { "sh" } else if code.starts_with('8') || code.starts_with('4') || code.starts_with("920") { "bj" } else { "sz" };
                                    Some(crate::stock_pool::StockSpot {
                                        symbol: format!("{}{}", prefix, code),
                                        code: code.clone(),
                                        name,
                                        market_id,
                                        ..Default::default()
                                    })
                                }).collect();
                            }
                            _ => { log::error!("东方财富爬虫也失败"); }
                        }
                    }

                    if spot_codes.is_empty() {
                        log::error!("快速初始化: 无法获取股票列表，请检查网络");
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.phase = "done".to_string();
                        broadcast_task_status();
                        return;
                    }

                    let boards = sync_conf.boards.clone();
                    let exclude_st = sync_conf.exclude_st;
                    let exclude_delisting = sync_conf.exclude_delisting_risk;
                    let exclude_new = sync_conf.exclude_new_stock;

                    let filtered: Vec<crate::stock_pool::StockSpot> = spot_codes
                        .iter()
                        .filter(|s| crate::stock_pool::StockPool::should_include_pub(&s.code, &s.name, &boards, exclude_st, exclude_delisting, exclude_new))
                        .cloned()
                        .collect();

                    let pool_total = filtered.len() as u32;
                    log::info!("快速初始化 [Phase 1]: 过滤后 {} 只股票，开始写入", pool_total);
                    {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.total = pool_total;
                        st.phase = "init_pool_writing".to_string();
                    }
                    broadcast_task_status();

                    let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
                    let connector = match dsa_core::db::get_db_connector() {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("快速初始化DB连接失败: {}", e);
                            let mut st = DATA_SYNC_STATUS.lock().unwrap();
                            st.running = false;
                            st.phase = "done".to_string();
                            broadcast_task_status();
                            return;
                        }
                    };

                    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                    let mut pool_done: u32 = 0;

                    for spot in filtered.chunks(100) {
                        if !wait_if_paused() { break; }
                        for s in spot {
                            {
                                let mut st = DATA_SYNC_STATUS.lock().unwrap();
                                st.current_code = s.code.clone();
                                st.current_name = s.name.clone();
                            }

                            let outstanding = if s.close > 0.0 { s.liquid_market_cap / s.close } else { 0.0 };
                            let total_shares = if s.close > 0.0 { s.total_market_cap / s.close } else { 0.0 };

                            if is_sqlite {
                                let sql = "INSERT OR IGNORE INTO stock_pool \
                                   (stock_code, stock_name, symbol, market, market_id, industry, pe, pb, outstanding, total, status) \
                                   VALUES (:code, :name, :symbol, 'cn', :market_id, '', :pe, :pb, :outstanding, :total, 1)";
                                let sql_params = vec![
                                    ("code".to_string(), Value::from(s.code.clone())),
                                    ("name".to_string(), Value::from(s.name.clone())),
                                    ("symbol".to_string(), Value::from(s.symbol.clone())),
                                    ("market_id".to_string(), Value::from(s.market_id as i64)),
                                    ("pe".to_string(), Value::from(s.pe)),
                                    ("pb".to_string(), Value::from(s.pb)),
                                    ("outstanding".to_string(), Value::from(outstanding)),
                                    ("total".to_string(), Value::from(total_shares)),
                                ];
                                let affected = dsa_core::db::execute(sql, sql_params, &connector).unwrap_or(0);
                                if affected == 0 {
                                    let upd = "UPDATE stock_pool SET stock_name=:name, symbol=:symbol, market_id=:market_id, \
                                               pe=:pe, pb=:pb, outstanding=:outstanding, total=:total, status=1 \
                                               WHERE stock_code=:code AND (stock_name != :name OR status != 1)";
                                    let upd_params = vec![
                                        ("code".to_string(), Value::from(s.code.clone())),
                                        ("name".to_string(), Value::from(s.name.clone())),
                                        ("symbol".to_string(), Value::from(s.symbol.clone())),
                                        ("market_id".to_string(), Value::from(s.market_id as i64)),
                                        ("pe".to_string(), Value::from(s.pe)),
                                        ("pb".to_string(), Value::from(s.pb)),
                                        ("outstanding".to_string(), Value::from(outstanding)),
                                        ("total".to_string(), Value::from(total_shares)),
                                    ];
                                    let _ = dsa_core::db::execute(upd, upd_params, &connector);
                                }
                                let q_sql = "INSERT OR REPLACE INTO stock_quote \
                                    (stock_code, trade_date, open, high, low, close, previous_close, change_price, change_percent, \
                                    volume, amount, turnover_ratio, total_market_cap, liquid_market_cap, pe, pb) \
                                    VALUES (:code, :today, :open, :high, :low, :close, :prev_close, :change_price, :change_percent, \
                                    :volume, :amount, :turnover_ratio, :total_mcap, :liquid_mcap, :pe, :pb)";
                                let q_params = vec![
                                    ("code".to_string(), Value::from(s.code.clone())),
                                    ("today".to_string(), Value::from(today.clone())),
                                    ("open".to_string(), Value::from(s.open)),
                                    ("high".to_string(), Value::from(s.high)),
                                    ("low".to_string(), Value::from(s.low)),
                                    ("close".to_string(), Value::from(s.close)),
                                    ("prev_close".to_string(), Value::from(s.previous_close)),
                                    ("change_price".to_string(), Value::from(s.change_price)),
                                    ("change_percent".to_string(), Value::from(s.change_percent)),
                                    ("volume".to_string(), Value::from(s.volume)),
                                    ("amount".to_string(), Value::from(s.amount)),
                                    ("turnover_ratio".to_string(), Value::from(s.turnover_ratio)),
                                    ("total_mcap".to_string(), Value::from(s.total_market_cap)),
                                    ("liquid_mcap".to_string(), Value::from(s.liquid_market_cap)),
                                    ("pe".to_string(), Value::from(s.pe)),
                                    ("pb".to_string(), Value::from(s.pb)),
                                ];
                                let _ = dsa_core::db::execute(q_sql, q_params, &connector);
                            } else {
                                let sql = "INSERT INTO stock_pool \
                                   (stock_code, stock_name, symbol, market, market_id, industry, pe, pb, outstanding, total, status) \
                                   VALUES (:code, :name, :symbol, 'cn', :market_id, '', :pe, :pb, :outstanding, :total, 1) \
                                   ON DUPLICATE KEY UPDATE stock_name=VALUES(stock_name), symbol=VALUES(symbol), \
                                   market_id=VALUES(market_id), pe=VALUES(pe), pb=VALUES(pb), \
                                   outstanding=VALUES(outstanding), total=VALUES(total), status=1";
                                let sql_params = vec![
                                    ("code".to_string(), Value::from(s.code.clone())),
                                    ("name".to_string(), Value::from(s.name.clone())),
                                    ("symbol".to_string(), Value::from(s.symbol.clone())),
                                    ("market_id".to_string(), Value::from(s.market_id as i64)),
                                    ("pe".to_string(), Value::from(s.pe)),
                                    ("pb".to_string(), Value::from(s.pb)),
                                    ("outstanding".to_string(), Value::from(outstanding)),
                                    ("total".to_string(), Value::from(total_shares)),
                                ];
                                let _ = dsa_core::db::execute(sql, sql_params, &connector);
                                let q_sql = "INSERT INTO stock_quote \
                                    (stock_code, trade_date, open, high, low, close, previous_close, change_price, change_percent, \
                                    volume, amount, turnover_ratio, total_market_cap, liquid_market_cap, pe, pb) \
                                    VALUES (:code, :today, :open, :high, :low, :close, :prev_close, :change_price, :change_percent, \
                                    :volume, :amount, :turnover_ratio, :total_mcap, :liquid_mcap, :pe, :pb) \
                                    ON DUPLICATE KEY UPDATE \
                                    close=VALUES(close), high=VALUES(high), low=VALUES(low), \
                                    open=VALUES(open), previous_close=VALUES(previous_close), change_price=VALUES(change_price), \
                                    change_percent=VALUES(change_percent), volume=VALUES(volume), amount=VALUES(amount), \
                                    turnover_ratio=VALUES(turnover_ratio), total_market_cap=VALUES(total_market_cap), \
                                    liquid_market_cap=VALUES(liquid_market_cap), pe=VALUES(pe), pb=VALUES(pb)";
                                let q_params = vec![
                                    ("code".to_string(), Value::from(s.code.clone())),
                                    ("today".to_string(), Value::from(today.clone())),
                                    ("open".to_string(), Value::from(s.open)),
                                    ("high".to_string(), Value::from(s.high)),
                                    ("low".to_string(), Value::from(s.low)),
                                    ("close".to_string(), Value::from(s.close)),
                                    ("prev_close".to_string(), Value::from(s.previous_close)),
                                    ("change_price".to_string(), Value::from(s.change_price)),
                                    ("change_percent".to_string(), Value::from(s.change_percent)),
                                    ("volume".to_string(), Value::from(s.volume)),
                                    ("amount".to_string(), Value::from(s.amount)),
                                    ("turnover_ratio".to_string(), Value::from(s.turnover_ratio)),
                                    ("total_mcap".to_string(), Value::from(s.total_market_cap)),
                                    ("liquid_mcap".to_string(), Value::from(s.liquid_market_cap)),
                                    ("pe".to_string(), Value::from(s.pe)),
                                    ("pb".to_string(), Value::from(s.pb)),
                                ];
                                let _ = dsa_core::db::execute(q_sql, q_params, &connector);
                            }

                            pool_done += 1;
                            {
                                let mut st = DATA_SYNC_STATUS.lock().unwrap();
                                st.done = pool_done;
                            }
                        }
                        broadcast_task_status();
                    }

                    log::info!("快速初始化 [Phase 1]: 股票池写入完成 {} 只", pool_done);

                    // ===== Phase 2: Init Daily Data =====
                    log::info!("快速初始化 [Phase 2]: 开始同步日线数据");

                    let codes: Vec<(String, String, i8)> = {
                        let sql = "SELECT stock_code, stock_name, market_id FROM stock_pool WHERE status = 1 ORDER BY market_id DESC, stock_code ASC";
                        match dsa_core::db::query_rows(sql, vec![], &connector) {
                            Ok(rows) => rows.iter().filter_map(|r| {
                                let code = dsa_core::db::row_get_string(r, "stockCode");
                                let name = dsa_core::db::row_get_string(r, "stockName");
                                let market_id = dsa_core::db::row_get_i64(r, "marketId") as i8;
                                if code.is_empty() { return None; }
                                Some((code, name, market_id))
                            }).collect(),
                            Err(_) => Vec::new(),
                        }
                    };

                    if codes.is_empty() {
                        log::error!("快速初始化 [Phase 2]: 股票池为空，无法同步日线");
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.phase = "done".to_string();
                        broadcast_task_status();
                        return;
                    }

                    let daily_total = codes.len() as u32;
                    {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.total = daily_total;
                        st.done = 0;
                        st.failed = 0;
                        st.phase = "init_daily_data".to_string();
                        st.current_code = String::new();
                        st.current_name = String::new();
                    }
                    broadcast_task_status();

                    let is_sqlite2 = dsa_core::get_global_config().database.is_sqlite();
                    let retention_date = {
                        let now = chrono::Local::now();
                        let cutoff = now - chrono::Duration::days(retention_days);
                        cutoff.format("%Y-%m-%d").to_string()
                    };
                    let kline_base_url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";

                    for (code, name, market_id) in &codes {
                        if !wait_if_paused() { break; }

                        {
                            let mut st = DATA_SYNC_STATUS.lock().unwrap();
                            st.current_code = code.clone();
                            st.current_name = name.clone();
                        }
                        broadcast_task_status();

                        let secid = format!("{}.{}", market_id, code);
                        let params = format!(
                            "fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f116&ut=7eea3edcaed734bea9cbfc24409ed989&klt=101&fqt=1&secid={}&beg=19700101&end=20500101",
                            urlencoding::encode(&secid)
                        );
                        let full_url = format!("{}?{}", kline_base_url, params);

                        let mut headers = std::collections::HashMap::new();
                        headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36".to_string());
                        headers.insert("Referer".to_string(), "https://quote.eastmoney.com/".to_string());
                        let resp = tube_net::AsyncClient::new("")
                            .add_headers(headers)
                            .timeout(15000)
                            .get(&full_url)
                            .await;

                        match resp {
                            Ok(response_text) => {
                                match parse_kline_response(&response_text, is_sqlite2, &retention_date) {
                                    Ok(bars) => {
                                        if !bars.is_empty() {
                                            dsa_core::utils::save_all_kline_to_db(code, &bars);
                                        }
                                    }
                                    Err(_) => { DATA_SYNC_STATUS.lock().unwrap().failed += 1; }
                                }
                            }
                            Err(_) => { DATA_SYNC_STATUS.lock().unwrap().failed += 1; }
                        }

                        {
                            let mut st = DATA_SYNC_STATUS.lock().unwrap();
                            st.done += 1;
                        }
                        broadcast_task_status();

                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    }

                    // ===== Phase 2b: Calculate Indicators =====
                    {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.phase = "calculating_indicators".to_string();
                    }
                    broadcast_task_status();

                    let indicator_connector = match dsa_core::db::get_db_connector() {
                        Ok(c) => c,
                        Err(_) => {
                            let mut st = DATA_SYNC_STATUS.lock().unwrap();
                            st.running = false;
                            st.phase = "done".to_string();
                            broadcast_task_status();
                            return;
                        }
                    };

                    let sql = "SELECT DISTINCT stock_code FROM stock_daily WHERE status = 1 ORDER BY stock_code";
                    let rows = match dsa_core::db::query_rows(sql, vec![], &indicator_connector) {
                        Ok(r) => r,
                        Err(_) => {
                            let mut st = DATA_SYNC_STATUS.lock().unwrap();
                            st.running = false;
                            st.phase = "done".to_string();
                            broadcast_task_status();
                            return;
                        }
                    };

                    let analyzer = dsa_pipeline::technical::TechnicalAnalyzer::new();
                    let mut indicator_done = 0u32;
                    let indicator_total = rows.len() as u32;

                    for row in &rows {
                        if !wait_if_paused() { break; }
                        let code = dsa_core::db::row_get_string(row, "stockCode");
                        if code.is_empty() { continue; }

                        let hist_sql = "SELECT close, trade_date FROM stock_daily \
                             WHERE stock_code = :code AND status >= 1 ORDER BY trade_date ASC";
                        let hist_rows = match dsa_core::db::query_rows(
                            hist_sql,
                            vec![("code".to_string(), Value::from(code.clone()))],
                            &indicator_connector,
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
                            &indicator_connector,
                        );

                        indicator_done += 1;
                        if indicator_done % 50 == 0 {
                            let mut st = DATA_SYNC_STATUS.lock().unwrap();
                            st.phase = format!("calculating_indicators ({}/{})", indicator_done, indicator_total);
                            broadcast_task_status();
                        }
                    }

                    {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.paused = false;
                        st.phase = "done".to_string();
                        st.current_code = String::new();
                        st.current_name = String::new();
                    }
                    broadcast_task_status();

                    log::info!("快速初始化完成: 股票池{}只, 日线同步{}, 指标计算{}", pool_done, daily_total, indicator_done);
                });
            }));

            if let Err(_) = result {
                log::error!("快速初始化线程 panic");
                let mut st = DATA_SYNC_STATUS.lock().unwrap();
                st.running = false;
                st.paused = false;
                st.phase = "error".to_string();
                broadcast_task_status();
            }
        });

        Ok(value!({"message": "快速初始化已启动"}))
    }

    async fn sync_status(&self) -> Result<Value> {
        let st = DATA_SYNC_STATUS.lock().unwrap();
        let conf = dsa_core::get_global_config();
        Ok(value!({
            "running": st.running,
            "paused": st.paused,
            "total": st.total,
            "done": st.done,
            "failed": st.failed,
            "phase": st.phase.clone(),
            "task": st.task_name.clone(),
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
                format!(
                    "DELETE FROM stock_daily WHERE trade_date < '{}'",
                    cutoff_date
                ),
                format!(
                    "SELECT COUNT(*) as cnt FROM stock_daily WHERE trade_date < '{}'",
                    cutoff_date
                ),
            )
        } else {
            (
                format!("DELETE FROM stock_daily WHERE trade_date < DATE_SUB(CURDATE(), INTERVAL {} DAY)", retention_days),
                format!("SELECT COUNT(*) as cnt FROM stock_daily WHERE trade_date < DATE_SUB(CURDATE(), INTERVAL {} DAY)", retention_days),
            )
        };

        let count_before = dsa_core::db::query_rows(&count_sql, vec![], &connector)
            .ok()
            .and_then(|rows| {
                rows.first()
                    .map(|r| dsa_core::db::row_get_f64(r, "cnt") as i64)
            })
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

    async fn daily_data_stats(&self) -> Result<Value> {
        let connector = dsa_core::db::get_db_connector()
            .map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let stock_count_sql =
            "SELECT COUNT(DISTINCT stock_code) AS cnt FROM stock_daily WHERE status >= 1";
        let total_count_sql = "SELECT COUNT(*) AS cnt FROM stock_daily WHERE status >= 1";

        let stock_count: i64 = dsa_core::db::query_rows(stock_count_sql, vec![], &connector)
            .ok()
            .and_then(|rows| {
                rows.first()
                    .map(|r| dsa_core::db::row_get_f64(r, "cnt") as i64)
            })
            .unwrap_or(0);

        let total_count: i64 = dsa_core::db::query_rows(total_count_sql, vec![], &connector)
            .ok()
            .and_then(|rows| {
                rows.first()
                    .map(|r| dsa_core::db::row_get_f64(r, "cnt") as i64)
            })
            .unwrap_or(0);

        Ok(value!({
            "stockCount": stock_count,
            "totalCount": total_count,
        }))
    }

    async fn dashboard_stats(&self) -> Result<Value> {
        let connector = dsa_core::db::get_db_connector()
            .map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let daily_stock_count = dsa_core::db::query_rows(
            "SELECT COUNT(DISTINCT stock_code) AS cnt FROM stock_daily WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let daily_total_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM stock_daily WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let daily_latest = dsa_core::db::query_rows(
            "SELECT MAX(trade_date) AS val FROM stock_daily WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_string(row, "val"))).unwrap_or_default();

        let pool_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM stock_pool WHERE status = 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let watchlist_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM watchlist_stocks WHERE enabled = 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let position_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM portfolio_positions WHERE status >= 1 AND quantity > 0", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let alert_rule_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM alert_rules WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let alert_triggered = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM alert_triggers WHERE DATE(create_time) = DATE('now')", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let decision_total = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM decision_signals WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let decision_bullish = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM decision_signals WHERE status >= 1 AND action IN ('buy','add','strong_buy')", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let decision_bearish = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM decision_signals WHERE status >= 1 AND action IN ('sell','reduce','avoid','strong_sell')", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let decision_avg_score = dsa_core::db::query_rows(
            "SELECT AVG(sentiment_score) AS val FROM decision_signals WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_f64(row, "val"))).unwrap_or(0.0);

        let analysis_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM analysis_history WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let backtest_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM backtest_results WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let backtest_win_rate = dsa_core::db::query_rows(
            "SELECT CASE WHEN COUNT(*) > 0 THEN CAST(SUM(CASE WHEN outcome='win' THEN 1 ELSE 0 END) AS REAL) / COUNT(*) * 100 ELSE 0 END AS val FROM backtest_results WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_f64(row, "val"))).unwrap_or(0.0);

        let intel_source_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM intelligence_sources WHERE enabled = 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let intel_item_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM intelligence_items WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let llm_today_calls = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM llm_usage WHERE DATE(create_time) = DATE('now')", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let llm_total_tokens = dsa_core::db::query_rows(
            "SELECT COALESCE(SUM(total_tokens), 0) AS val FROM llm_usage", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "val"))).unwrap_or(0);

        let news_count = dsa_core::db::query_rows(
            "SELECT COUNT(*) AS cnt FROM news_intel WHERE status >= 1", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_i64(row, "cnt"))).unwrap_or(0);

        let portfolio_total_value = dsa_core::db::query_rows(
            "SELECT COALESCE(SUM(quantity * current_price), 0) AS val FROM portfolio_positions WHERE status >= 1 AND quantity > 0", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_f64(row, "val"))).unwrap_or(0.0);

        let portfolio_total_pnl = dsa_core::db::query_rows(
            "SELECT COALESCE(SUM(unrealized_pnl), 0) AS val FROM portfolio_positions WHERE status >= 1 AND quantity > 0", vec![], &connector,
        ).ok().and_then(|r| r.first().map(|row| dsa_core::db::row_get_f64(row, "val"))).unwrap_or(0.0);

        let sync_running = {
            let st = DATA_SYNC_STATUS.lock().unwrap();
            st.running
        };

        Ok(value!({
            "daily": {
                "stockCount": daily_stock_count,
                "totalCount": daily_total_count,
                "latestDate": daily_latest,
            },
            "pool": {
                "count": pool_count,
            },
            "watchlist": {
                "count": watchlist_count,
            },
            "portfolio": {
                "positionCount": position_count,
                "totalValue": portfolio_total_value,
                "totalPnl": portfolio_total_pnl,
            },
            "alert": {
                "ruleCount": alert_rule_count,
                "todayTriggered": alert_triggered,
            },
            "decision": {
                "total": decision_total,
                "bullish": decision_bullish,
                "bearish": decision_bearish,
                "avgScore": decision_avg_score,
            },
            "analysis": {
                "count": analysis_count,
            },
            "backtest": {
                "count": backtest_count,
                "winRate": backtest_win_rate,
            },
            "intelligence": {
                "sourceCount": intel_source_count,
                "itemCount": intel_item_count,
            },
            "llm": {
                "todayCalls": llm_today_calls,
                "totalTokens": llm_total_tokens,
            },
            "news": {
                "count": news_count,
            },
            "sync": {
                "running": sync_running,
            },
        }))
    }

    async fn export_daily_data(&self) -> Result<Value> {
        let connector = dsa_core::db::get_db_connector()
            .map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let sql = "SELECT stock_code, stock_name, trade_date, open, high, low, close, \
                   volume, amount, pct_chg, ma5, ma10, ma20, ma60, dif, dea, macd_hist, \
                   volume_ratio, turnover_rate \
                   FROM stock_daily WHERE status >= 1 ORDER BY stock_code, trade_date";

        let rows = dsa_core::db::query_rows(sql, vec![], &connector)
            .map_err(|e| tube::Error::from(format!("查询日线数据失败: {}", e)))?;

        let mut records: Vec<Value> = Vec::with_capacity(rows.len());
        for row in &rows {
            let trade_date_raw = dsa_core::db::row_get_string(row, "tradeDate");
            let trade_date = if trade_date_raw.len() > 10 {
                trade_date_raw[..10].to_string()
            } else {
                trade_date_raw
            };
            records.push(value!({
                "c": dsa_core::db::row_get_string(row, "stockCode"),
                "n": dsa_core::db::row_get_string(row, "stockName"),
                "d": trade_date,
                "o": dsa_core::db::row_get_f64(row, "open"),
                "h": dsa_core::db::row_get_f64(row, "high"),
                "l": dsa_core::db::row_get_f64(row, "low"),
                "cl": dsa_core::db::row_get_f64(row, "close"),
                "v": dsa_core::db::row_get_f64(row, "volume") as i64,
                "a": dsa_core::db::row_get_f64(row, "amount"),
                "pc": dsa_core::db::row_get_f64(row, "pctChg"),
                "m5": dsa_core::db::row_get_f64(row, "ma5"),
                "m10": dsa_core::db::row_get_f64(row, "ma10"),
                "m20": dsa_core::db::row_get_f64(row, "ma20"),
                "m60": dsa_core::db::row_get_f64(row, "ma60"),
                "df": dsa_core::db::row_get_f64(row, "dif"),
                "de": dsa_core::db::row_get_f64(row, "dea"),
                "mh": dsa_core::db::row_get_f64(row, "macdHist"),
                "vr": dsa_core::db::row_get_f64(row, "volumeRatio"),
                "tr": dsa_core::db::row_get_f64(row, "turnoverRate"),
            }));
        }

        let count = records.len() as i64;
        let stock_count = dsa_core::db::query_rows(
            "SELECT COUNT(DISTINCT stock_code) AS cnt FROM stock_daily WHERE status >= 1",
            vec![],
            &connector,
        )
        .ok()
        .and_then(|rows| {
            rows.first()
                .map(|r| dsa_core::db::row_get_f64(r, "cnt") as i64)
        })
        .unwrap_or(0);

        Ok(value!({
            "version": "1.0",
            "exportTime": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            "stockCount": stock_count,
            "recordCount": count,
            "records": Value::Array(records),
        }))
    }

    async fn import_daily_data(&self) -> Result<Value> {
        let params = self.params();
        let data = params
            .get("data")
            .ok_or_else(|| tube::Error::msg("缺少data字段"))?;

        let records = data
            .get("records")
            .and_then(|v| v.as_array())
            .ok_or_else(|| tube::Error::msg("缺少records数组"))?;

        if records.is_empty() {
            return Ok(value!({"imported": 0, "skipped": 0}));
        }

        let connector = dsa_core::db::get_db_connector()
            .map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;
        let is_sqlite = dsa_core::get_global_config().database.is_sqlite();
        let now_expr = if is_sqlite {
            "datetime('now')"
        } else {
            "NOW()"
        };

        let sql = if is_sqlite {
            format!(
                "INSERT INTO stock_daily \
                 (stock_code, stock_name, trade_date, open, high, low, close, volume, amount, \
                  pct_chg, ma5, ma10, ma20, ma60, dif, dea, macd_hist, volume_ratio, turnover_rate, status, create_time) \
                 VALUES (:code, :name, :date, :open, :high, :low, :close, :vol, :amt, \
                  :pct, :m5, :m10, :m20, :m60, :df, :de, :mh, :vr, :tr, 1, {}) \
                 ON CONFLICT(stock_code, trade_date) DO UPDATE SET \
                 stock_name=CASE WHEN excluded.stock_name != '' THEN excluded.stock_name ELSE stock_daily.stock_name END, \
                 open=excluded.open, high=excluded.high, low=excluded.low, close=excluded.close, \
                 volume=excluded.volume, amount=excluded.amount, pct_chg=excluded.pct_chg, \
                 ma5=excluded.ma5, ma10=excluded.ma10, ma20=excluded.ma20, ma60=excluded.ma60, \
                 dif=excluded.dif, dea=excluded.dea, macd_hist=excluded.macd_hist, \
                 volume_ratio=excluded.volume_ratio, turnover_rate=excluded.turnover_rate",
                now_expr
            )
        } else {
            format!(
                "INSERT INTO stock_daily \
                 (stock_code, stock_name, trade_date, open, high, low, close, volume, amount, \
                  pct_chg, ma5, ma10, ma20, ma60, dif, dea, macd_hist, volume_ratio, turnover_rate, status, create_time) \
                 VALUES (:code, :name, :date, :open, :high, :low, :close, :vol, :amt, \
                  :pct, :m5, :m10, :m20, :m60, :df, :de, :mh, :vr, :tr, 1, {}) \
                 ON DUPLICATE KEY UPDATE \
                 stock_name=IF(VALUES(stock_name)!='',VALUES(stock_name),stock_name), \
                 open=VALUES(open), high=VALUES(high), low=VALUES(low), close=VALUES(close), \
                 volume=VALUES(volume), amount=VALUES(amount), pct_chg=VALUES(pct_chg), \
                 ma5=VALUES(ma5), ma10=VALUES(ma10), ma20=VALUES(ma20), ma60=VALUES(ma60), \
                 dif=VALUES(dif), dea=VALUES(dea), macd_hist=VALUES(macd_hist), \
                 volume_ratio=VALUES(volume_ratio), turnover_rate=VALUES(turnover_rate)",
                now_expr
            )
        };

        let mut imported = 0u32;
        let mut skipped = 0u32;

        for rec in records {
            let code = rec.get("c").and_then(|v| v.as_str()).unwrap_or_default();
            let name = rec.get("n").and_then(|v| v.as_str()).unwrap_or_default();
            let date = rec.get("d").and_then(|v| v.as_str()).unwrap_or_default();
            if code.is_empty() || date.is_empty() {
                skipped += 1;
                continue;
            }

            let result = dsa_core::db::execute(
                &sql,
                vec![
                    ("code".to_string(), Value::from(code.to_string())),
                    ("name".to_string(), Value::from(name.to_string())),
                    ("date".to_string(), Value::from(date.to_string())),
                    (
                        "open".to_string(),
                        Value::from(rec.get("o").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "high".to_string(),
                        Value::from(rec.get("h").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "low".to_string(),
                        Value::from(rec.get("l").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "close".to_string(),
                        Value::from(rec.get("cl").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "vol".to_string(),
                        Value::from(rec.get("v").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64),
                    ),
                    (
                        "amt".to_string(),
                        Value::from(rec.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "pct".to_string(),
                        Value::from(rec.get("pc").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "m5".to_string(),
                        Value::from(rec.get("m5").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "m10".to_string(),
                        Value::from(rec.get("m10").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "m20".to_string(),
                        Value::from(rec.get("m20").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "m60".to_string(),
                        Value::from(rec.get("m60").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "df".to_string(),
                        Value::from(rec.get("df").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "de".to_string(),
                        Value::from(rec.get("de").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "mh".to_string(),
                        Value::from(rec.get("mh").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "vr".to_string(),
                        Value::from(rec.get("vr").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                    (
                        "tr".to_string(),
                        Value::from(rec.get("tr").and_then(|v| v.as_f64()).unwrap_or(0.0)),
                    ),
                ],
                &connector,
            );

            match result {
                Ok(_) => imported += 1,
                Err(_) => skipped += 1,
            }
        }

        tracing::info!("日线数据导入完成: 导入{}, 跳过{}", imported, skipped);

        Ok(value!({
            "imported": imported as i64,
            "skipped": skipped as i64,
        }))
    }
    async fn pause_sync(&self) -> Result<Value> {
        {
            let mut st = DATA_SYNC_STATUS.lock().unwrap();
            if !st.running {
                return Ok(value!({"message": "没有正在运行的任务"}));
            }
            if st.paused {
                return Ok(value!({"message": "任务已处于暂停状态"}));
            }
            st.paused = true;
        }
        broadcast_task_status();
        Ok(value!({"message": "任务已暂停"}))
    }

    async fn resume_sync(&self) -> Result<Value> {
        {
            let mut st = DATA_SYNC_STATUS.lock().unwrap();
            if !st.running {
                return Ok(value!({"message": "没有正在运行的任务"}));
            }
            if !st.paused {
                return Ok(value!({"message": "任务未暂停"}));
            }
            st.paused = false;
        }
        broadcast_task_status();
        Ok(value!({"message": "任务已继续"}))
    }

    async fn stop_sync(&self) -> Result<Value> {
        {
            let mut st = DATA_SYNC_STATUS.lock().unwrap();
            if !st.running {
                return Ok(value!({"message": "没有正在运行的任务"}));
            }
            st.running = false;
            st.paused = false;
        }
        broadcast_task_status();
        Ok(value!({"message": "任务已停止"}))
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
