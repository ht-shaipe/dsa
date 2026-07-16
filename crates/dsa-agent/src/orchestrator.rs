//! Agent 编排器 - 协调多Agent对话流程

use std::sync::Arc;

use ai_llm_kit::{LlmFactory, LlmProvider, LlmService, StreamCallback, StreamCallbackFuture};
use dsa_core::{DsaError, DsaResult};
use tube::Value;

use crate::agents::base_agent::BaseAgent;
use crate::agents::{decision_agent, intel_agent, portfolio_agent, risk_agent, technical_agent};
use crate::memory::AgentMemory;
use crate::skills::defaults::default_skills;
use crate::skills::router::SkillRouter;
use crate::tools::data_tools::DataTools;

pub struct Orchestrator {
    memory: AgentMemory,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            memory: AgentMemory::new(),
        }
    }

    /// 创建LLM实例的辅助方法
    fn create_llm() -> DsaResult<Box<dyn LlmService>> {
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Err(DsaError::LlmAnalysis("API Key 未配置".to_string()));
        }
        let provider = LlmProvider::instance(&conf.llm.provider)
            .map_err(|e| DsaError::LlmAnalysis(format!("不支持的LLM: {}", e)))?;
        Ok(LlmFactory::create(provider, &api_key))
    }

    /// 重新创建LLM实例（用于多Agent并行）
    fn recreate_llm() -> Box<dyn LlmService> {
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();
        if let Ok(provider) = LlmProvider::instance(&conf.llm.provider) {
            LlmFactory::create(provider, &api_key)
        } else {
            LlmFactory::create(LlmProvider::DeepSeek, &api_key)
        }
    }

    /// 获取模型名
    fn get_model() -> String {
        dsa_core::get_global_config().llm.model.clone()
    }

    pub async fn dispatch(&self, _module: &str, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "chat" => self.chat(params).await,
            "stream" => self.stream(params).await,
            "models" => self.models().await,
            "history" => self.history(params).await,
            "pipeline" => self.run_pipeline(params).await,
            "skills" => self.skills().await,
            "strategies" => self.strategies().await,
            _ => Err(DsaError::ApiRouting(format!("agent不支持方法: {}", method))),
        }
    }

    async fn chat(&self, params: &Value) -> DsaResult<Value> {
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if message.is_empty() {
            return Err(DsaError::Validation("请输入消息".to_string()));
        }

        let llm = Self::create_llm()?;
        let conf = dsa_core::get_global_config();

        let system_prompt = format!(
            "你是一位资深证券分析师助手，擅长回答股票分析、技术指标、市场趋势等问题。请用中文回答。\n\n{}\n重要: 如果涉及具体股票或市场分析，必须基于当下时间判断，不得使用过时数据。",
            dsa_core::utils::current_time_context()
        );

        let body = value!({
            "model": &conf.llm.model,
            "messages": [
                {"role": "system", "content": &system_prompt},
                {"role": "user", "content": &message}
            ],
            "temperature": 0.7,
        });

        let start = std::time::Instant::now();
        let response = llm
            .chat(&body)
            .await
            .map_err(|e| DsaError::LlmAnalysis(format!("LLM调用失败: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as i64;

        let content = response
            .get("choices")
            .and_then(|c| Value::as_array(c))
            .and_then(|a| a.first().cloned())
            .and_then(|f| f.get("message").cloned())
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .unwrap_or_default();

        let usage_default = value!({});
        let usage = response.get("usage").unwrap_or(&usage_default);
        let prompt_tokens = usage
            .get("prompt_tokens")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i32;
        let completion_tokens = usage
            .get("completion_tokens")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i32;

        let session_id = params
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        dsa_core::utils::record_llm_usage(
            &conf.llm.provider,
            &conf.llm.model,
            "chat",
            prompt_tokens,
            completion_tokens,
            elapsed,
            "",
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
            prompt_tokens,
            completion_tokens,
        );

        Ok(value!({
            "role": "assistant",
            "content": content,
        }))
    }

    async fn stream(&self, params: &Value) -> DsaResult<Value> {
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if message.is_empty() {
            return Err(DsaError::Validation("请输入消息".to_string()));
        }

        let llm = Self::create_llm()?;
        let conf = dsa_core::get_global_config();

        let system_prompt = format!(
            "你是一位资深证券分析师助手，擅长回答股票分析、技术指标、市场趋势等问题。请用中文回答。\n\n{}\n重要: 如果涉及具体股票或市场分析，必须基于当下时间判断，不得使用过时数据。",
            dsa_core::utils::current_time_context()
        );

        let body = value!({
            "model": &conf.llm.model,
            "messages": [
                {"role": "system", "content": &system_prompt},
                {"role": "user", "content": message}
            ],
            "temperature": 0.7,
            "stream": true,
        });

        // 创建流式回调
        let callback: Arc<StreamCallback> =
            Arc::new(|chunk: String, done: bool| -> StreamCallbackFuture {
                Box::pin(async move {
                    if done {
                        Ok(value!({"type": "done"}))
                    } else {
                        Ok(value!({"type": "chunk", "content": chunk}))
                    }
                })
            });

        let result = llm
            .chat_stream(&body, callback)
            .await
            .map_err(|e| DsaError::LlmAnalysis(format!("LLM流式调用失败: {}", e)))?;

        Ok(result)
    }

    async fn models(&self) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Ok(value!([]));
        }

        let provider = LlmProvider::instance(&conf.llm.provider)
            .map_err(|e| DsaError::LlmAnalysis(format!("不支持的LLM: {}", e)))?;
        let llm = LlmFactory::create(provider, &api_key);

        let models = llm
            .models()
            .await
            .map_err(|e| DsaError::LlmAnalysis(format!("获取模型列表失败: {}", e)))?;

        Ok(models)
    }

    async fn history(&self, params: &Value) -> DsaResult<Value> {
        let session_id = dsa_core::utils::param_string(params, "sessionId");
        let connector = match dsa_core::utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => {
                let messages: Vec<Value> = self.memory.get_messages().to_vec();
                return Ok(Value::Array(messages));
            }
        };

        let (sql, p) = if session_id.is_empty() {
            ("SELECT session_id, role, content, create_time FROM conversation_messages ORDER BY create_time ASC LIMIT 500".to_string(), vec![])
        } else {
            ("SELECT session_id, role, content, create_time FROM conversation_messages WHERE session_id = :sid ORDER BY create_time ASC LIMIT 500".to_string(),
             vec![("sid".to_string(), Value::from(session_id.as_str()))])
        };
        let messages: Vec<Value> = dsa_core::db::query_rows(&sql, p, &connector)
            .unwrap_or_default()
            .iter()
            .map(|r| {
                let mut v = r.clone();
                if let Value::Object(ref mut map) = v {
                    if let Some(Value::Text(ref mut s)) = map.get_mut("content") {
                        let clean: String = s
                            .replace('\r', "")
                            .replace('\t', " ")
                            .chars()
                            .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
                            .collect();
                        *s = clean;
                    }
                }
                v
            })
            .collect();
        Ok(Value::Array(messages))
    }

    /// 运行完整的多Agent分析管道
    /// 流程: 数据采集 → 技术分析 → 情报分析 → 风控评估 → 综合决策 → 组合建议
    pub async fn run_pipeline(&self, params: &Value) -> DsaResult<Value> {
        let code = params
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or_default();

        if code.is_empty() {
            return Err(DsaError::Validation("分析管道需要股票代码".to_string()));
        }

        let conf = dsa_core::get_global_config();
        let model = Self::get_model();

        let arch = &conf.agent.arch;
        let mode = &conf.agent.orchestrator_mode;

        if arch == "react" {
            return self.run_react_pipeline(&code, &model, &conf).await;
        }

        if mode == "quick" {
            return self.run_quick_pipeline(&code, &model, &conf).await;
        }

        if mode == "specialist" {
            return self.run_specialist_pipeline(&code, &model, &conf).await;
        }

        if arch == "single" {
            return self.run_single_pipeline(&code, &model, &conf).await;
        }

        self.run_multi_pipeline(&code, &model, &conf).await
    }

    async fn run_single_pipeline(
        &self,
        code: &str,
        model: &str,
        conf: &dsa_core::config::AppConfig,
    ) -> DsaResult<Value> {
        let llm = Self::create_llm()?;
        let system_prompt = format!(
            "你是一位资深证券分析师，请对股票{}进行全方位分析，包括技术面、基本面、情报面、风控和操作建议。以JSON格式输出。\n\n{}\n重要: 所有分析必须基于当前时间点，基于最新数据判断，不得使用过时结论。",
            code,
            dsa_core::utils::current_time_context()
        );

        let quote_result = DataTools::get_realtime_quote(&format!("sz{}", code)).await;
        let quote_result = match quote_result {
            Ok(q) => Ok(q),
            Err(_) => DataTools::get_realtime_quote(&format!("sh{}", code)).await,
        };
        let kline_result = DataTools::get_kline_data(code, "daily").await;

        let current_price = quote_result
            .as_ref()
            .ok()
            .and_then(|q| q.get("current_price").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);

        let kline_summary = kline_result
            .as_ref()
            .ok()
            .and_then(|k| k.get("data").and_then(|d| tube::Value::as_array(d)))
            .map(|arr| format!("最近{}根K线", arr.len()))
            .unwrap_or_default();

        let user_msg = format!(
            "股票代码: {}\n当前价: {:.2}\nK线: {}\n请给出分析结果。",
            code, current_price, kline_summary
        );

        let start = std::time::Instant::now();
        let body = value!({
            "model": model,
            "messages": [
                {"role": "system", "content": &system_prompt},
                {"role": "user", "content": &user_msg}
            ],
            "temperature": 0.7,
        });

        let response = llm
            .chat(&body)
            .await
            .map_err(|e| DsaError::LlmAnalysis(format!("LLM调用失败: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as i64;

        let content = response
            .get("choices")
            .and_then(|c| tube::Value::as_array(c))
            .and_then(|a| a.first().cloned())
            .and_then(|f| f.get("message").cloned())
            .and_then(|m| m.get("content").and_then(|c| c.as_str()))
            .unwrap_or_default();

        let usage_default = value!({});
        let usage = response.get("usage").unwrap_or(&usage_default);
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
            model,
            "pipeline_single",
            pt,
            ct,
            elapsed,
            code,
        );

        Ok(value!({
            "code": code,
            "arch": "single",
            "analysis": content,
        }))
    }

    async fn run_multi_pipeline(
        &self,
        code: &str,
        model: &str,
        conf: &dsa_core::config::AppConfig,
    ) -> DsaResult<Value> {
        let max_steps = conf.agent.max_steps as usize;
        let mut step = 0usize;

        // 1. 数据采集阶段
        let quote_result = DataTools::get_realtime_quote(&format!("sz{}", code)).await;
        let quote_result = match quote_result {
            Ok(q) => Ok(q),
            Err(_) => DataTools::get_realtime_quote(&format!("sh{}", code)).await,
        };

        let kline_result = DataTools::get_kline_data(&code, "daily").await;

        // 构建基础上下文
        let current_price = quote_result
            .as_ref()
            .ok()
            .and_then(|q| q.get("current_price").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);

        let change_percent = quote_result
            .as_ref()
            .ok()
            .and_then(|q| q.get("change_percent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);

        let turnover_rate = quote_result
            .as_ref()
            .ok()
            .and_then(|q| q.get("turnover_rate").and_then(|t| t.as_f64()))
            .unwrap_or(0.0);

        let kline_data = kline_result
            .as_ref()
            .ok()
            .and_then(|k| k.get("data").and_then(|d| Value::as_array(d)))
            .unwrap_or_default();

        // 技术指标本地计算
        let trend_analysis =
            crate::tools::analysis_tools::AnalysisTools::analyze_trend(&kline_data);
        let volume_analysis =
            crate::tools::analysis_tools::AnalysisTools::analyze_volume(&kline_data);

        let trend_str = trend_analysis
            .get("trend")
            .and_then(|t| t.as_str())
            .unwrap_or_default();
        let vol_signal = volume_analysis
            .get("signal")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let market_trend = trend_analysis
            .get("direction")
            .and_then(|d| d.as_str())
            .unwrap_or_default();

        // 技能评估
        let chip_concentration = quote_result
            .as_ref()
            .ok()
            .and_then(|q| q.get("chipConcentration").and_then(|v| v.as_f64()))
            .or_else(|| {
                if conf.stock.enable_chip_distribution {
                    None
                } else {
                    None
                }
            })
            .unwrap_or(0.0);

        let skill_context = value!({
            "code": &code,
            "trend": &trend_str,
            "volumeSignal": &vol_signal,
            "changePercent": change_percent,
            "chipConcentration": chip_concentration,
        });

        let router = SkillRouter::new(default_skills());
        let skill_results = router.route(&skill_context);

        // 2. 技术分析Agent
        step += 1;
        if step > max_steps {
            return Err(DsaError::Agent("已达到最大步骤数限制".to_string()));
        }
        let tech_agent = technical_agent::TechnicalAgent::new(Self::recreate_llm(), &model);
        let tech_input = value!({
            "code": &code,
            "klineData": kline_data,
            "trend": trend_analysis.clone(),
            "volume": volume_analysis.clone(),
        });
        let tech_result = tech_agent.process(&tech_input).await?;
        let tech_analysis = tech_result
            .get("llmAnalysis")
            .and_then(|a| a.as_str())
            .unwrap_or_default();

        // 3. 情报分析Agent - 使用真实新闻数据
        step += 1;
        if step > max_steps {
            return Err(DsaError::Agent("已达到最大步骤数限制".to_string()));
        }
        let search_tools = crate::tools::search_tools::SearchTools::new();
        let news_result = search_tools.search_stock_news(&code).await;
        let news_items: Vec<Value> = news_result
            .ok()
            .and_then(|r| r.get("results").and_then(|v| tube::Value::as_array(v)))
            .unwrap_or_default();

        let industry: String = quote_result
            .as_ref()
            .ok()
            .and_then(|q| q.get("industry").and_then(|v| v.as_str()))
            .unwrap_or_default();

        let intel_agent = intel_agent::IntelAgent::new(Self::recreate_llm(), &model);
        let intel_input = value!({
            "code": &code,
            "industry": industry,
            "news": news_items,
        });
        let intel_result = intel_agent.process(&intel_input).await?;
        let intel_analysis = intel_result
            .get("llmAnalysis")
            .and_then(|a| a.as_str())
            .unwrap_or_default();

        // 4. 风控Agent
        step += 1;
        if step > max_steps {
            return Err(DsaError::Agent("已达到最大步骤数限制".to_string()));
        }
        let risk_agent = risk_agent::RiskAgent::new(Self::recreate_llm(), &model);
        let risk_input = value!({
            "code": &code,
            "currentPrice": current_price,
            "changePercent": change_percent,
            "turnoverRate": turnover_rate,
            "marketTrend": &market_trend,
            "technical": &tech_analysis,
        });
        let risk_result = risk_agent.process(&risk_input).await?;
        let risk_analysis = risk_result
            .get("riskAnalysis")
            .and_then(|a| a.as_str())
            .unwrap_or_default();

        // 5. 决策Agent
        step += 1;
        if step > max_steps {
            return Err(DsaError::Agent("已达到最大步骤数限制".to_string()));
        }
        let decision_agent = decision_agent::DecisionAgent::new(Self::recreate_llm(), &model);
        let decision_input = value!({
            "code": &code,
            "currentPrice": current_price,
            "technical": &tech_analysis,
            "intel": &intel_analysis,
            "risk": &risk_analysis,
        });
        let decision_result = decision_agent.process(&decision_input).await?;
        let decision_analysis = decision_result
            .get("decision")
            .and_then(|d| d.as_str())
            .unwrap_or_default();

        let stock_name_for_decision = quote_result
            .as_ref()
            .ok()
            .and_then(|q| q.get("name").and_then(|n| n.as_str()))
            .unwrap_or_default();
        Self::persist_decision_signal(
            &code,
            &stock_name_for_decision,
            &decision_analysis,
            current_price,
        );

        // 6. 组合Agent - 使用真实持仓数据
        step += 1;
        if step > max_steps {
            return Err(DsaError::Agent("已达到最大步骤数限制".to_string()));
        }
        let (positions, total_assets) = Self::fetch_portfolio_positions(&code);
        let portfolio_agent = portfolio_agent::PortfolioAgent::new(Self::recreate_llm(), &model);
        let portfolio_input = value!({
            "code": &code,
            "decision": &decision_analysis,
            "positions": positions,
            "totalAssets": total_assets,
        });
        let portfolio_result = portfolio_agent.process(&portfolio_input).await?;

        Ok(value!({
            "code": &code,
            "currentPrice": current_price,
            "changePercent": change_percent,
            "technical": tech_result,
            "intel": intel_result,
            "risk": risk_result,
            "decision": decision_result,
            "portfolio": portfolio_result,
            "skills": skill_results,
            "localIndicators": {
                "trend": trend_analysis,
                "volume": volume_analysis,
            },
        }))
    }

    fn persist_decision_signal(code: &str, name: &str, decision_content: &str, current_price: f64) {
        let connector = match dsa_core::utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => return,
        };

        let action = if decision_content.contains("\"buy\"")
            || decision_content.contains("买入")
            || decision_content.contains("\"action\": \"buy\"")
        {
            "buy"
        } else if decision_content.contains("\"sell\"")
            || decision_content.contains("卖出")
            || decision_content.contains("\"action\": \"sell\"")
        {
            "sell"
        } else {
            "hold"
        };

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        let check_sql = "SELECT id FROM decision_signals WHERE stock_code = :code AND action = :action AND signal_date = :date AND status = 1 LIMIT 1";
        if let Ok(existing) = dsa_core::db::query_rows(
            check_sql,
            vec![
                ("code".to_string(), Value::from(code.to_string())),
                ("action".to_string(), Value::from(action.to_string())),
                ("date".to_string(), Value::from(today.clone())),
            ],
            &connector,
        ) {
            if !existing.is_empty() {
                return;
            }
        }

        let sql = "INSERT INTO decision_signals \
             (stock_code, stock_name, signal_date, action, sentiment_score, confidence_level, \
              entry_price, reasoning, evidence, scope_type, status, create_time) \
             VALUES (:code, :name, :date, :action, 0, 'medium', :price, :reasoning, :evidence, 'pipeline', 1, NOW())";
        let _ = dsa_core::db::execute(
            sql,
            vec![
                ("code".to_string(), Value::from(code.to_string())),
                ("name".to_string(), Value::from(name.to_string())),
                ("date".to_string(), Value::from(today)),
                ("action".to_string(), Value::from(action.to_string())),
                ("price".to_string(), Value::from(current_price)),
                (
                    "reasoning".to_string(),
                    Value::from(decision_content.chars().take(500).collect::<String>()),
                ),
                ("evidence".to_string(), Value::from("auto_pipeline")),
            ],
            &connector,
        );
    }

    async fn skills(&self) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let router = SkillRouter::new(default_skills());
        let all = router.evaluate_all(&value!({}));
        let skills: Vec<Value> = all
            .iter()
            .map(|s| {
                let name_string: String =
                    s.get("skill").and_then(|v| v.as_str()).unwrap_or_default();
                let desc: String = match name_string.as_str() {
                    "bull_trend" => "多头趋势策略".to_string(),
                    "shrink_pullback" => "缩量回调策略".to_string(),
                    "chip_focus" => "筹码集中策略".to_string(),
                    "no_chase" => "不追高策略".to_string(),
                    _ => name_string.clone(),
                };
                let enabled =
                    conf.agent.skills.is_empty() || conf.agent.skills.contains(&name_string);
                value!({
                    "name": name_string,
                    "description": desc,
                    "enabled": enabled,
                })
            })
            .collect();
        Ok(Value::Array(skills))
    }

    async fn strategies(&self) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let strategies = vec![
            value!({"name": "dual_low", "description": "双低策略(低市盈率+低市净率)", "enabled": true}),
            value!({"name": "breakout", "description": "突破策略(放量突破平台)", "enabled": true}),
            value!({"name": "value", "description": "价值策略(低估值高分红)", "enabled": true}),
            value!({"name": "momentum", "description": "动量策略(趋势跟踪)", "enabled": true}),
        ];
        let filtered: Vec<Value> = if conf.agent.skills.is_empty() {
            strategies
        } else {
            strategies
                .into_iter()
                .filter(|s| {
                    let name = s.get("name").and_then(|n| n.as_str()).unwrap_or_default();
                    conf.agent.skills.contains(&name.to_string())
                })
                .collect()
        };
        Ok(Value::Array(filtered))
    }

    fn fetch_portfolio_positions(code: &str) -> (Vec<Value>, Option<f64>) {
        let connector = match dsa_core::utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => return (vec![], None),
        };
        let sql = if code.is_empty() {
            "SELECT p.stock_code, p.stock_name, p.quantity, p.avg_cost, p.current_price, \
             p.market_value, p.unrealized_pnl, p.unrealized_pnl_pct, a.initial_capital \
             FROM portfolio_positions p \
             JOIN portfolio_accounts a ON p.account_id = a.id \
             WHERE p.status >= 1 AND a.status >= 1"
                .to_string()
        } else {
            format!(
                "SELECT p.stock_code, p.stock_name, p.quantity, p.avg_cost, p.current_price, \
             p.market_value, p.unrealized_pnl, p.unrealized_pnl_pct, a.initial_capital \
             FROM portfolio_positions p \
             JOIN portfolio_accounts a ON p.account_id = a.id \
             WHERE p.status >= 1 AND a.status >= 1 AND p.stock_code = '{}'",
                code
            )
        };
        match dsa_core::db::query_rows(&sql, vec![], &connector) {
            Ok(rows) => {
                let total = rows.iter().fold(0.0, |acc, p| {
                    acc + p.get("marketValue").and_then(|v| v.as_f64()).unwrap_or(0.0)
                });
                let account_total = rows
                    .first()
                    .and_then(|r| r.get("initialCapital").and_then(|v| v.as_f64()))
                    .filter(|v| *v > 0.0);
                (
                    rows,
                    if total > 0.0 || account_total.is_some() {
                        account_total
                    } else {
                        None
                    },
                )
            }
            Err(_) => (vec![], None),
        }
    }

    /// ReAct模式 - LLM自主调用工具循环
    async fn run_react_pipeline(
        &self,
        code: &str,
        model: &str,
        _conf: &dsa_core::config::AppConfig,
    ) -> DsaResult<Value> {
        let llm = Self::recreate_llm();
        let executor = crate::agents::react_executor::ReactExecutor::new(llm, model);
        let query = format!(
            "请全面分析股票{}的技术面、资金面、消息面，给出操作建议",
            code
        );
        let context = value!({"code": code});
        executor.run(&query, &context).await
    }

    /// Quick模式 - 仅技术分析+简短建议
    async fn run_quick_pipeline(
        &self,
        code: &str,
        model: &str,
        _conf: &dsa_core::config::AppConfig,
    ) -> DsaResult<Value> {
        let kline_result = DataTools::get_kline_data(&code, "daily").await;
        let kline_data = kline_result
            .ok()
            .and_then(|k| k.get("data").and_then(|d| tube::Value::as_array(d)))
            .unwrap_or_default();

        let trend = crate::tools::analysis_tools::AnalysisTools::analyze_trend(&kline_data);
        let volume = crate::tools::analysis_tools::AnalysisTools::analyze_volume(&kline_data);

        let llm = Self::recreate_llm();
        let body = value!({
            "model": model,
            "messages": [
                {"role": "system", "content": "你是一个简明的股票分析助手。请用3-5句话给出简要分析。"},
                {"role": "user", "content": format!("股票{}\n趋势: {}\n量能: {}\n请简要给出操作建议", code,
                    serde_json::to_string(&trend).unwrap_or_default(),
                    serde_json::to_string(&volume).unwrap_or_default())}
            ],
            "temperature": 0.5,
        });
        let start = std::time::Instant::now();
        let response = llm
            .chat(&body)
            .await
            .map_err(|e| DsaError::LlmAnalysis(format!("Quick分析LLM失败: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as i64;
        let analysis = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let conf = dsa_core::get_global_config();
        dsa_core::utils::record_llm_usage_from_response(
            &response,
            &conf.llm.provider,
            model,
            "pipeline_quick",
            elapsed,
            code,
        );

        Ok(value!({
            "mode": "quick",
            "code": code,
            "analysis": analysis,
            "trend": trend,
            "volume": volume,
        }))
    }

    /// Specialist模式 - 技能Agent+策略Agent+深度分析
    async fn run_specialist_pipeline(
        &self,
        code: &str,
        model: &str,
        _conf: &dsa_core::config::AppConfig,
    ) -> DsaResult<Value> {
        let kline_result = DataTools::get_kline_data(&code, "daily").await;
        let kline_data = kline_result
            .ok()
            .and_then(|k| k.get("data").and_then(|d| tube::Value::as_array(d)))
            .unwrap_or_default();

        let quote_result = match DataTools::get_realtime_quote(&format!("sz{}", code)).await {
            Ok(q) => Ok(q),
            Err(_) => DataTools::get_realtime_quote(&format!("sh{}", code)).await,
        };
        let change_pct = quote_result
            .ok()
            .and_then(|q| {
                q.get("changePercent")
                    .or_else(|| q.get("change_pct"))
                    .and_then(|v| v.as_f64())
            })
            .unwrap_or(0.0);

        let trend = crate::tools::analysis_tools::AnalysisTools::analyze_trend(&kline_data);
        let volume = crate::tools::analysis_tools::AnalysisTools::analyze_volume(&kline_data);
        let patterns = crate::tools::pattern_tools::PatternTools::analyze_patterns(&kline_data);
        let chip = crate::tools::chip_tools::ChipTools::get_chip_distribution(code);

        let trend_str = trend
            .get("trend")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string();
        let vol_signal = volume
            .get("signal")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let chip_conc = chip
            .get("concentration")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0);

        // Skill Agent
        let llm = Self::recreate_llm();
        let skill_agent = crate::agents::skill_agent::SkillAgent::new(llm, model);
        let skill_context = value!({
            "code": code,
            "trend": &trend_str,
            "volumeSignal": &vol_signal,
            "changePercent": change_pct,
            "chipConcentration": chip_conc,
        });
        let skill_result = skill_agent
            .evaluate_skills(code, &skill_context)
            .await
            .unwrap_or(value!({}));

        // Strategy Agent
        let strategy_context = value!({
            "trend": &trend_str,
            "changePercent": change_pct,
            "volumeSignal": &vol_signal,
        });
        let strategy_result =
            crate::agents::strategy_agent::StrategyAgent::route_strategy(&strategy_context);

        // Deep LLM analysis with all context
        let llm2 = Self::recreate_llm();
        let body = value!({
            "model": model,
            "messages": [
                {"role": "system", "content": "你是一个专业的股票深度分析助手。综合技术、筹码、技能和策略给出详细分析。"},
                {"role": "user", "content": format!(
                    "股票: {}\n趋势: {}\n量能: {}\n形态: {}\n筹码: {:.1}%\n技能评估: {}\n推荐策略: {}\n请给出深度分析",
                    code,
                    serde_json::to_string(&trend).unwrap_or_default(),
                    serde_json::to_string(&volume).unwrap_or_default(),
                    serde_json::to_string(&patterns).unwrap_or_default(),
                    chip_conc,
                    serde_json::to_string(&skill_result).unwrap_or_default(),
                    serde_json::to_string(&strategy_result).unwrap_or_default()
                )}
            ],
            "temperature": 0.6,
        });
        let start = std::time::Instant::now();
        let response = llm2
            .chat(&body)
            .await
            .map_err(|e| DsaError::LlmAnalysis(format!("Specialist分析LLM失败: {}", e)))?;
        let elapsed = start.elapsed().as_millis() as i64;
        let analysis = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let conf = dsa_core::get_global_config();
        dsa_core::utils::record_llm_usage_from_response(
            &response,
            &conf.llm.provider,
            model,
            "pipeline_specialist",
            elapsed,
            code,
        );

        Ok(value!({
            "mode": "specialist",
            "code": code,
            "analysis": analysis,
            "trend": trend,
            "volume": volume,
            "patterns": patterns,
            "chip": chip,
            "skills": skill_result,
            "strategy": strategy_result,
        }))
    }
}
