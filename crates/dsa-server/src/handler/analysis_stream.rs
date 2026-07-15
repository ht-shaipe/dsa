//! SSE streaming handler for stock analysis

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use tube_web::sse_channel;

use ai_llm_kit::{LlmFactory, LlmProvider, LlmService};
use dsa_core::models::AnalysisReport;
use dsa_core::utils;
use dsa_pipeline::pipeline::AnalysisPipeline;
use dsa_pipeline::report_renderer::ReportRenderer;
use std::sync::Arc;
use tracing::info;

pub async fn analysis_stream(req: HttpRequest, payload: web::Payload) -> HttpResponse {
    let param = tube_web::parse_request(req.clone(), payload).await;
    let (mut sender, receiver) = sse_channel(32);

    let code = utils::param_string(&param.value, "code");
    let name = utils::param_string(&param.value, "name");

    if code.is_empty() {
        let msg = serde_json::json!({"type": "error", "content": "请提供股票代码"});
        let _ = sender
            .send_data(&serde_json::to_string(&msg).unwrap_or_default())
            .await;
        let _ = sender.done("{}").await;
        return receiver.respond_to(&req);
    }

    actix_rt::spawn(async move {
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();

        if api_key.is_empty() {
            let msg = serde_json::json!({"type": "error", "content": "API Key 未配置"});
            let _ = sender
                .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                .await;
            let _ = sender.done("{}").await;
            return;
        }

        let status_msg = serde_json::json!({"type": "status", "content": "正在获取行情数据..."});
        let _ = sender
            .send_data(&serde_json::to_string(&status_msg).unwrap_or_default())
            .await;

        let kline_data = match utils::fetch_kline(&code, "daily").await {
            Ok(d) => d,
            Err(e) => {
                let msg =
                    serde_json::json!({"type": "error", "content": format!("获取K线失败: {}", e)});
                let _ = sender
                    .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                    .await;
                let _ = sender.done("{}").await;
                return;
            }
        };

        let realtime = match utils::fetch_realtime_quote(&code).await {
            Ok(r) => r,
            Err(e) => {
                let msg = serde_json::json!({"type": "error", "content": format!("获取实时行情失败: {}", e)});
                let _ = sender
                    .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                    .await;
                let _ = sender.done("{}").await;
                return;
            }
        };
        let market_ctx = utils::fetch_market_context().await;

        let stock_name = if name.is_empty() {
            realtime
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| code.clone())
                .to_string()
        } else {
            name.clone()
        };

        let status_msg = serde_json::json!({"type": "status", "content": "正在进行AI分析..."});
        let _ = sender
            .send_data(&serde_json::to_string(&status_msg).unwrap_or_default())
            .await;

        let pipeline = match AnalysisPipeline::new(
            &conf.llm.provider,
            &api_key,
            &conf.llm.model,
            conf.llm.temperature,
            conf.llm.timeout_seconds,
        ) {
            Ok(p) => p,
            Err(e) => {
                let msg = serde_json::json!({"type": "error", "content": format!("Pipeline初始化失败: {}", e)});
                let _ = sender
                    .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                    .await;
                let _ = sender.done("{}").await;
                return;
            }
        };

        let llm_provider = match LlmProvider::instance(&conf.llm.provider) {
            Ok(p) => p,
            Err(_) => {
                let msg = serde_json::json!({"type": "error", "content": "Provider错误"});
                let _ = sender
                    .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                    .await;
                let _ = sender.done("{}").await;
                return;
            }
        };
        let llm: Arc<Box<dyn LlmService>> = Arc::new(LlmFactory::create(llm_provider, &api_key));

        let body = pipeline.build_stream_body(
            &code,
            &stock_name,
            &kline_data,
            Some(&realtime),
            market_ctx.as_deref(),
        );
        if let Err(e) = body {
            let msg =
                serde_json::json!({"type": "error", "content": format!("构建请求失败: {}", e)});
            let _ = sender
                .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                .await;
            let _ = sender.done("{}").await;
            return;
        }
        let body = body.unwrap();

        let start = std::time::Instant::now();
        let full_content = Arc::new(tokio::sync::Mutex::new(String::new()));
        let full_content_clone = full_content.clone();
        let sender_clone = sender.clone();

        let callback: Arc<ai_llm_kit::StreamCallback> =
            Arc::new(move |chunk: String, is_over: bool| {
                let full_content = full_content_clone.clone();
                let mut sender = sender_clone.clone();
                Box::pin(async move {
                    if !chunk.is_empty() {
                        {
                            let mut fc = full_content.lock().await;
                            fc.push_str(&chunk);
                        }
                        let text_msg = serde_json::json!({"type": "text", "content": chunk});
                        let _ = sender
                            .send_data(&serde_json::to_string(&text_msg).unwrap_or_default())
                            .await;
                    }
                    if is_over {
                        let fc = full_content.lock().await;
                        let complete_msg = serde_json::json!({"type": "complete", "content": &*fc});
                        let _ = sender
                            .send_data(&serde_json::to_string(&complete_msg).unwrap_or_default())
                            .await;
                    }
                    Ok(tube::value!("ok"))
                }) as ai_llm_kit::StreamCallbackFuture
            });

        let stream_result = llm.chat_stream(&body, callback).await;

        let elapsed = start.elapsed().as_millis() as i64;

        match stream_result {
            Ok(stream_val) => {
                let fc = full_content.lock().await;
                let content_str = fc.clone();
                drop(fc);

                let (pt, ct) = if let Some(usage) = stream_val.get("usage") {
                    let p = usage
                        .get("prompt_tokens")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as i32;
                    let c = usage
                        .get("completion_tokens")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as i32;
                    (p, c)
                } else {
                    let est = estimate_tokens(&content_str);
                    (est, est)
                };
                dsa_core::utils::record_llm_usage(
                    &conf.llm.provider,
                    &conf.llm.model,
                    "analyze_stream",
                    pt,
                    ct,
                    elapsed,
                    &code,
                );

                let report = pipeline.parse_report_from_content(&content_str);
                match report {
                    Ok(report) => {
                        let renderer = ReportRenderer::new();
                        let markdown = renderer.render_markdown(&report);
                        let text = renderer.render_text(&report);

                        let report_json = match serde_json::to_value(&report) {
                            Ok(v) => v,
                            Err(_) => serde_json::Value::Null,
                        };

                        let mut report_json_for_db = report_json.clone();
                        if let serde_json::Value::Object(ref mut map) = report_json_for_db {
                            map.insert(
                                "markdown".to_string(),
                                serde_json::Value::String(markdown.clone()),
                            );
                            map.insert("text".to_string(), serde_json::Value::String(text.clone()));
                        }

                        save_report_to_db(&code, &stock_name, &report, &report_json_for_db);

                        let result_msg = serde_json::json!({
                            "type": "report",
                            "report": report_json,
                            "markdown": markdown,
                            "text": text,
                            "code": code,
                            "name": stock_name,
                        });
                        let _ = sender
                            .send_data(&serde_json::to_string(&result_msg).unwrap_or_default())
                            .await;
                    }
                    Err(e) => {
                        let content_display: String = if content_str.len() > 200 {
                            format!("{}...", &content_str[..200])
                        } else {
                            content_str.clone()
                        };
                        info!(
                            "流式分析完成但报告解析失败: {}, 原始内容前200字符: {}",
                            e, content_display
                        );

                        let result_msg = serde_json::json!({
                            "type": "raw",
                            "content": content_str,
                            "code": code,
                            "name": stock_name,
                            "parse_error": format!("{}", e),
                        });
                        let _ = sender
                            .send_data(&serde_json::to_string(&result_msg).unwrap_or_default())
                            .await;
                    }
                }
            }
            Err(e) => {
                let msg =
                    serde_json::json!({"type": "error", "content": format!("LLM调用失败: {}", e)});
                let _ = sender
                    .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                    .await;
            }
        }

        let _ = sender.done("{}").await;
    });

    receiver.respond_to(&req)
}

fn save_report_to_db(
    code: &str,
    name: &str,
    report: &AnalysisReport,
    report_json: &serde_json::Value,
) {
    let sentiment_score = report.sentiment_score.unwrap_or(0);
    let decision_type = report.decision_type.as_deref().unwrap_or("");
    let operation_advice = report.operation_advice.as_deref().unwrap_or("");
    let analysis_summary = report.analysis_summary.as_deref().unwrap_or("");
    let risk_warning = report.risk_warning.as_deref().unwrap_or("");
    let report_json_str = match serde_json::to_string(report_json) {
        Ok(s) => s,
        Err(_) => "{}".to_string(),
    };

    let conf = dsa_core::get_global_config();
    let connector = match dsa_core::db::get_db_connector() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("数据库连接获取失败，跳过保存: {}", e);
            return;
        }
    };

    let is_sqlite = conf.database.is_sqlite();
    let now_expr = if is_sqlite {
        "datetime('now')"
    } else {
        "NOW()"
    };
    let sql = format!(
        "INSERT INTO analysis_history (stock_code, stock_name, sentiment_score, decision_type, \
         operation_advice, analysis_summary, risk_warning, report_json, report_type, status, \
         llm_provider, llm_model, create_time, modify_time) \
         VALUES (:stock_code, :stock_name, :sentiment_score, :decision_type, \
         :operation_advice, :analysis_summary, :risk_warning, :report_json, 'full', 1, \
         :llm_provider, :llm_model, {}, {})",
        now_expr, now_expr
    );

    let params = vec![
        ("stock_code".to_string(), tube::Value::from(code)),
        ("stock_name".to_string(), tube::Value::from(name)),
        (
            "sentiment_score".to_string(),
            tube::Value::from(sentiment_score),
        ),
        (
            "decision_type".to_string(),
            tube::Value::from(decision_type),
        ),
        (
            "operation_advice".to_string(),
            tube::Value::from(operation_advice),
        ),
        (
            "analysis_summary".to_string(),
            tube::Value::from(analysis_summary),
        ),
        ("risk_warning".to_string(), tube::Value::from(risk_warning)),
        (
            "report_json".to_string(),
            tube::Value::from(report_json_str.as_str()),
        ),
        (
            "llm_provider".to_string(),
            tube::Value::from(conf.llm.provider.as_str()),
        ),
        (
            "llm_model".to_string(),
            tube::Value::from(conf.llm.model.as_str()),
        ),
    ];

    if let Err(e) = dsa_core::db::execute(&sql, params, &connector) {
        tracing::error!("stream save_report_to_db 失败: {}", e);
    }
}

fn estimate_tokens(text: &str) -> i32 {
    let mut token_count: f64 = 0.0;
    for ch in text.chars() {
        if ch as u32 > 0x7F {
            token_count += 1.5;
        } else {
            token_count += 0.25;
        }
    }
    token_count.ceil() as i32
}
