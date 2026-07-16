use chrono::Timelike;
use dsa_core::utils;
use std::sync::atomic::{AtomicBool, Ordering};
use tube::{Result, Value};
use tube_web::RequestParameter;

lazy_static::lazy_static! {
    static ref SCHEDULER_RUNNING: AtomicBool = AtomicBool::new(false);
    static ref LAST_TRIGGER_TIME: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
}

pub struct Scheduler {
    request: RequestParameter,
}

impl Scheduler {
    pub fn new(param: &RequestParameter) -> Self {
        Scheduler {
            request: param.clone(),
        }
    }
    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "start" => self.start().await,
            "stop" => self.stop().await,
            "status" => self.status().await,
            "jobs" => self.jobs().await,
            "trigger" => self.trigger().await,
            _ => Err(tube::Error::from(format!(
                "scheduler不支持方法: {}",
                method
            ))),
        }
    }
    fn params(&self) -> &Value {
        &self.request.value
    }

    async fn start(&self) -> Result<Value> {
        if SCHEDULER_RUNNING.load(Ordering::SeqCst) {
            return Ok(value!({"message": "scheduler already running"}));
        }

        SCHEDULER_RUNNING.store(true, Ordering::SeqCst);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            while SCHEDULER_RUNNING.load(Ordering::SeqCst) {
                interval.tick().await;

                if !SCHEDULER_RUNNING.load(Ordering::SeqCst) {
                    break;
                }

                let conf = dsa_core::get_global_config();
                if !conf.scheduler.enabled {
                    continue;
                }

                let now = chrono::Local::now();
                let time_str = now.format("%H:%M").to_string();

                if conf.scheduler.times.contains(&time_str) {
                    if let Ok(mut last) = LAST_TRIGGER_TIME.lock() {
                        if *last != Some(time_str.clone()) {
                            *last = Some(time_str.clone());
                            tracing::info!("调度时间到达: {}, 需要触发分析", time_str);
                        }
                    }
                }
            }
        });

        Ok(value!({"message": "scheduler started"}))
    }

    async fn stop(&self) -> Result<Value> {
        SCHEDULER_RUNNING.store(false, Ordering::SeqCst);
        Ok(value!({"message": "scheduler stopped"}))
    }

    async fn status(&self) -> Result<Value> {
        let running = SCHEDULER_RUNNING.load(Ordering::SeqCst);
        let conf = dsa_core::get_global_config();

        let next_run = if running && !conf.scheduler.times.is_empty() {
            let now = chrono::Local::now();
            let current_minutes = now.time().hour() as i32 * 60 + now.time().minute() as i32;
            let mut next_time_str = conf.scheduler.times[0].clone();

            for t in &conf.scheduler.times {
                if let Ok(time) = chrono::NaiveTime::parse_from_str(t, "%H:%M") {
                    let time_minutes = time.hour() as i32 * 60 + time.minute() as i32;
                    if time_minutes > current_minutes {
                        next_time_str = t.clone();
                        break;
                    }
                }
            }

            next_time_str
        } else {
            String::new()
        };

        Ok(value!({
            "running": running,
            "nextRun": next_run,
            "scheduleTimes": conf.scheduler.times,
        }))
    }

    async fn jobs(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let jobs = vec![value!({
            "name": "daily_analysis",
            "type": "daily",
            "schedule": conf.scheduler.times,
            "enabled": conf.scheduler.enabled,
            "stocks": conf.stock.watchlist,
        })];

        Ok(Value::Array(jobs))
    }

    async fn trigger(&self) -> Result<Value> {
        let params = self.params();
        let job_type = params
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "daily".to_string());

        let codes_val = params
            .get("codes")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let conf = dsa_core::get_global_config();
        let codes: Vec<String> = if !codes_val.is_empty() {
            codes_val
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            conf.stock.watchlist.clone()
        };

        if codes.is_empty() {
            return Err(tube::Error::from("无股票代码可供分析"));
        }

        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Err(tube::Error::from("API Key 未配置"));
        }

        let pipeline = dsa_pipeline::pipeline::AnalysisPipeline::new(
            &conf.llm.provider,
            &api_key,
            &conf.llm.model,
            conf.llm.temperature,
            conf.llm.timeout_seconds,
        )
        .map_err(|e| tube::Error::msg(e.to_string()))?;

        let renderer = dsa_pipeline::report_renderer::ReportRenderer::new();
        let mut results = Vec::new();

        for code in &codes {
            match Self::analyze_single_code(&pipeline, &renderer, code).await {
                Ok((report_text, report)) => {
                    Self::save_scheduled_report(code, &report, &report_text);
                    results
                        .push(value!({"code": code.as_str(), "status": "ok", "text": report_text}));
                }
                Err(e) => {
                    results.push(
                        value!({"code": code.as_str(), "status": "error", "error": e.to_string()}),
                    );
                }
            }
        }

        Ok(value!({
            "message": format!("{} analysis completed", job_type),
            "results": results,
        }))
    }

    async fn analyze_single_code(
        pipeline: &dsa_pipeline::pipeline::AnalysisPipeline,
        renderer: &dsa_pipeline::report_renderer::ReportRenderer,
        code: &str,
    ) -> Result<(String, dsa_core::models::AnalysisReport)> {
        let bars = utils::fetch_kline(code, "daily")
            .await
            .map_err(|e| tube::Error::msg(e.to_string()))?;

        let realtime = utils::fetch_realtime_quote(code).await.ok();

        let name = realtime
            .as_ref()
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| code.to_string());

        let market_ctx: Option<&str> = None;

        let mut report = pipeline
            .analyze_stock(code, &name, &bars, realtime.as_ref(), market_ctx)
            .await
            .map_err(|e| tube::Error::msg(e.to_string()))?;

        let analysis_time = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let last_kline_date = bars.last().map(|b| b.date.as_str()).unwrap_or("未知");
        let freshness = dsa_core::utils::data_freshness_warning(last_kline_date, "K线");
        report.data_as_of = Some(format!("{} (分析时间: {}) | {}", last_kline_date, analysis_time, freshness));

        let text = renderer.render_text(&report);
        Ok((text, report))
    }

    fn save_scheduled_report(code: &str, report: &dsa_core::models::AnalysisReport, _text: &str) {
        let connector = match utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => return,
        };

        let sentiment_score = report.sentiment_score.unwrap_or(0);
        let decision_type = report.decision_type.as_deref().unwrap_or("");
        let operation_advice = report.operation_advice.as_deref().unwrap_or("");
        let analysis_summary = report.analysis_summary.as_deref().unwrap_or("");
        let risk_warning = report.risk_warning.as_deref().unwrap_or("");
        let name = report.stock_name.as_deref().unwrap_or(code);
        let conf = dsa_core::get_global_config();

        let report_json_str = serde_json::to_string(report).unwrap_or_else(|_| "{}".to_string());

        let is_sqlite = conf.database.is_sqlite();
        let now_expr = if is_sqlite {
            "datetime('now')"
        } else {
            "NOW()"
        };
        let sql = &format!("INSERT INTO analysis_history \
             (stock_code, stock_name, sentiment_score, decision_type, operation_advice, \
              analysis_summary, risk_warning, report_json, report_type, status, \
              llm_provider, llm_model, data_as_of, create_time, modify_time) \
             VALUES (:code, :name, :score, :dtype, :advice, :summary, :risk, :json, 'scheduled', 1, \
              :provider, :model, :data_as_of, {}, {})", now_expr, now_expr);

        if let Err(e) = dsa_core::db::execute(
            sql,
            vec![
                ("code".to_string(), Value::from(code.to_string())),
                ("name".to_string(), Value::from(name.to_string())),
                ("score".to_string(), Value::from(sentiment_score)),
                ("dtype".to_string(), Value::from(decision_type.to_string())),
                (
                    "advice".to_string(),
                    Value::from(operation_advice.to_string()),
                ),
                (
                    "summary".to_string(),
                    Value::from(analysis_summary.to_string()),
                ),
                ("risk".to_string(), Value::from(risk_warning.to_string())),
                ("json".to_string(), Value::from(report_json_str)),
                (
                    "provider".to_string(),
                    Value::from(conf.llm.provider.clone()),
                ),
                ("model".to_string(), Value::from(conf.llm.model.clone())),
                ("data_as_of".to_string(), Value::from(report.data_as_of.as_deref().unwrap_or(""))),
            ],
            &connector,
        ) {
            tracing::error!("save_scheduled_report 失败: {}", e);
        }
    }
}
