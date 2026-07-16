//! SSE streaming handler - Agent chat with intent parsing and tool orchestration

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use tube_web::sse_channel;

// ===== 意图识别 =====

#[derive(Debug, Clone)]
enum ChatIntent {
    StockQuery { code: String, name: String },
    MarketOverview,
    SectorQuery { keyword: String },
    General,
}

/// 从消息中提取6位股票代码
fn extract_stock_code(message: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)(?:sh|sz)?(\d{6})").ok()?;
    let caps = re.captures(message)?;
    let code = caps.get(1)?.as_str().to_string();
    if code.starts_with('6')
        || code.starts_with('9')
        || code.starts_with('0')
        || code.starts_with('3')
        || code.starts_with('8')
        || code.starts_with('4')
    {
        Some(code)
    } else {
        None
    }
}

/// 在线解析股票名称为代码（东方财富搜索API）
async fn resolve_name_online(keyword: &str) -> Option<(String, String)> {
    let url = format!(
        "https://searchapi.eastmoney.com/api/suggest/get?input={}&type=14&token=D84BF7C9-6EC6-4CB1-A820-8738966D5C9B&count=3",
        urlencoding::encode(keyword)
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build().ok()?;
    let resp = client
        .get(&url)
        .header("Referer", "https://so.eastmoney.com/")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let items = json
        .get("QuotationCodeTable")
        .and_then(|v| v.get("Data"))
        .and_then(|v| v.as_array())?;
    for item in items {
        let code = item
            .get("Code")
            .and_then(|c| c.as_str())
            .unwrap_or_default();
        let name = item
            .get("Name")
            .and_then(|n| n.as_str())
            .unwrap_or_default();
        let mkt = item
            .get("MktNum")
            .and_then(|m| m.as_str())
            .unwrap_or_default();
        let pure_code = if code.len() >= 6 {
            &code[code.len() - 6..]
        } else {
            code
        };
        if pure_code.starts_with('6') || pure_code.starts_with('0') || pure_code.starts_with('3') {
            let _full_code = if mkt == "1" {
                format!("sh{}", pure_code)
            } else {
                format!("sz{}", pure_code)
            };
            return Some((pure_code.to_string(), name.to_string()));
        }
    }
    None
}

/// 意图解析：判断用户想问什么
async fn parse_intent(message: &str) -> ChatIntent {
    if let Some(code) = extract_stock_code(message) {
        return ChatIntent::StockQuery {
            code,
            name: String::new(),
        };
    }

    let market_keywords = [
        "大盘",
        "指数",
        "上证",
        "深证",
        "创业板",
        "沪深",
        "A股",
        "市场",
    ];
    if market_keywords.iter().any(|k| message.contains(k)) {
        return ChatIntent::MarketOverview;
    }

    let sector_keywords = ["板块", "概念", "行业", "题材"];
    if sector_keywords.iter().any(|k| message.contains(k)) {
        let kw = message
            .replace(
                |c: char| "板块概念行业题材的分析一下看看怎么最近请给推荐有哪些".contains(c),
                "",
            )
            .trim()
            .to_string();
        return ChatIntent::SectorQuery {
            keyword: if kw.is_empty() {
                message.to_string()
            } else {
                kw
            },
        };
    }

    let stock_patterns = [
        "分析",
        "看看",
        "怎么样",
        "走势",
        "买入",
        "卖出",
        "持仓",
        "关注",
        "推荐",
        "估值",
        "财报",
        "业绩",
        "分红",
        "市盈",
        "市净",
    ];
    if stock_patterns.iter().any(|k| message.contains(k)) {
        let cleaned = message
            .replace(
                |c: char| {
                    "的分析一下看看怎么样走势买入卖出持仓关注推荐估值财报业绩分红市盈市净请给"
                        .contains(c)
                },
                "",
            )
            .trim()
            .to_string();
        if !cleaned.is_empty() && cleaned.len() >= 2 {
            if let Some((code, name)) = resolve_name_online(&cleaned).await {
                return ChatIntent::StockQuery { code, name };
            }
        }
    }

    if message.len() >= 2 && message.len() <= 6 && !message.contains(' ') {
        if let Some((code, name)) = resolve_name_online(message).await {
            return ChatIntent::StockQuery { code, name };
        }
    }

    ChatIntent::General
}

// ===== 工具调用 =====

fn symbol_from_code(code: &str) -> String {
    if code.starts_with('6') || code.starts_with('9') {
        format!("sh{}", code)
    } else {
        format!("sz{}", code)
    }
}

async fn fetch_stock_context(
    code: &str,
    name: &str,
    sender: &mut tube_web::SSESender,
) -> Option<String> {
    let symbol = symbol_from_code(code);
    let display = if name.is_empty() {
        code.to_string()
    } else {
        format!("{} {}", code, name)
    };
    let mut parts: Vec<String> = Vec::new();

    let s = serde_json::json!({"type": "data_loading", "content": format!("正在获取 {} 实时行情...", display)});
    let _ = sender
        .send_data(&serde_json::to_string(&s).unwrap_or_default())
        .await;

    let quote = match dsa_agent::tools::data_tools::DataTools::get_realtime_quote(&symbol).await {
        Ok(q) => q,
        Err(_) => {
            let alt = if symbol.starts_with("sh") {
                format!("sz{}", code)
            } else {
                format!("sh{}", code)
            };
            dsa_agent::tools::data_tools::DataTools::get_realtime_quote(&alt)
                .await
                .ok()?
        }
    };

    let price = quote
        .get("price")
        .or_else(|| quote.get("current_price"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let change_pct = quote
        .get("changePercent")
        .or_else(|| quote.get("change_pct"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let turnover = quote
        .get("turnoverRate")
        .or_else(|| quote.get("turnover_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let volume = quote.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let amount = quote.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let high = quote.get("high").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let low = quote.get("low").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let open = quote.get("open").and_then(|v| v.as_f64()).unwrap_or(0.0);

    parts.push(format!(
        "【实时行情 {}】\n当前价: {:.2}, 涨跌幅: {:.2}%, 开盘: {:.2}, 最高: {:.2}, 最低: {:.2}\n换手率: {:.2}%, 成交量: {:.0}, 成交额: {:.0}\n数据获取时间: {}",
        display, price, change_pct, open, high, low, turnover, volume, amount,
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    ));

    let s = serde_json::json!({"type": "data_loading", "content": "正在获取K线和技术指标..."});
    let _ = sender
        .send_data(&serde_json::to_string(&s).unwrap_or_default())
        .await;

    if let Ok(kline_result) =
        dsa_agent::tools::data_tools::DataTools::get_kline_data(code, "daily").await
    {
        if let Some(kline_data) = kline_result
            .get("data")
            .and_then(|d| tube::Value::as_array(&d.clone()))
        {
            if !kline_data.is_empty() {
                let trend =
                    dsa_agent::tools::analysis_tools::AnalysisTools::analyze_trend(&kline_data);
                let vol_analysis =
                    dsa_agent::tools::analysis_tools::AnalysisTools::analyze_volume(&kline_data);
                let trend_dir = trend
                    .get("trend")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();
                let trend_str = trend
                    .get("strength")
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0);
                let vol_sig = vol_analysis
                    .get("signal")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let last_n: Vec<_> = kline_data.iter().rev().take(5).cloned().collect();
                let kline_summary: Vec<String> = last_n
                    .iter()
                    .filter_map(|bar| {
                        let date = bar
                            .get("日期")
                            .or_else(|| bar.get("date"))
                            .and_then(|d| d.as_str())
                            .unwrap_or_default();
                        let close = bar
                            .get("收盘")
                            .or_else(|| bar.get("close"))
                            .and_then(|c| c.as_f64())
                            .unwrap_or(0.0);
                        let vol = bar
                            .get("成交量")
                            .or_else(|| bar.get("volume"))
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        if !date.is_empty() {
                            Some(format!("{}: 收盘{:.2} 量{:.0}", date, close, vol))
                        } else {
                            None
                        }
                    })
                    .collect();
                let last_kline_date: String = last_n.iter().filter_map(|bar| {
                    bar.get("日期").or_else(|| bar.get("date")).and_then(|d| d.as_str()).map(|s| s.to_string())
                }).next().unwrap_or_default();
                let freshness = if !last_kline_date.is_empty() {
                    format!("\n{}", dsa_core::utils::data_freshness_warning(&last_kline_date, "K线"))
                } else {
                    String::new()
                };
                parts.push(format!(
                    "【技术分析】\n趋势: {} (强度{:.1}), 量能信号: {}\n近5日K线:\n{}{}",
                    trend_dir,
                    trend_str,
                    vol_sig,
                    kline_summary.join("\n"),
                    freshness
                ));
            }
        }
    }

    let s = serde_json::json!({"type": "data_loading", "content": "正在获取最新新闻..."});
    let _ = sender
        .send_data(&serde_json::to_string(&s).unwrap_or_default())
        .await;

    let search = dsa_agent::tools::search_tools::SearchTools::new();
    if let Ok(news_result) = search
        .search_stock_news(&format!("{} {}", code, name))
        .await
    {
        if let Some(items) = news_result
            .get("results")
            .and_then(|v| tube::Value::as_array(&v.clone()))
        {
            let headlines: Vec<String> = items
                .iter()
                .take(3)
                .filter_map(|n| {
                    let title = n.get("title").and_then(|t| t.as_str()).unwrap_or_default();
                    if !title.is_empty() {
                        Some(format!("- {}", title))
                    } else {
                        None
                    }
                })
                .collect();
            if !headlines.is_empty() {
                parts.push(format!("【最新新闻】\n{}", headlines.join("\n")));
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

async fn fetch_market_context(sender: &mut tube_web::SSESender) -> Option<String> {
    let s = serde_json::json!({"type": "data_loading", "content": "正在获取大盘数据..."});
    let _ = sender
        .send_data(&serde_json::to_string(&s).unwrap_or_default())
        .await;

    let overview = dsa_agent::tools::market_tools::MarketTools::get_market_overview()
        .await
        .ok()?;
    let mut parts: Vec<String> = Vec::new();

    for (key, label) in [
        ("shanghai", "上证指数"),
        ("shenzhen", "深证成指"),
        ("chinext", "创业板指"),
    ] {
        if let Some(idx) = overview.get(key) {
            let price = idx
                .get("close")
                .or_else(|| idx.get("price"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let change = idx
                .get("changePercent")
                .or_else(|| idx.get("change_pct"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let amount = idx.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if price > 0.0 {
                parts.push(format!(
                    "{}: {:.2} ({:.2}%) 成交额: {:.0}亿",
                    label,
                    price,
                    change,
                    amount / 1e8
                ));
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("【大盘指数】\n{}", parts.join("\n")))
    }
}

// ===== 主入口 =====

pub async fn chat_stream(req: HttpRequest, payload: web::Payload) -> HttpResponse {
    let param = tube_web::parse_request(req.clone(), payload).await;
    let (mut sender, receiver) = sse_channel(10);

    let message = param
        .value
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let session_id = param
        .value
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let skill_name = param
        .value
        .get("skill")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    actix_rt::spawn(async move {
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();

        if api_key.is_empty() {
            let msg = serde_json::json!({"type": "error", "content": "API Key未配置"});
            let _ = sender
                .send_data(&serde_json::to_string(&msg).unwrap_or_default())
                .await;
            let _ = sender.done("{}").await;
            return;
        }

        let llm_provider = match ai_llm_kit::LlmProvider::instance(&conf.llm.provider) {
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
        let llm = ai_llm_kit::LlmFactory::create(llm_provider, &api_key);

        let msg = serde_json::json!({"type": "thinking"});
        let _ = sender
            .send_data(&serde_json::to_string(&msg).unwrap_or_default())
            .await;

        // ===== 意图识别 + 工具调用 =====
        let intent = parse_intent(&message).await;
        let time_ctx = dsa_core::utils::current_time_context();
        let (system_prompt, data_context, detected_code) = match &intent {
            ChatIntent::StockQuery { code, name } => {
                let sp = if skill_name.is_empty() {
                    format!("你是一位资深证券分析师助手，擅长回答股票分析、技术指标、市场趋势等问题。请用中文回答。如果提供了实时数据，请基于数据进行分析。\n\n{}\n重要: 所有分析和建议必须针对当下时间，不得基于过时数据做出判断。", time_ctx)
                } else {
                    let sd = match skill_name.as_str() {
                        "bull_trend" => "多头趋势策略。重点分析趋势方向、均线支撑，给出顺势操作建议。",
                        "shrink_pullback" => "缩量回调策略。重点分析量价关系、回调深度，判断是否为健康回调。",
                        "chip_focus" => "筹码集中策略。重点分析筹码分布、主力动向、控盘程度。",
                        "no_chase" => "不追高策略。重点评估当前位置风险，给出合理买入区间和止损建议。",
                        _ => &format!("当前用户选择了{}策略，请在分析中侧重该策略视角。", skill_name),
                    };
                    format!("你是一位资深证券分析师助手，当前分析视角为：{}。请用中文回答，结合实时数据和该策略给出针对性建议。\n\n{}\n重要: 所有分析和建议必须针对当下时间，不得基于过时数据做出判断。", sd, time_ctx)
                };
                let ctx = fetch_stock_context(code, name, &mut sender).await;
                (sp, ctx, code.clone())
            }
            ChatIntent::MarketOverview => {
                let sp = format!("你是一位资深证券分析师助手，擅长分析大盘走势和市场情绪。请用中文回答，基于提供的指数数据给出市场研判。\n\n{}\n重要: 所有分析必须针对当下市场环境，不得套用历史结论。", time_ctx);
                let ctx = fetch_market_context(&mut sender).await;
                (sp, ctx, String::new())
            }
            ChatIntent::SectorQuery { keyword } => {
                let sp = format!("你是一位资深证券分析师助手，用户关注\"{}\"相关板块。请用中文回答，分析该板块当下的投资机会和风险。\n\n{}\n重要: 分析必须基于当前市场环境，不得套用过时结论。", keyword, time_ctx);
                let s = serde_json::json!({"type": "data_loading", "content": format!("正在获取{}板块数据...", keyword)});
                let _ = sender.send_data(&serde_json::to_string(&s).unwrap_or_default()).await;
                let search = dsa_agent::tools::search_tools::SearchTools::new();
                let ctx = search.search_stock_news(keyword).await.ok().and_then(|r| {
                    let items = r.get("results").and_then(|v| tube::Value::as_array(&v.clone()))?;
                    let headlines: Vec<String> = items.iter().take(5).filter_map(|n| {
                        let t = n.get("title").and_then(|t| t.as_str()).unwrap_or_default();
                        if !t.is_empty() { Some(format!("- {}", t)) } else { None }
                    }).collect();
                    if headlines.is_empty() { None } else { Some(format!("【{}相关新闻】\n{}", keyword, headlines.join("\n"))) }
                });
                (sp, ctx, String::new())
            }
            ChatIntent::General => {
                (format!("你是一位资深证券分析师助手，擅长回答股票分析、技术指标、市场趋势等问题。请用中文回答。\n\n{}\n如果涉及具体股票或市场分析，请基于当下时间判断，不得使用过时数据。", time_ctx), None, String::new())
            }
        };

        let user_content = if let Some(ctx) = data_context {
            format!(
                "{}\n\n---\n以下是实时市场数据，请基于这些数据回答：\n{}",
                message, ctx
            )
        } else {
            message.clone()
        };

        let body = value!({
            "model": &conf.llm.model,
            "messages": [
                {"role": "system", "content": &system_prompt},
                {"role": "user", "content": &user_content}
            ],
        });

        let start = std::time::Instant::now();
        match llm.chat(&body).await {
            Ok(resp) => {
                let elapsed = start.elapsed().as_millis() as i64;
                let content = resp
                    .get("choices")
                    .and_then(|c| tube::Value::as_array(&c.clone()))
                    .and_then(|a| a.first().cloned())
                    .and_then(|f| f.get("message").cloned())
                    .and_then(|m| m.get("content").and_then(|c| c.as_str()))
                    .unwrap_or_default();

                let usage_default = value!({});
                let usage = resp.get("usage").unwrap_or(&usage_default);
                let pt = usage
                    .get("prompt_tokens")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as i32;
                let ct = usage
                    .get("completion_tokens")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as i32;

                dsa_core::utils::record_llm_usage(
                    &conf.llm.provider,
                    &conf.llm.model,
                    &format!(
                        "chat_stream{}",
                        if skill_name.is_empty() {
                            "".to_string()
                        } else {
                            format!("/{}", skill_name)
                        }
                    ),
                    pt,
                    ct,
                    elapsed,
                    &detected_code,
                );
                dsa_core::utils::record_conversation_message(
                    &session_id,
                    "user",
                    &message,
                    &conf.llm.provider,
                    &conf.llm.model,
                    0,
                    0,
                );
                dsa_core::utils::record_conversation_message(
                    &session_id,
                    "assistant",
                    &content,
                    &conf.llm.provider,
                    &conf.llm.model,
                    pt,
                    ct,
                );

                let json_content = serde_json::to_string(&content).unwrap_or_default();
                let out = serde_json::json!({"type": "message", "content": serde_json::from_str::<serde_json::Value>(&json_content).unwrap_or_default()});
                let _ = sender
                    .send_data(&serde_json::to_string(&out).unwrap_or_default())
                    .await;
            }
            Err(e) => {
                let out = serde_json::json!({"type": "error", "content": format!("{}", e)});
                let _ = sender
                    .send_data(&serde_json::to_string(&out).unwrap_or_default())
                    .await;
            }
        }
        let _ = sender.done("{}").await;
    });

    receiver.respond_to(&req)
}
