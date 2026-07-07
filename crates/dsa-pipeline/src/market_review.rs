//! 大盘综述生成 - 对齐原项目 market_review

use ai_llm_kit::{LlmFactory, LlmProvider, LlmService};
use dsa_core::{DsaError, DsaResult};
use qta_crawler::Real;
use tube::Value;

pub struct MarketReviewGenerator;

impl MarketReviewGenerator {
    pub fn new() -> Self {
        Self
    }

    pub async fn generate(&self, _params: &Value) -> DsaResult<Value> {
        let conf = dsa_core::get_global_config();
        let api_key = conf.resolve_api_key();
        if api_key.is_empty() {
            return Err(DsaError::LlmAnalysis("API Key 未配置".to_string()));
        }

        // 获取三大指数数据
        let real = Real::new();
        let sh = real.get_price("sh000001").await.ok();
        let sz = real.get_price("sz399001").await.ok();
        let cy = real.get_price("sz399006").await.ok();

        // 构建市场上下文
        let mut market_context = String::new();

        if let Some(ref v) = sh {
            let name = v
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| "上证指数".to_string());
            let price = v.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let chg = v
                .get("changePercent")
                .or_else(|| v.get("change_pct"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            market_context.push_str(&format!(
                "{}: {:.2} ({:+.2}%)  ",
                name, price, chg
            ));
        }

        if let Some(ref v) = sz {
            let name = v
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| "深证成指".to_string());
            let price = v.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let chg = v
                .get("changePercent")
                .or_else(|| v.get("change_pct"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            market_context.push_str(&format!(
                "{}: {:.2} ({:+.2}%)  ",
                name, price, chg
            ));
        }

        if let Some(ref v) = cy {
            let name = v
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| "创业板指".to_string());
            let price = v.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let chg = v
                .get("changePercent")
                .or_else(|| v.get("change_pct"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            market_context.push_str(&format!(
                "{}: {:.2} ({:+.2}%)",
                name, price, chg
            ));
        }

        if market_context.is_empty() {
            market_context = "无法获取大盘数据".to_string();
        }

        // 调用LLM生成综述
        let llm_provider = LlmProvider::instance(&conf.llm.provider)
            .map_err(|e| DsaError::LlmAnalysis(format!("不支持的provider: {}", e)))?;
        let llm: Box<dyn LlmService> = LlmFactory::create(llm_provider, &api_key);

        let system_prompt = "你是一位资深市场分析师，请根据提供的指数数据，生成简洁的市场综述报告。\
             包含：1)市场概况 2)板块表现 3)后市展望。输出JSON格式：\
             {\"title\":\"标题\",\"summary\":\"摘要\",\"outlook\":\"展望\",\"sentiment\":\"偏多/偏空/中性\"}";

        let user_prompt = format!("今日A股市场数据：{}\n\n请生成市场综述。", market_context);

        let body = value!({
            "model": &conf.llm.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": &user_prompt}
            ],
            "temperature": 0.7,
        });

        let response = llm
            .chat(&body)
            .await
            .map_err(|e| DsaError::LlmAnalysis(format!("LLM调用失败: {}", e)))?;

        // 提取内容
        let content = response
            .get("choices")
            .and_then(|c| tube::Value::as_array(&c.clone()))
            .and_then(|a| a.first().cloned())
            .and_then(|first| first.get("message").cloned())
            .and_then(|msg| msg.get("content").and_then(|c| c.as_str()))
            .unwrap_or_default();

        // 尝试解析JSON
        let review_data = if content.starts_with('{') {
            serde_json::from_str::<serde_json::Value>(&content)
                .map(|v| v.into())
                .unwrap_or_else(|_| value!({"content": content.as_str()}))
        } else {
            value!({"content": content.as_str()})
        };

        Ok(value!({
            "status": "ok",
            "data": {
                "marketContext": market_context,
                "review": review_data,
                "indices": {
                    "shanghai": sh.unwrap_or(Value::Null),
                    "shenzhen": sz.unwrap_or(Value::Null),
                    "chinext": cy.unwrap_or(Value::Null),
                }
            }
        }))
    }
}
