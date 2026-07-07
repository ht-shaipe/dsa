//! ReAct执行器 - Reason+Act循环，LLM自主调用工具直到得出最终答案

use dsa_core::{DsaError, DsaResult};
use tube::Value;

use ai_llm_kit::LlmService;

use crate::tools::analysis_tools::AnalysisTools;
use crate::tools::backtest_tools::BacktestTools;
use crate::tools::chip_tools::ChipTools;
use crate::tools::data_tools::DataTools;
use crate::tools::history_tools::HistoryTools;
use crate::tools::market_tools::MarketTools;
use crate::tools::pattern_tools::PatternTools;
use crate::tools::portfolio_tools::PortfolioTools;
use crate::tools::search_tools::SearchTools;

/// ReAct执行器 - 实现Think→Act→Observe循环
pub struct ReactExecutor {
    llm: Box<dyn LlmService>,
    model: String,
    max_iterations: usize,
}

impl ReactExecutor {
    pub fn new(llm: Box<dyn LlmService>, model: &str) -> Self {
        let conf = dsa_core::get_global_config();
        Self {
            llm,
            model: model.to_string(),
            max_iterations: conf.agent.max_steps as usize,
        }
    }

    fn tool_definitions() -> Vec<Value> {
        vec![
            value!({"name": "get_realtime_quote", "description": "获取股票实时行情", "parameters": {"stock_code": {"type": "string", "description": "股票代码"}}}),
            value!({"name": "get_kline_data", "description": "获取K线数据", "parameters": {"code": {"type": "string"}, "period": {"type": "string"}}}),
            value!({"name": "analyze_trend", "description": "分析趋势指标", "parameters": {"kline_data": {"type": "array"}}}),
            value!({"name": "analyze_volume", "description": "分析成交量", "parameters": {"kline_data": {"type": "array"}}}),
            value!({"name": "analyze_patterns", "description": "识别K线形态", "parameters": {"kline_data": {"type": "array"}}}),
            value!({"name": "get_chip_distribution", "description": "获取筹码分布", "parameters": {"code": {"type": "string"}}}),
            value!({"name": "search_stock_news", "description": "搜索股票新闻", "parameters": {"code": {"type": "string"}}}),
            value!({"name": "get_market_overview", "description": "获取市场概览", "parameters": {}}),
            value!({"name": "get_hot_sectors", "description": "获取热门板块", "parameters": {}}),
            value!({"name": "get_portfolio_snapshot", "description": "获取组合快照", "parameters": {"code": {"type": "string"}}}),
            value!({"name": "get_capital_flow", "description": "获取资金流向", "parameters": {"code": {"type": "string"}}}),
            value!({"name": "get_analysis_context", "description": "获取历史分析上下文", "parameters": {"code": {"type": "string"}, "limit": {"type": "integer"}}}),
            value!({"name": "get_backtest_summary", "description": "获取回测摘要", "parameters": {"code": {"type": "string"}}}),
        ]
    }

    async fn execute_tool(&self, tool_name: &str, args: &Value) -> DsaResult<Value> {
        match tool_name {
            "get_realtime_quote" => {
                let code = args.get("stock_code")
                    .or_else(|| args.get("code"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                DataTools::get_realtime_quote(&code).await
                    .map_err(|e| DsaError::StockData(format!("获取行情失败: {}", e)))
            }
            "get_kline_data" => {
                let code = args.get("code").and_then(|v| v.as_str()).unwrap_or_default();
                let period = args.get("period").and_then(|v| v.as_str()).unwrap_or_else(|| "daily".to_string());
                DataTools::get_kline_data(&code, &period).await
                    .map_err(|e| DsaError::StockData(format!("获取K线失败: {}", e)))
            }
            "analyze_trend" => {
                let data = args.get("kline_data")
                    .and_then(|v| Value::as_array(v))
                    .unwrap_or_default();
                Ok(AnalysisTools::analyze_trend(&data))
            }
            "analyze_volume" => {
                let data = args.get("kline_data")
                    .and_then(|v| Value::as_array(v))
                    .unwrap_or_default();
                Ok(AnalysisTools::analyze_volume(&data))
            }
            "analyze_patterns" => {
                let data = args.get("kline_data")
                    .and_then(|v| Value::as_array(v))
                    .unwrap_or_default();
                Ok(PatternTools::analyze_patterns(&data))
            }
            "get_chip_distribution" => {
                let code = args.get("code").and_then(|v| v.as_str()).unwrap_or_default();
                Ok(ChipTools::get_chip_distribution(&code))
            }
            "search_stock_news" => {
                let code = args.get("code").and_then(|v| v.as_str()).unwrap_or_default();
                let tools = SearchTools::new();
                tools.search_stock_news(&code).await
                    .map_err(|e| DsaError::StockData(format!("搜索新闻失败: {}", e)))
            }
            "get_market_overview" => {
                MarketTools::get_market_overview().await
                    .map_err(|e| DsaError::StockData(format!("获取市场概览失败: {}", e)))
            }
            "get_hot_sectors" => {
                MarketTools::get_hot_sectors().await
                    .map_err(|e| DsaError::StockData(format!("获取板块失败: {}", e)))
            }
            "get_portfolio_snapshot" => {
                let code = args.get("code").and_then(|v| v.as_str()).unwrap_or_default();
                Ok(PortfolioTools::get_portfolio_snapshot(&code))
            }
            "get_capital_flow" => {
                let code = args.get("code").and_then(|v| v.as_str()).unwrap_or_default();
                PortfolioTools::get_capital_flow(&code).await
            }
            "get_analysis_context" => {
                let code = args.get("code").and_then(|v| v.as_str()).unwrap_or_default();
                let limit = args.get("limit")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(5.0) as i64;
                Ok(HistoryTools::get_analysis_context(&code, limit))
            }
            "get_backtest_summary" => {
                let code = args.get("code").and_then(|v| v.as_str()).unwrap_or_default();
                Ok(BacktestTools::get_backtest_summary(&code))
            }
            _ => Err(DsaError::Agent(format!("未知工具: {}", tool_name))),
        }
    }

    /// 运行ReAct循环: Think → Act → Observe → 重复直到Final Answer
    pub async fn run(&self, query: &str, context: &Value) -> DsaResult<Value> {
        let tools_json = serde_json::to_string(&Self::tool_definitions())
            .unwrap_or_else(|_| "[]".to_string());

        let system_prompt = format!(
            "你是一个股票分析助手，可以使用工具来获取数据和分析。\n\
             可用工具:\n{}\n\
             请按以下格式思考和回答:\n\
             Thought: 你的思考过程\n\
             Action: 工具名称\n\
             Action Input: 工具参数JSON\n\
             Observation: 工具返回结果\n\
             ... (重复 Thought/Action/Observation 直到得出结论)\n\
             Thought: 我现在知道最终答案了\n\
             Final Answer: 你的最终分析和建议\n\n\
             上下文信息: {}",
            tools_json,
            serde_json::to_string(context).unwrap_or_else(|_| "{}".to_string()),
        );

        let mut messages: Vec<Value> = vec![
            value!({"role": "system", "content": system_prompt}),
            value!({"role": "user", "content": query}),
        ];

        let mut iteration = 0;
        let mut tool_results: Vec<Value> = Vec::new();

        loop {
            iteration += 1;
            if iteration > self.max_iterations {
                return Ok(value!({
                    "status": "ok",
                    "data": {
                        "answer": format!("达到最大迭代次数({}), 分析过程中断", self.max_iterations),
                        "toolCalls": tool_results.len() as i64,
                        "iterations": iteration as i64,
                    }
                }));
            }

            // 调用LLM
            let body = value!({
                "model": &self.model,
                "messages": messages.clone(),
                "temperature": 0.7,
            });

            let response = self.llm.chat(&body).await
                .map_err(|e| DsaError::LlmAnalysis(format!("ReAct LLM调用失败: {}", e)))?;

            let content = response.get("choices")
                .and_then(|c| Value::as_array(c))
                .and_then(|a| a.first().cloned())
                .and_then(|f| f.get("message").cloned())
                .and_then(|m| m.get("content").and_then(|c| c.as_str()))
                .unwrap_or_default()
                .to_string();

            // 检查是否为最终答案
            if let Some(answer) = Self::extract_final_answer(&content) {
                return Ok(value!({
                    "status": "ok",
                    "data": {
                        "answer": answer,
                        "toolCalls": tool_results.len() as i64,
                        "iterations": iteration as i64,
                        "rawResponse": content,
                    }
                }));
            }

            // 尝试提取工具调用
            if let Some((tool_name, tool_input)) = Self::extract_tool_call(&content) {
                let observation = match self.execute_tool(&tool_name, &tool_input).await {
                    Ok(result) => serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()),
                    Err(e) => format!("工具执行错误: {}", e),
                };

                tool_results.push(value!({
                    "iteration": iteration as i64,
                    "tool": tool_name,
                    "input": tool_input,
                    "output": observation.clone(),
                }));

                // 追加到消息列表
                messages.push(value!({"role": "assistant", "content": content}));
                messages.push(value!({"role": "user", "content": format!("Observation: {}", observation)}));
            } else {
                // 未找到工具调用，当作最终答案
                return Ok(value!({
                    "status": "ok",
                    "data": {
                        "answer": content,
                        "toolCalls": tool_results.len() as i64,
                        "iterations": iteration as i64,
                    }
                }));
            }
        }
    }

    fn extract_final_answer(text: &str) -> Option<String> {
        if let Some(pos) = text.find("Final Answer:") {
            Some(text[pos + 13..].trim().to_string())
        } else if let Some(pos) = text.find("最终答案:") {
            Some(text[pos + 5..].trim().to_string())
        } else {
            None
        }
    }

    fn extract_tool_call(text: &str) -> Option<(String, Value)> {
        let action = if let Some(pos) = text.find("Action:") {
            text[pos + 7..].lines().next()?.trim().to_string()
        } else if let Some(pos) = text.find("调用工具:") {
            text[pos + 5..].lines().next()?.trim().to_string()
        } else {
            return None;
        };

        let input_str = if let Some(pos) = text.find("Action Input:") {
            text[pos + 13..].lines().next().unwrap_or("{}").trim()
        } else if let Some(pos) = text.find("工具参数:") {
            text[pos + 5..].lines().next().unwrap_or("{}").trim()
        } else {
            "{}"
        };

        let input: Value = serde_json::from_str(input_str).unwrap_or(value!({}));
        Some((action, input))
    }
}
