use dsa_core::db::{
    execute, get_db_connector, query_rows, row_get_f64, row_get_string, row_get_value,
};
use dsa_core::utils;
use dsa_pipeline::technical::TechnicalAnalyzer;
use log::{debug as log_debug, warn as log_warn, error as log_error};
use qta_crawler::EastMoney;
use tube::{Result, Value};
use tube_web::RequestParameter;

lazy_static::lazy_static! {
    pub static ref SYNC_STATUS: std::sync::Mutex<SyncStatus> = std::sync::Mutex::new(SyncStatus::default());
}

#[derive(Default, Clone)]
pub struct SyncStatus {
    pub running: bool,
    pub paused: bool,
    pub total: u32,
    pub done: u32,
    pub failed: u32,
    pub phase: String,
}

impl SyncStatus {
    pub fn to_value(&self) -> Value {
        value!({
            "task": "sync_daily",
            "running": self.running,
            "paused": self.paused,
            "total": self.total,
            "done": self.done,
            "failed": self.failed,
            "phase": self.phase.clone(),
        })
    }
}

fn broadcast_screening_status() {
    let st = SYNC_STATUS.lock().unwrap();
    let val = st.to_value();
    let _ = crate::system::TASK_BROADCAST.send(val);
}

fn wait_if_screening_paused() -> bool {
    loop {
        {
            let st = SYNC_STATUS.lock().unwrap();
            if !st.running { return false; }
            if !st.paused { return true; }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

#[derive(Debug, Clone)]
struct MacdParams {
    lookback: usize,
    hist_lookback: usize,
    dif_threshold: f64,
    dea_threshold: f64,
    ma_period: String,
}

impl Default for MacdParams {
    fn default() -> Self {
        Self {
            lookback: 5,
            hist_lookback: 10,
            dif_threshold: 0.0,
            dea_threshold: 0.0,
            ma_period: "ma60".to_string(),
        }
    }
}

impl MacdParams {
    fn from_value(v: &Value) -> Self {
        Self {
            lookback: v.get("lookback").and_then(|x| x.as_u64()).unwrap_or(5) as usize,
            hist_lookback: v.get("hist_lookback").and_then(|x| x.as_u64()).unwrap_or(10) as usize,
            dif_threshold: v.get("dif_threshold").and_then(|x| x.as_f64()).unwrap_or(0.0),
            dea_threshold: v.get("dea_threshold").and_then(|x| x.as_f64()).unwrap_or(0.0),
            ma_period: v.get("ma_period").and_then(|x| x.as_str()).map(|s| s.to_string()).unwrap_or_else(|| "ma60".to_string()),
        }
    }

    fn to_value(&self) -> Value {
        value!({
            "lookback": self.lookback as i64,
            "hist_lookback": self.hist_lookback as i64,
            "dif_threshold": self.dif_threshold,
            "dea_threshold": self.dea_threshold,
            "ma_period": self.ma_period.clone(),
        })
    }

    fn to_json_string(&self) -> String {
        serde_json::to_string(&self.to_value()).unwrap_or_else(|_| "{}".to_string())
    }
}

pub struct Screening {
    request: RequestParameter,
}

impl Screening {
    pub fn new(param: &RequestParameter) -> Self {
        Screening {
            request: param.clone(),
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "status" => self.status().await,
            "strategies" => self.strategies().await,
            "hotspots" => self.hotspots().await,
            "hotspot_detail" => self.hotspot_detail().await,
            "screen" => self.screen().await,
            "sync_daily" => self.sync_daily().await,
            "sync_progress" => self.sync_progress().await,
            "pause_sync" => self.pause_sync().await,
            "resume_sync" => self.resume_sync().await,
            "stop_sync" => self.stop_sync().await,
            "history" => self.history().await,
            "history_detail" => self.history_detail().await,
            "compare" => self.compare().await,
            "latest_summary" => self.latest_summary().await,
            _ => Err(error!("screening不支持方法: {}", method)),
        }
    }

    async fn status(&self) -> Result<Value> {
        let has_daily = self.check_daily_data().await;
        Ok(value!({
            "enabled": true,
            "installed": true,
            "version": "0.2.0",
            "dailyDataReady": has_daily,
        }))
    }

    async fn check_daily_data(&self) -> bool {
        let connector = match get_db_connector() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let sql = "SELECT COUNT(*) AS cnt FROM stock_daily WHERE status >= 1";
        match query_rows(sql, vec![], &connector) {
            Ok(rows) => {
                if let Some(row) = rows.first() {
                    let count = row_get_value(row, "cnt").as_u64().unwrap_or(0);
                    return count > 0;
                }
                false
            }
            Err(_) => false,
        }
    }

    async fn sync_daily(&self) -> Result<Value> {
        {
            let st = SYNC_STATUS.lock().unwrap();
            if st.running {
                return Ok(
                    value!({"message": "同步已在进行中", "progress": st.done, "total": st.total}),
                );
            }
        }

        let em = EastMoney::new();
        let spot = em
            .stock_zh_a_spot()
            .await
            .map_err(|e| tube::Error::from(format!("获取行情失败: {}", e)))?;

        let codes: Vec<String> = spot
            .iter()
            .filter_map(|s| {
                let code: String = s.get("代码").and_then(|v| v.as_str()).unwrap_or_default();
                if code.starts_with('8') || code.starts_with('4') {
                    None
                } else {
                    Some(code)
                }
            })
            .collect();

        let total = codes.len() as u32;
        {
            let mut st = SYNC_STATUS.lock().unwrap();
            st.running = true;
            st.paused = false;
            st.total = total;
            st.done = 0;
            st.failed = 0;
            st.phase = "fetching".to_string();
        }
        broadcast_screening_status();

        let codes_clone = codes.clone();
        let conf = dsa_core::get_global_config();
        let is_sqlite = conf.database.is_sqlite();
        let retention_days = conf.data_sync.retention_days as i64;
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime for sync_daily");

                rt.block_on(async {
                    let kline_base_url = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
                    let retention_date = {
                        let now = chrono::Local::now();
                        let cutoff = now - chrono::Duration::days(retention_days);
                        cutoff.format("%Y-%m-%d").to_string()
                    };

                    for code in &codes_clone {
                        if !wait_if_screening_paused() {
                            break;
                        }

                        let market_id = if code.starts_with('6') { 1 } else { 0 };
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
                                match super::system::parse_kline_response(&response_text, is_sqlite, &retention_date) {
                                    Ok(bars) => {
                                        if bars.is_empty() {
                                            log_debug!("股票 {} 无K线数据", code);
                                        } else {
                                            log_debug!("股票 {} 获取到{}条K线，写入DB", code, bars.len());
                                            dsa_core::utils::save_all_kline_to_db(code, &bars);
                                        }
                                    }
                                    Err(e) => {
                                        log_warn!("解析日线数据失败 {}: {}", code, e);
                                        SYNC_STATUS.lock().unwrap().failed += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                log_warn!("获取日线失败 {}: {}", code, e);
                                SYNC_STATUS.lock().unwrap().failed += 1;
                            }
                        }

                        {
                            let mut st = SYNC_STATUS.lock().unwrap();
                            st.done += 1;
                        }
                        broadcast_screening_status();

                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    }

                {
                    let mut st = SYNC_STATUS.lock().unwrap();
                    st.phase = "calculating_indicators".to_string();
                }
                broadcast_screening_status();

                let connector = match get_db_connector() {
                    Ok(c) => c,
                    Err(e) => {
                        log_error!("指标计算DB连接失败: {}", e);
                        let mut st = SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.phase = "done".to_string();
                        return;
                    }
                };

                let sql = "SELECT DISTINCT stock_code, stock_name FROM stock_daily WHERE status = 1 ORDER BY stock_code";
                let rows = match query_rows(sql, vec![], &connector) {
                    Ok(r) => r,
                    Err(e) => {
                        log_error!("查询股票列表失败: {}", e);
                        let mut st = SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.phase = "done".to_string();
                        return;
                    }
                };

                let analyzer = TechnicalAnalyzer::new();
                for row in &rows {
                    let code = row_get_string(row, "stockCode");
                    let hist_sql = "SELECT close, trade_date FROM stock_daily \
                         WHERE stock_code = :code AND status >= 1 ORDER BY trade_date ASC";
                    let hist_rows = match query_rows(
                        hist_sql,
                        vec![("code".to_string(), Value::from(code.clone()))],
                        &connector,
                    ) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };

                    let closes: Vec<f64> = hist_rows.iter()
                        .map(|r| row_get_f64(r, "close"))
                        .collect();

                    if closes.len() < 60 {
                        continue;
                    }

                    let ma5 = analyzer.sma(&closes, 5);
                    let ma10 = analyzer.sma(&closes, 10);
                    let ma20 = analyzer.sma(&closes, 20);
                    let ma60 = analyzer.sma(&closes, 60);
                    let (dif, dea, macd_hist) = analyzer.macd(&closes, 12, 26, 9);

                    let last_date_row = hist_rows.last().unwrap();
                    let last_date = row_get_string(last_date_row, "tradeDate");

                    let update_sql = "UPDATE stock_daily SET \
                         ma5 = :ma5, ma10 = :ma10, ma20 = :ma20, ma60 = :ma60, \
                         dif = :dif, dea = :dea, macd_hist = :macd_hist \
                         WHERE stock_code = :code AND trade_date = :date AND status >= 1";
                    let _ = execute(
                        update_sql,
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
                }

                {
                    let mut st = SYNC_STATUS.lock().unwrap();
                    st.running = false;
                    st.paused = false;
                    st.phase = "done".to_string();
                }
                broadcast_screening_status();
            });
            }));

            if let Err(_) = result {
                log_error!("筛选日线同步线程 panic, 已自动恢复状态");
                let mut st = SYNC_STATUS.lock().unwrap();
                st.running = false;
                st.paused = false;
                st.phase = "error".to_string();
                broadcast_screening_status();
            }
        });

        Ok(value!({"message": "同步已启动", "total": total}))
    }

    async fn sync_progress(&self) -> Result<Value> {
        let st = SYNC_STATUS.lock().unwrap();
        Ok(value!({
            "running": st.running,
            "paused": st.paused,
            "total": st.total,
            "done": st.done,
            "failed": st.failed,
            "phase": st.phase.clone(),
            "task": "sync_daily",
        }))
    }

    async fn pause_sync(&self) -> Result<Value> {
        {
            let mut st = SYNC_STATUS.lock().unwrap();
            if !st.running {
                return Ok(value!({"message": "没有正在运行的任务"}));
            }
            if st.paused {
                return Ok(value!({"message": "任务已处于暂停状态"}));
            }
            st.paused = true;
        }
        broadcast_screening_status();
        Ok(value!({"message": "任务已暂停"}))
    }

    async fn resume_sync(&self) -> Result<Value> {
        {
            let mut st = SYNC_STATUS.lock().unwrap();
            if !st.running {
                return Ok(value!({"message": "没有正在运行的任务"}));
            }
            if !st.paused {
                return Ok(value!({"message": "任务未暂停"}));
            }
            st.paused = false;
        }
        broadcast_screening_status();
        Ok(value!({"message": "任务已继续"}))
    }

    async fn stop_sync(&self) -> Result<Value> {
        {
            let mut st = SYNC_STATUS.lock().unwrap();
            if !st.running {
                return Ok(value!({"message": "没有正在运行的任务"}));
            }
            st.running = false;
            st.paused = false;
        }
        broadcast_screening_status();
        Ok(value!({"message": "任务已停止"}))
    }

    async fn strategies(&self) -> Result<Value> {
        Ok(value!([
            {"id": "dual_low", "name": "双低策略", "description": "低价+低市盈率筛选"},
            {"id": "breakout", "name": "突破策略", "description": "价格突破均线压力位"},
            {"id": "value", "name": "价值策略", "description": "低PB+高ROE筛选"},
            {"id": "momentum", "name": "动量策略", "description": "近期涨幅领先股票"},
            {
                "id": "macd_golden_cross",
                "name": "MACD零上金叉",
                "description": "股价60日线上+DIF/DEA零上+绿柱缩短金叉",
                "requiresDailyData": true,
                "parameters": {
                    "lookback": {"type": "integer", "default": 5, "min": 1, "max": 20, "label": "回看天数", "description": "判断金叉的回看窗口"},
                    "hist_lookback": {"type": "integer", "default": 10, "min": 2, "max": 30, "label": "柱状历史条数", "description": "取最近N条MACD柱判断趋势"},
                    "dif_threshold": {"type": "number", "default": 0, "min": -1, "max": 10, "step": 0.01, "label": "DIF阈值", "description": "DIF需大于此值(0=零上)"},
                    "dea_threshold": {"type": "number", "default": 0, "min": -1, "max": 10, "step": 0.01, "label": "DEA阈值", "description": "DEA需大于此值(0=零上)"},
                    "ma_period": {"type": "select", "default": "ma60", "options": ["ma20", "ma60", "ma120"], "label": "均线周期", "description": "价格需高于此均线"},
                },
            },
        ]))
    }

    async fn hotspots(&self) -> Result<Value> {
        let em = EastMoney::new();
        let industry = em.sector_rank("industry").await.unwrap_or_else(|e| {
            tube::err_log!("获取行业热点失败: {}", e);
            vec![]
        });
        let concept = em.sector_rank("concept").await.unwrap_or_else(|e| {
            tube::err_log!("获取概念热点失败: {}", e);
            vec![]
        });

        if industry.is_empty() && concept.is_empty() {
            return Err(error!("获取热点数据失败，请检查网络连接"));
        }

        let mut all: Vec<Value> = Vec::new();
        all.extend(industry.into_iter().take(10));
        all.extend(concept.into_iter().take(10));

        let mut ranked: Vec<Value> = all;
        ranked.sort_by(|a, b| {
            let ca = a
                .get("changePercent")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let cb = b
                .get("changePercent")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked.truncate(12);

        Ok(Value::Array(ranked))
    }

    async fn hotspot_detail(&self) -> Result<Value> {
        let params = self.value();
        let topic_val = utils::param_string(&params, "topic");
        if topic_val.is_empty() {
            return Err(error!("请提供热点主题"));
        }

        let em = EastMoney::new();
        let mut matched: Vec<Value> = Vec::new();

        for sector_type in &["industry", "concept"] {
            if let Ok(sectors) = em.sector_rank(sector_type).await {
                for item in &sectors {
                    let name = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    if name.contains(topic_val.as_str()) || topic_val.contains(name.as_str()) {
                        matched.push(item.clone());
                    }
                }
            }
        }

        Ok(value!({
            "topic": topic_val.clone(),
            "description": format!("{}相关热门板块", topic_val),
            "sectors": matched,
        }))
    }

    async fn screen(&self) -> Result<Value> {
        let params = self.value();
        let strategy = params
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "dual_low".to_string());
        let limit = params.get("limit").and_then(|v| v.as_f64()).unwrap_or(20.0) as usize;

        if strategy == "macd_golden_cross" {
            if !self.check_daily_data().await {
                return Err(error!("MACD策略需要历史日线数据，请先执行「同步日线数据」"));
            }
            let macd_params_val = params.get("macd_params").cloned().unwrap_or(Value::Null);
            let macd_params = MacdParams::from_value(&macd_params_val);
            return self.filter_macd_golden_cross(limit, &macd_params).await;
        }

        let em = EastMoney::new();
        let spot = em
            .stock_zh_a_spot()
            .await
            .map_err(|e| tube::Error::from(format!("获取行情失败: {}", e)))?;

        let results: Vec<Value> = match strategy.as_str() {
            "dual_low" => Self::filter_dual_low(&spot, limit),
            "breakout" => Self::filter_breakout(&spot, limit),
            "value" => Self::filter_value(&spot, limit),
            "momentum" => Self::filter_momentum(&spot, limit),
            _ => spot.into_iter().take(limit).collect(),
        };
        let count = results.len() as i64;
        Ok(value!({"strategy": strategy, "count": count, "results": results}))
    }

    async fn filter_macd_golden_cross(&self, limit: usize, macd_params: &MacdParams) -> Result<Value> {
        let connector =
            get_db_connector().map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let ma_col = macd_params.ma_period.as_str();
        let dif_th = macd_params.dif_threshold;
        let dea_th = macd_params.dea_threshold;

        let sql = format!(
            "SELECT sd.stock_code, sd.stock_name, sd.close, {ma_col} AS ma_val, sd.dif, sd.dea, sd.macd_hist, \
             sd.trade_date, sd.pct_chg, sd.volume, sd.amount, sd.turnover_rate, sd.volume_ratio \
             FROM stock_daily sd \
             INNER JOIN ( \
                 SELECT stock_code, MAX(trade_date) AS max_date \
                 FROM stock_daily WHERE status >= 1 AND {ma_col} > 0 \
                 GROUP BY stock_code \
             ) latest ON sd.stock_code = latest.stock_code AND sd.trade_date = latest.max_date \
             WHERE sd.status >= 1 AND sd.{ma_col} > 0 AND sd.close > sd.{ma_col} AND sd.dif > :dif_th AND sd.dea > :dea_th \
             ORDER BY sd.macd_hist DESC"
        );

        let rows = query_rows(
            &sql,
            vec![
                ("dif_th".to_string(), Value::from(dif_th)),
                ("dea_th".to_string(), Value::from(dea_th)),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询MACD零上金叉候选失败: {}", e)))?;

        let analyzer = TechnicalAnalyzer::new();
        let mut results: Vec<Value> = Vec::new();
        let mut checked = 0u32;
        let batch_id = format!("macd_{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let params_json = macd_params.to_json_string();
        let hist_limit = macd_params.hist_lookback;

        for row in &rows {
            if results.len() >= limit {
                break;
            }
            let code = row_get_string(row, "stockCode");
            let code_val: &str = &code;

            let hist_sql = format!(
                "SELECT macd_hist FROM stock_daily \
                 WHERE stock_code = :code AND status >= 1 AND macd_hist != 0 \
                 ORDER BY trade_date DESC LIMIT {}",
                hist_limit
            );
            let hist_rows = query_rows(
                &hist_sql,
                vec![("code".to_string(), Value::from(code_val.to_string()))],
                &connector,
            )
            .map_err(|e| tube::Error::from(format!("查询MACD历史失败: {}", e)))?;

            if hist_rows.len() < 2 {
                checked += 1;
                continue;
            }

            let hist_series: Vec<f64> = hist_rows
                .iter()
                .map(|r| row_get_f64(r, "macdHist"))
                .collect();
            let mut hist_asc = hist_series;
            hist_asc.reverse();

            if !analyzer.is_macd_golden_cross(&hist_asc, macd_params.lookback) {
                checked += 1;
                continue;
            }

            let name = row_get_string(row, "stockName");
            let close = row_get_f64(row, "close");
            let ma_val = row_get_f64(row, "maVal");
            let dif = row_get_f64(row, "dif");
            let dea = row_get_f64(row, "dea");
            let macd_hist = row_get_f64(row, "macdHist");
            let pct_chg = row_get_f64(row, "pctChg");
            let turnover_rate = row_get_f64(row, "turnoverRate");
            let volume_ratio = row_get_f64(row, "volumeRatio");

            let above_ma_pct = if ma_val > 0.0 {
                (close - ma_val) / ma_val * 100.0
            } else {
                0.0
            };
            let above_ma_rounded = (above_ma_pct * 100.0).round() / 100.0;

            let _ = Self::save_screening_result(
                &connector,
                "macd_golden_cross",
                code_val,
                &name,
                close,
                ma_val,
                dif,
                dea,
                macd_hist,
                pct_chg,
                turnover_rate,
                volume_ratio,
                above_ma_rounded,
                &params_json,
                &batch_id,
            );

            results.push(value!({
                "code": code_val,
                "name": name,
                "close": close,
                "ma60": ma_val,
                "dif": dif,
                "dea": dea,
                "macd_hist": macd_hist,
                "pct_chg": pct_chg,
                "turnover_rate": turnover_rate,
                "volume_ratio": volume_ratio,
                "above_ma60_pct": above_ma_rounded,
                "strategy": "macd_golden_cross",
                "batch_id": batch_id.clone(),
            }));

            checked += 1;
        }

        Ok(value!({
            "strategy": "macd_golden_cross",
            "count": results.len() as i64,
            "checked": checked as i64,
            "results": results,
            "batch_id": batch_id,
            "params": macd_params.to_value(),
        }))
    }

    fn save_screening_result(
        connector: &deck_connector::Connector,
        strategy: &str,
        stock_code: &str,
        stock_name: &str,
        close: f64,
        ma_value: f64,
        dif: f64,
        dea: f64,
        macd_hist: f64,
        pct_chg: f64,
        turnover_rate: f64,
        volume_ratio: f64,
        above_ma_pct: f64,
        params_json: &str,
        batch_id: &str,
    ) -> std::result::Result<(), String> {        let conf = dsa_core::get_global_config();
        let is_sqlite = conf.database.is_sqlite();
        let now_expr = if is_sqlite {
            "datetime('now')"
        } else {
            "NOW()"
        };
        let sql = format!(
            "INSERT INTO screening_results \
             (strategy, stock_code, stock_name, close, ma_value, dif, dea, macd_hist, \
              pct_chg, turnover_rate, volume_ratio, above_ma_pct, params_json, batch_id, status, create_time, modify_time) \
             VALUES (:strategy, :code, :name, :close, :ma_value, :dif, :dea, :macd_hist, \
              :pct_chg, :turnover_rate, :volume_ratio, :above_ma_pct, :params_json, :batch_id, 1, {}, {})",
            now_expr, now_expr
        );
        execute(
            &sql,
            vec![
                ("strategy".to_string(), Value::from(strategy.to_string())),
                ("code".to_string(), Value::from(stock_code.to_string())),
                ("name".to_string(), Value::from(stock_name.to_string())),
                ("close".to_string(), Value::from(close)),
                ("ma_value".to_string(), Value::from(ma_value)),
                ("dif".to_string(), Value::from(dif)),
                ("dea".to_string(), Value::from(dea)),
                ("macd_hist".to_string(), Value::from(macd_hist)),
                ("pct_chg".to_string(), Value::from(pct_chg)),
                ("turnover_rate".to_string(), Value::from(turnover_rate)),
                ("volume_ratio".to_string(), Value::from(volume_ratio)),
                ("above_ma_pct".to_string(), Value::from(above_ma_pct)),
                ("params_json".to_string(), Value::from(params_json.to_string())),
                ("batch_id".to_string(), Value::from(batch_id.to_string())),
            ],
            connector,
        )
        .map(|_| ())
        .map_err(|e| format!("保存筛选结果失败: {}", e))
    }

    async fn history(&self) -> Result<Value> {
        let params = self.value();
        let strategy = utils::param_string(&params, "strategy");
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

        let connector =
            get_db_connector().map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let sql = if strategy.is_empty() {
            "SELECT batch_id, strategy, COUNT(*) AS count, MIN(create_time) AS run_time, params_json \
             FROM screening_results WHERE status >= 1 \
             GROUP BY batch_id, strategy, params_json \
             ORDER BY MIN(create_time) DESC".to_string()
        } else {
            format!(
                "SELECT batch_id, strategy, COUNT(*) AS count, MIN(create_time) AS run_time, params_json \
                 FROM screening_results WHERE status >= 1 AND strategy = '{}'
                 GROUP BY batch_id, strategy, params_json \
                 ORDER BY MIN(create_time) DESC",
                strategy.replace('\'', "''")
            )
        };

        let rows = query_rows(&sql, vec![], &connector)
            .map_err(|e| tube::Error::from(format!("查询筛选历史失败: {}", e)))?;

        let items: Vec<Value> = rows
            .iter()
            .take(limit)
            .map(|row| {
                let batch_id = row_get_string(row, "batchId");
                let strat = row_get_string(row, "strategy");
                let count = row_get_value(row, "count").as_u64().unwrap_or(0) as i64;
                let run_time = row_get_string(row, "runTime");
                let params_json = row_get_string(row, "paramsJson");
                value!({
                    "batch_id": batch_id,
                    "strategy": strat,
                    "count": count,
                    "run_time": run_time,
                    "params_json": params_json,
                })
            })
            .collect();

        Ok(Value::Array(items))
    }

    async fn history_detail(&self) -> Result<Value> {
        let params = self.value();
        let batch_id = utils::param_string(&params, "batch_id");
        if batch_id.is_empty() {
            return Err(error!("请提供batch_id"));
        }

        let connector =
            get_db_connector().map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let sql = "SELECT id, strategy, stock_code, stock_name, close, ma_value, dif, dea, macd_hist, \
             pct_chg, turnover_rate, volume_ratio, above_ma_pct, params_json, batch_id, create_time \
             FROM screening_results \
             WHERE batch_id = :batch_id AND status >= 1 \
             ORDER BY create_time ASC";

        let rows = query_rows(
            sql,
            vec![("batch_id".to_string(), Value::from(batch_id.clone()))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询筛选历史详情失败: {}", e)))?;

        let items: Vec<Value> = rows
            .iter()
            .map(|row| {
                value!({
                    "id": row_get_value(row, "id").as_i64().unwrap_or(0),
                    "strategy": row_get_string(row, "strategy"),
                    "code": row_get_string(row, "stockCode"),
                    "name": row_get_string(row, "stockName"),
                    "close": row_get_f64(row, "close"),
                    "ma60": row_get_f64(row, "maValue"),
                    "dif": row_get_f64(row, "dif"),
                    "dea": row_get_f64(row, "dea"),
                    "macd_hist": row_get_f64(row, "macdHist"),
                    "pct_chg": row_get_f64(row, "pctChg"),
                    "turnover_rate": row_get_f64(row, "turnoverRate"),
                    "volume_ratio": row_get_f64(row, "volumeRatio"),
                    "above_ma60_pct": row_get_f64(row, "aboveMaPct"),
                    "batch_id": row_get_string(row, "batchId"),
                    "create_time": row_get_string(row, "createTime"),
                })
            })
            .collect();

        let count = items.len() as i64;

        Ok(value!({
            "batch_id": batch_id,
            "results": items,
            "count": count,
        }))
    }

    async fn compare(&self) -> Result<Value> {
        let params = self.value();
        let batch_id_1 = utils::param_string(&params, "batch_id_1");
        let batch_id_2 = utils::param_string(&params, "batch_id_2");
        if batch_id_1.is_empty() || batch_id_2.is_empty() {
            return Err(error!("请提供两个batch_id进行对比"));
        }

        let connector =
            get_db_connector().map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let sql = "SELECT stock_code, stock_name FROM screening_results \
             WHERE batch_id = :batch_id AND status >= 1";

        let rows1 = query_rows(
            sql,
            vec![("batch_id".to_string(), Value::from(batch_id_1.clone()))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询批次1失败: {}", e)))?;

        let rows2 = query_rows(
            sql,
            vec![("batch_id".to_string(), Value::from(batch_id_2.clone()))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询批次2失败: {}", e)))?;

        let codes1: std::collections::HashSet<String> = rows1
            .iter()
            .map(|r| row_get_string(r, "stockCode"))
            .collect();
        let codes2: std::collections::HashSet<String> = rows2
            .iter()
            .map(|r| row_get_string(r, "stockCode"))
            .collect();

        let only_in_1: Vec<String> = codes1.difference(&codes2).cloned().collect();
        let only_in_2: Vec<String> = codes2.difference(&codes1).cloned().collect();
        let common: Vec<String> = codes1.intersection(&codes2).cloned().collect();

        Ok(value!({
            "batch_id_1": batch_id_1,
            "batch_id_2": batch_id_2,
            "count_1": codes1.len() as i64,
            "count_2": codes2.len() as i64,
            "common": common,
            "common_count": (codes1.intersection(&codes2).count()) as i64,
            "only_in_1": only_in_1,
            "only_in_2": only_in_2,
        }))
    }

    async fn latest_summary(&self) -> Result<Value> {
        let connector =
            get_db_connector().map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let sql = "SELECT batch_id, strategy, COUNT(*) AS count, MIN(create_time) AS run_time \
             FROM screening_results WHERE status >= 1 AND strategy = 'macd_golden_cross' \
             GROUP BY batch_id ORDER BY MIN(create_time) DESC LIMIT 1";

        let rows = query_rows(sql, vec![], &connector)
            .map_err(|e| tube::Error::from(format!("查询最新MACD摘要失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(value!({
                "has_data": false,
                "latest_count": 0,
            }));
        }

        let row = &rows[0];
        let batch_id = row_get_string(row, "batchId");
        let count = row_get_value(row, "count").as_u64().unwrap_or(0) as i64;
        let run_time = row_get_string(row, "runTime");

        let detail_sql = "SELECT stock_code, stock_name, close, dif, dea, macd_hist, above_ma_pct \
                          FROM screening_results \
                          WHERE batch_id = :batch_id AND status >= 1 \
                          ORDER BY above_ma_pct DESC LIMIT 5";
        let detail_rows = query_rows(
            detail_sql,
            vec![("batch_id".to_string(), Value::from(batch_id.clone()))],
            &connector,
        )
        .unwrap_or_default();

        let top_stocks: Vec<Value> = detail_rows
            .iter()
            .map(|r| {
                value!({
                    "code": row_get_string(r, "stockCode"),
                    "name": row_get_string(r, "stockName"),
                    "close": row_get_f64(r, "close"),
                    "dif": row_get_f64(r, "dif"),
                    "dea": row_get_f64(r, "dea"),
                    "macd_hist": row_get_f64(r, "macdHist"),
                    "above_ma_pct": row_get_f64(r, "aboveMaPct"),
                })
            })
            .collect();

        Ok(value!({
            "has_data": true,
            "latest_batch_id": batch_id,
            "latest_count": count,
            "latest_run_time": run_time,
            "top_stocks": top_stocks,
        }))
    }

    fn filter_dual_low(stocks: &[Value], limit: usize) -> Vec<Value> {
        stocks
            .iter()
            .filter(|s| {
                let price = s.get("最新价").and_then(|v| v.as_f64()).unwrap_or(999.0);
                let pe = s.get("市盈率动").and_then(|v| v.as_f64()).unwrap_or(999.0);
                let code: String = s.get("代码").and_then(|v| v.as_str()).unwrap_or_default();
                price > 0.0
                    && price < 20.0
                    && pe > 0.0
                    && pe < 30.0
                    && !code.starts_with('8')
                    && !code.starts_with('4')
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn filter_breakout(stocks: &[Value], limit: usize) -> Vec<Value> {
        stocks
            .iter()
            .filter(|s| {
                let change_pct = s.get("涨跌幅").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let turnover = s.get("换手率").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let volume_ratio = s.get("量比").and_then(|v| v.as_f64()).unwrap_or(0.0);
                change_pct > 3.0 && change_pct < 9.8 && turnover > 3.0 && volume_ratio > 2.0
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn filter_value(stocks: &[Value], limit: usize) -> Vec<Value> {
        stocks
            .iter()
            .filter(|s| {
                let pb = s.get("市净率").and_then(|v| v.as_f64()).unwrap_or(999.0);
                let roe = s
                    .get("加权净资产收益率")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let pe = s.get("市盈率动").and_then(|v| v.as_f64()).unwrap_or(999.0);
                pb > 0.0 && pb < 2.0 && roe > 10.0 && pe > 0.0 && pe < 20.0
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn filter_momentum(stocks: &[Value], limit: usize) -> Vec<Value> {
        let mut ranked: Vec<&Value> = stocks
            .iter()
            .filter(|s| {
                let change_pct = s.get("涨跌幅").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let code: String = s.get("代码").and_then(|v| v.as_str()).unwrap_or_default();
                change_pct > 0.0 && !code.starts_with('8') && !code.starts_with('4')
            })
            .collect();
        ranked.sort_by(|a, b| {
            let ca = a.get("涨跌幅").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let cb = b.get("涨跌幅").and_then(|v| v.as_f64()).unwrap_or(0.0);
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked.into_iter().take(limit).cloned().collect()
    }

    fn value(&self) -> Value {
        self.request.value.clone()
    }
}
