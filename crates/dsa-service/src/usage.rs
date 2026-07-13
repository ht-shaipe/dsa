use dsa_core::db;
use dsa_core::models::db::LlmUsage as LlmUsageModel;
use dsa_core::utils;
use deck::sqlite::{DataTable, SelectExecutor};
use deck::TableService;

use deck_connector::get_connector;
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct Usage {
    request: RequestParameter,
}

impl DataTable<LlmUsageModel> for Usage {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}

impl TableService<LlmUsageModel> for Usage {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}

impl Usage {
    pub fn new(param: &RequestParameter) -> Self {
        Usage {
            request: param.clone(),
        }
    }

    fn connector(&self) -> Result<deck_connector::Connector> {
        get_connector(&self.datasource_key(), "mysql")
            .ok_or_else(|| error!("MySQL连接未初始化"))
    }

    fn price_per_token(model: &str) -> f64 {
        let m = model.to_lowercase();
        if m.contains("gpt-4o") || m.contains("gpt4o") {
            0.000005
        } else if m.contains("gpt-4") || m.contains("gpt4") {
            0.00003
        } else if m.contains("o1") || m.contains("o3") {
            0.000015
        } else if m.contains("claude-3.5") || m.contains("claude-3-5") || m.contains("claude3.5") {
            0.000003
        } else if m.contains("claude-3") || m.contains("claude3") {
            0.000015
        } else if m.contains("deepseek") {
            0.00000014
        } else if m.contains("qwen") || m.contains("通义") {
            0.0000004
        } else if m.contains("glm") || m.contains("chatglm") {
            0.000001
        } else if m.contains("gpt-3.5") || m.contains("gpt3.5") {
            0.0000015
        } else {
            0.000002
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "summary" => self.summary().await,
            "dashboard" => self.dashboard().await,
            "records" => self.records().await,
            "export" => self.export().await,
            _ => Err(error!("usage不支持方法: {}", method)),
        }
    }

    async fn summary(&self) -> Result<Value> {
        let params = self.value();
        let period = utils::param_string(&params, "period");
        let period_clause = match period.as_str() {
            "day" => "DATE(create_time) = CURDATE()",
            "week" => "YEARWEEK(create_time) = YEARWEEK(NOW())",
            "month" => "MONTH(create_time) = MONTH(NOW()) AND YEAR(create_time) = YEAR(NOW())",
            _ => "1=1",
        };

        let sql = format!(
            "SELECT llm_provider, llm_model, operation_type, \
             COUNT(*) as call_count, \
             SUM(prompt_tokens) as total_prompt_tokens, \
             SUM(completion_tokens) as total_completion_tokens, \
             SUM(total_tokens) as total_tokens, \
             AVG(latency_ms) as avg_latency_ms \
             FROM llm_usage WHERE {} GROUP BY llm_provider, llm_model, operation_type",
            period_clause
        );

        let connector = self.connector()?;
        let results = db::query_rows(&sql, vec![], &connector)?;

        let total_calls: i64 = results.iter().map(|r| r.get("callCount").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64).sum();
        let total_tokens: i64 = results.iter().map(|r| r.get("totalTokens").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64).sum();
        let total_cost_estimate: f64 = results.iter().map(|r| {
            let tokens = r.get("totalTokens").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let model = r.get("llmModel").and_then(|v| v.as_str()).unwrap_or_default();
            tokens * Self::price_per_token(&model)
        }).sum();

        Ok(value!({
            "period": period,
            "totalCalls": total_calls,
            "totalTokens": total_tokens,
            "totalCostEstimate": total_cost_estimate,
            "breakdown": results,
        }))
    }

    async fn dashboard(&self) -> Result<Value> {
        let connector = self.connector()?;

        let total_sql = "SELECT COUNT(*) as total_calls, SUM(total_tokens) as total_tokens, \
             AVG(latency_ms) as avg_latency \
             FROM llm_usage";
        let total_rows = db::query_rows(total_sql, vec![], &connector)?;

        let total_calls: i64 = db::first_row_i64(&total_rows, "totalCalls");
        let total_tokens: i64 = db::first_row_i64(&total_rows, "totalTokens");

        let total_cost_estimate: f64 = {
            let provider_sql = "SELECT llm_model, SUM(total_tokens) as total_tokens FROM llm_usage GROUP BY llm_model";
            let provider_rows = db::query_rows(provider_sql, vec![], &connector).unwrap_or_default();
            provider_rows.iter().map(|r| {
                let tokens = db::row_get_f64(&r, "totalTokens");
                let model = db::row_get_string(&r, "llmModel");
                tokens * Self::price_per_token(&model)
            }).sum()
        };

        let provider_sql = "SELECT llm_provider, COUNT(*) as call_count, SUM(total_tokens) as total_tokens \
             FROM llm_usage GROUP BY llm_provider";
        let provider_rows = db::query_rows(provider_sql, vec![], &connector)?;
        let calls_by_provider: Vec<Value> = provider_rows;

        let type_sql = "SELECT operation_type, COUNT(*) as call_count, SUM(total_tokens) as total_tokens \
             FROM llm_usage GROUP BY operation_type";
        let type_rows = db::query_rows(type_sql, vec![], &connector)?;
        let calls_by_type: Vec<Value> = type_rows;

        let daily_sql = "SELECT DATE(create_time) as date, COUNT(*) as call_count, SUM(total_tokens) as total_tokens \
             FROM llm_usage WHERE create_time >= DATE_SUB(NOW(), INTERVAL 30 DAY) \
             GROUP BY DATE(create_time) ORDER BY date";
        let daily_rows = db::query_rows(daily_sql, vec![], &connector)?;
        let daily_calls: Vec<Value> = daily_rows;

        let summary_sql = "SELECT COUNT(*) as calls, SUM(total_tokens) as tokens, \
             AVG(latency_ms) as avg_latency \
             FROM llm_usage WHERE DATE(create_time) = CURDATE()";
        let summary_rows = db::query_rows(summary_sql, vec![], &connector)?;

        let today_calls: i64 = db::first_row_i64(&summary_rows, "calls");
        let today_tokens: i64 = db::first_row_i64(&summary_rows, "tokens");
        let avg_latency: f64 = summary_rows
            .first()
            .map(|r| r.get("avgLatency").and_then(|v| v.as_f64()).unwrap_or(0.0))
            .unwrap_or(0.0);

        let recent_sql = "SELECT id, llm_provider, llm_model, operation_type, \
             prompt_tokens, completion_tokens, total_tokens, cache_hit, latency_ms, \
             stock_code, create_time \
             FROM llm_usage ORDER BY create_time DESC LIMIT 10";
        let recent_rows = db::query_rows(recent_sql, vec![], &connector)?;
        let recent: Vec<Value> = recent_rows;

        Ok(value!({
            "totalCalls": total_calls,
            "totalTokens": total_tokens,
            "totalCostEstimate": total_cost_estimate,
            "callsByProvider": calls_by_provider,
            "callsByType": calls_by_type,
            "dailyCalls": daily_calls,
            "todayCalls": today_calls,
            "todayTokens": today_tokens,
            "avgLatencyMs": avg_latency,
            "recentRecords": recent,
        }))
    }

    async fn records(&self) -> Result<Value> {
        let params = self.value();
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;
        let provider = utils::param_string(&params, "provider");

        let mut conds = vec![];
        if !provider.is_empty() {
            conds.push(cond!("llm_provider" = provider));
        }

        let results = self.select()
            .columns(cols![
                "id", "llm_provider", "llm_model", "operation_type",
                "prompt_tokens", "completion_tokens", "total_tokens", "cache_hit",
                "latency_ms", "stock_code", "create_time"
            ])
            .r#where(conds)
            .order(ord!("create_time DESC"))
            .limit(limit as u64)
            .query_values()?;

        Ok(Value::Array(results))
    }

    async fn export(&self) -> Result<Value> {
        let params = self.value();
        let start_date = utils::param_string(&params, "start_date");
        let end_date = utils::param_string(&params, "end_date");

        let connector = self.connector()?;

        let (sql, p) = if !start_date.is_empty() && !end_date.is_empty() {
            ("SELECT id, llm_provider, llm_model, operation_type, \
              prompt_tokens, completion_tokens, total_tokens, cache_hit, latency_ms, \
              stock_code, create_time \
              FROM llm_usage WHERE DATE(create_time) >= :start_date AND DATE(create_time) <= :end_date \
              ORDER BY create_time DESC".to_string(),
             vec![
                 ("start_date".to_string(), Value::from(start_date.as_str())),
                 ("end_date".to_string(), Value::from(end_date.as_str())),
             ])
        } else if !start_date.is_empty() {
            ("SELECT id, llm_provider, llm_model, operation_type, \
              prompt_tokens, completion_tokens, total_tokens, cache_hit, latency_ms, \
              stock_code, create_time \
              FROM llm_usage WHERE DATE(create_time) >= :start_date \
              ORDER BY create_time DESC".to_string(),
             vec![("start_date".to_string(), Value::from(start_date.as_str()))])
        } else {
            ("SELECT id, llm_provider, llm_model, operation_type, \
              prompt_tokens, completion_tokens, total_tokens, cache_hit, latency_ms, \
              stock_code, create_time \
              FROM llm_usage ORDER BY create_time DESC".to_string(),
             vec![])
        };

        let rows = db::query_rows(&sql, p, &connector)?;
        let records: Vec<Value> = rows;

        let total_calls = records.len() as i64;
        let total_tokens: i64 = records.iter()
            .filter_map(|r| r.get("totalTokens").and_then(|v| v.as_f64()))
            .sum::<f64>() as i64;
        let total_cost_estimate: f64 = records.iter()
            .filter_map(|r| {
                let tokens = r.get("totalTokens").and_then(|v| v.as_f64())?;
                let model = r.get("llmModel").and_then(|v| v.as_str()).unwrap_or_default();
                Some(tokens * Self::price_per_token(&model))
            })
            .sum();

        Ok(value!({
            "records": records,
            "summary": {
                "totalCalls": total_calls,
                "totalTokens": total_tokens,
                "totalCostEstimate": total_cost_estimate,
                "startDate": start_date,
                "endDate": end_date,
            },
        }))
    }
}
