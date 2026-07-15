use dsa_core::db::{
    execute, get_db_connector, query_rows, row_get_f64, row_get_string, row_get_value,
};
use dsa_core::utils;
use dsa_pipeline::technical::TechnicalAnalyzer;
use qta_crawler::EastMoney;
use tube::{Result, Value};
use tube_web::RequestParameter;

lazy_static::lazy_static! {
    static ref SYNC_STATUS: std::sync::Mutex<SyncStatus> = std::sync::Mutex::new(SyncStatus::default());
}

#[derive(Default)]
struct SyncStatus {
    running: bool,
    total: u32,
    done: u32,
    failed: u32,
    phase: String,
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
                .expect("Failed to build tokio runtime for sync_daily");

            rt.block_on(async {
                let em = EastMoney::new();

                for code in &codes_clone {
                    if !SYNC_STATUS.lock().unwrap().running {
                        break;
                    }

                    match em.stock_zh_a_hist(code, Some("daily"), None, None, Some("qfq")).await {
                        Ok(raw) => {
                            let bars: Vec<dsa_core::models::KlineBar> = raw.iter().map(|item| {
                                dsa_core::models::KlineBar {
                                    date: item.get("日期").and_then(|v| v.as_str()).unwrap_or_default(),
                                    open: item.get("开盘").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                    high: item.get("最高").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                    low: item.get("最低").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                    close: item.get("收盘").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                    volume: item.get("成交量").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
                                    amount: item.get("成交额").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                }
                            }).collect();

                            if !bars.is_empty() {
                                dsa_core::utils::save_all_kline_to_db(code, &bars);
                            }
                        }
                        Err(_) => {
                            SYNC_STATUS.lock().unwrap().failed += 1;
                        }
                    }

                    {
                        let mut st = SYNC_STATUS.lock().unwrap();
                        st.done += 1;
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                }

                {
                    let mut st = SYNC_STATUS.lock().unwrap();
                    st.phase = "calculating_indicators".to_string();
                }

                let connector = match get_db_connector() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("指标计算DB连接失败: {}", e);
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
                        tracing::error!("查询股票列表失败: {}", e);
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
                    st.phase = "done".to_string();
                }
            });
        });

        Ok(value!({"message": "同步已启动", "total": total}))
    }

    async fn sync_progress(&self) -> Result<Value> {
        let st = SYNC_STATUS.lock().unwrap();
        Ok(value!({
            "running": st.running,
            "total": st.total,
            "done": st.done,
            "failed": st.failed,
            "phase": st.phase.clone(),
        }))
    }

    async fn strategies(&self) -> Result<Value> {
        Ok(value!([
            {"id": "dual_low", "name": "双低策略", "description": "低价+低市盈率筛选"},
            {"id": "breakout", "name": "突破策略", "description": "价格突破均线压力位"},
            {"id": "value", "name": "价值策略", "description": "低PB+高ROE筛选"},
            {"id": "momentum", "name": "动量策略", "description": "近期涨幅领先股票"},
            {"id": "macd_golden_cross", "name": "MACD零上金叉", "description": "股价60日线上+DIF/DEA零上+绿柱缩短金叉", "requiresDailyData": true},
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
            return self.filter_macd_golden_cross(limit).await;
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

    async fn filter_macd_golden_cross(&self, limit: usize) -> Result<Value> {
        let connector =
            get_db_connector().map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let sql = "SELECT sd.stock_code, sd.stock_name, sd.close, sd.ma60, sd.dif, sd.dea, sd.macd_hist, \
             sd.trade_date, sd.pct_chg, sd.volume, sd.amount, sd.turnover_rate, sd.volume_ratio \
             FROM stock_daily sd \
             INNER JOIN ( \
                 SELECT stock_code, MAX(trade_date) AS max_date \
                 FROM stock_daily WHERE status >= 1 AND ma60 > 0 \
                 GROUP BY stock_code \
             ) latest ON sd.stock_code = latest.stock_code AND sd.trade_date = latest.max_date \
             WHERE sd.status >= 1 AND sd.ma60 > 0 AND sd.close > sd.ma60 AND sd.dif > 0 AND sd.dea > 0 \
             ORDER BY sd.macd_hist DESC";

        let rows = query_rows(sql, vec![], &connector)
            .map_err(|e| tube::Error::from(format!("查询MACD零上金叉候选失败: {}", e)))?;

        let analyzer = TechnicalAnalyzer::new();
        let mut results: Vec<Value> = Vec::new();
        let mut checked = 0u32;

        for row in &rows {
            if results.len() >= limit {
                break;
            }
            let code = row_get_string(row, "stockCode");
            let code_val: &str = &code;

            let hist_sql = "SELECT macd_hist FROM stock_daily \
                 WHERE stock_code = :code AND status >= 1 AND macd_hist != 0 \
                 ORDER BY trade_date DESC LIMIT 10";
            let hist_rows = query_rows(
                hist_sql,
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

            if !analyzer.is_macd_golden_cross(&hist_asc, 5) {
                checked += 1;
                continue;
            }

            let name = row_get_string(row, "stockName");
            let close = row_get_f64(row, "close");
            let ma60 = row_get_f64(row, "ma60");
            let dif = row_get_f64(row, "dif");
            let dea = row_get_f64(row, "dea");
            let macd_hist = row_get_f64(row, "macdHist");
            let pct_chg = row_get_f64(row, "pctChg");
            let turnover_rate = row_get_f64(row, "turnoverRate");
            let volume_ratio = row_get_f64(row, "volumeRatio");

            let above_ma60_pct = if ma60 > 0.0 {
                (close - ma60) / ma60 * 100.0
            } else {
                0.0
            };

            results.push(value!({
                "code": code_val,
                "name": name,
                "close": close,
                "ma60": ma60,
                "dif": dif,
                "dea": dea,
                "macd_hist": macd_hist,
                "pct_chg": pct_chg,
                "turnover_rate": turnover_rate,
                "volume_ratio": volume_ratio,
                "above_ma60_pct": (above_ma60_pct * 100.0).round() / 100.0,
                "strategy": "macd_golden_cross",
            }));

            checked += 1;
        }

        Ok(value!({
            "strategy": "macd_golden_cross",
            "count": results.len() as i64,
            "checked": checked as i64,
            "results": results,
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
