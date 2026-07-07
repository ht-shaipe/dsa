//! 调度服务 - 定时分析任务

use dsa_core::{DsaError, DsaResult, utils};
use std::sync::atomic::{AtomicBool, Ordering};
use tube::Value;
use chrono::Timelike;

lazy_static::lazy_static! {
    static ref SCHEDULER_RUNNING: AtomicBool = AtomicBool::new(false);
    static ref LAST_TRIGGER_TIME: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
}

/// 调度服务
pub struct SchedulerService {}

impl SchedulerService {
    /// 创建调度服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: start, stop, status, jobs, trigger
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "start" => self.start().await,
            "stop" => self.stop().await,
            "status" => self.status().await,
            "jobs" => self.jobs().await,
            "trigger" => self.trigger(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "scheduler不支持方法: {}",
                method
            ))),
        }
    }

    async fn start(&self) -> DsaResult<Value> {
        if SCHEDULER_RUNNING.load(Ordering::SeqCst) {
            return Ok(value!({"message": "scheduler already running"}));
        }

        SCHEDULER_RUNNING.store(true, Ordering::SeqCst);

        // Spawn a lightweight timer that sets a flag for the HTTP handler to check
        // The actual analysis is triggered via the HTTP API / trigger method
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(60)
            );

            while SCHEDULER_RUNNING.load(Ordering::SeqCst) {
                interval.tick().await;

                if !SCHEDULER_RUNNING.load(Ordering::SeqCst) {
                    break;
                }

                let conf = dsa_core::get_global_config();
                if !conf.scheduler.enabled {
                    continue;
                }

                // Check if current time matches schedule
                let now = chrono::Local::now();
                let time_str = now.format("%H:%M").to_string();

                if conf.scheduler.times.contains(&time_str) {
                    // Mark that analysis should be triggered
                    if let Ok(mut last) = LAST_TRIGGER_TIME.lock() {
                        // Only trigger once per minute
                        if *last != Some(time_str.clone()) {
                            *last = Some(time_str.clone());
                            tracing::info!("调度时间到达: {}, 需要触发分析", time_str);
                            // Note: actual analysis must be triggered via HTTP API
                            // because LLMService is not Send-safe for tokio::spawn
                        }
                    }
                }
            }
        });

        Ok(value!({"message": "scheduler started"}))
    }

    async fn stop(&self) -> DsaResult<Value> {
        SCHEDULER_RUNNING.store(false, Ordering::SeqCst);
        Ok(value!({"message": "scheduler stopped"}))
    }

    async fn status(&self) -> DsaResult<Value> {
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

    async fn jobs(&self) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let jobs = vec![
            value!({
                "name": "daily_analysis",
                "type": "daily",
                "schedule": conf.scheduler.times,
                "enabled": conf.scheduler.enabled,
                "stocks": conf.stock.watchlist,
            }),
        ];

        Ok(Value::Array(jobs))
    }

    async fn trigger(&self, params: &Value) -> DsaResult<Value> {
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
            return Err(DsaError::Validation("无股票代码可供分析".to_string()));
        }

        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Err(DsaError::LlmAnalysis("API Key 未配置".to_string()));
        }

        let pipeline = dsa_pipeline::pipeline::AnalysisPipeline::new(
            &conf.llm.provider,
            &api_key,
            &conf.llm.model,
            conf.llm.temperature,
            conf.llm.timeout_seconds,
        )?;

        let renderer = dsa_pipeline::report_renderer::ReportRenderer::new();
        let mut results = Vec::new();

        for code in &codes {
            match Self::analyze_single_code(&pipeline, &renderer, code).await {
                Ok(report_text) => {
                    results.push(value!({"code": code.as_str(), "status": "ok", "text": report_text}));
                }
                Err(e) => {
                    results.push(value!({"code": code.as_str(), "status": "error", "error": e.to_string()}));
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
    ) -> DsaResult<String> {
        let bars = utils::fetch_kline(code, "daily").await?;

        let realtime = utils::fetch_realtime_quote(code).await.ok();

        let name = realtime
            .as_ref()
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| code.to_string());

        let market_ctx: Option<&str> = None;

        let report = pipeline
            .analyze_stock(code, &name, &bars, realtime.as_ref(), market_ctx)
            .await?;

        let text = renderer.render_text(&report);
        Ok(text)
    }
}
