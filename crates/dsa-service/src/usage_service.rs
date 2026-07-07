//! LLM使用量追踪服务

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use tube::Value;

/// LLM使用量追踪服务
pub struct UsageService {}

impl UsageService {
    /// 创建LLM使用量追踪服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: summary, dashboard, records, export
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "summary" => self.summary(params).await,
            "dashboard" => self.dashboard(params).await,
            "records" => self.records(params).await,
            "export" => self.export(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "usage不支持方法: {}",
                method
            ))),
        }
    }

    /// 聚合使用统计
    async fn summary(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let period = utils::param_string(params, "period");
        let period_clause = match period.as_str() {
            "day" => "DATE(createTime) = CURDATE()",
            "week" => "YEARWEEK(createTime) = YEARWEEK(NOW())",
            "month" => "MONTH(create_time) = MONTH(NOW()) AND YEAR(create_time) = YEAR(NOW())",
            _ => "1=1",
        };

        let sql = format!(
            "SELECT llm_provider, llm_model, operation_type, \
             COUNT(*) as callCount, \
             SUM(promptTokens) as totalPromptTokens, \
             SUM(completionTokens) as totalCompletionTokens, \
             SUM(totalTokens) as totalTokens, \
             AVG(latencyMs) as avgLatencyMs \
             FROM llm_usage WHERE {} GROUP BY llm_provider, llm_model, operation_type",
            period_clause
        );

        let rows = Helper::query_rows(&sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询使用统计失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();

        // 汇总
        let total_calls: i64 = results.iter().map(|r| r.get("callCount").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64).sum();
        let total_tokens: i64 = results.iter().map(|r| r.get("totalTokens").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64).sum();

        Ok(value!({
            "period": period,
            "totalCalls": total_calls,
            "totalTokens": total_tokens,
            "breakdown": results,
        }))
    }

    /// 仪表盘 (汇总+最近记录+聚合统计)
    async fn dashboard(&self, _params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;

        // 总计汇总
        let total_sql = "SELECT COUNT(*) as totalCalls, SUM(total_tokens) as total_tokens, \
             AVG(latencyMs) as avgLatency \
             FROM llm_usage";
        let total_rows = Helper::query_rows(total_sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询总计统计失败: {}", e)))?;

        let total_calls: i64 = total_rows
            .first()
            .map(|r| r.get_value(0).as_f64().unwrap_or(0.0) as i64)
            .unwrap_or(0);
        let total_tokens: i64 = total_rows
            .first()
            .map(|r| r.get_value(1).as_f64().unwrap_or(0.0) as i64)
            .unwrap_or(0);

        // 粗略成本估算: 每千token约0.002元
        let total_cost_estimate = (total_tokens as f64) * 0.002 / 1000.0;

        // 按Provider聚合
        let provider_sql = "SELECT llm_provider, COUNT(*) as callCount, SUM(total_tokens) as total_tokens \
             FROM llm_usage GROUP BY llm_provider";
        let provider_rows = Helper::query_rows(provider_sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询Provider统计失败: {}", e)))?;
        let calls_by_provider: Vec<Value> = provider_rows.iter().map(|r| r.to_value2()).collect();

        // 按操作类型聚合
        let type_sql = "SELECT operation_type, COUNT(*) as callCount, SUM(total_tokens) as total_tokens \
             FROM llm_usage GROUP BY operation_type";
        let type_rows = Helper::query_rows(type_sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询操作类型统计失败: {}", e)))?;
        let calls_by_type: Vec<Value> = type_rows.iter().map(|r| r.to_value2()).collect();

        // 最近30天每日调用
        let daily_sql = "SELECT DATE(create_time) as date, COUNT(*) as callCount, SUM(total_tokens) as total_tokens \
             FROM llm_usage WHERE create_time >= DATE_SUB(NOW(), INTERVAL 30 DAY) \
             GROUP BY DATE(create_time) ORDER BY date";
        let daily_rows = Helper::query_rows(daily_sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询每日统计失败: {}", e)))?;
        let daily_calls: Vec<Value> = daily_rows.iter().map(|r| r.to_value2()).collect();

        // 今日汇总
        let summary_sql = "SELECT COUNT(*) as calls, SUM(total_tokens) as tokens, \
             AVG(latencyMs) as avgLatency \
             FROM llm_usage WHERE DATE(create_time) = CURDATE()";
        let summary_rows = Helper::query_rows(summary_sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询今日统计失败: {}", e)))?;

        let today_calls: i64 = summary_rows
            .first()
            .map(|r| r.get_value(0).as_f64().unwrap_or(0.0) as i64)
            .unwrap_or(0);
        let today_tokens: i64 = summary_rows
            .first()
            .map(|r| r.get_value(1).as_f64().unwrap_or(0.0) as i64)
            .unwrap_or(0);
        let avg_latency: f64 = summary_rows
            .first()
            .map(|r| r.get_value(2).as_f64().unwrap_or(0.0))
            .unwrap_or(0.0);

        // 最近记录
        let recent_sql = "SELECT id, llm_provider, llm_model, operation_type, \
             promptTokens, completionTokens, totalTokens, cacheHit, latencyMs, \
             stockCode, createTime \
             FROM llm_usage ORDER BY create_time DESC LIMIT 10";
        let recent_rows = Helper::query_rows(recent_sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询最近记录失败: {}", e)))?;

        let recent: Vec<Value> = recent_rows.iter().map(|r| r.to_value2()).collect();

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

    /// 导出LLM使用量数据 - 支持日期范围过滤
    async fn export(&self, params: &Value) -> DsaResult<Value> {
        let start_date = utils::param_string(params, "start_date");
        let end_date = utils::param_string(params, "end_date");
        let _format = utils::param_string(params, "format");
        // format参数预留扩展, 当前仅支持json

        let connector = utils::get_db_connector()?;

        let (sql, p) = if !start_date.is_empty() && !end_date.is_empty() {
            ("SELECT id, llm_provider, llm_model, operation_type, \
              promptTokens, completionTokens, totalTokens, cacheHit, latencyMs, \
              stockCode, createTime \
              FROM llm_usage WHERE DATE(create_time) >= :start_date AND DATE(create_time) <= :end_date \
              ORDER BY create_time DESC".to_string(),
             vec![
                 ("start_date".to_string(), Value::from(start_date.as_str())),
                 ("end_date".to_string(), Value::from(end_date.as_str())),
             ])
        } else if !start_date.is_empty() {
            ("SELECT id, llm_provider, llm_model, operation_type, \
              promptTokens, completionTokens, totalTokens, cacheHit, latencyMs, \
              stockCode, createTime \
              FROM llm_usage WHERE DATE(create_time) >= :start_date \
              ORDER BY create_time DESC".to_string(),
             vec![("start_date".to_string(), Value::from(start_date.as_str()))])
        } else {
            ("SELECT id, llm_provider, llm_model, operation_type, \
              promptTokens, completionTokens, totalTokens, cacheHit, latencyMs, \
              stockCode, createTime \
              FROM llm_usage ORDER BY create_time DESC".to_string(),
             vec![])
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("导出使用量数据失败: {}", e)))?;

        let records: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();

        // 聚合统计
        let total_calls = records.len() as i64;
        let total_tokens: i64 = records.iter()
            .filter_map(|r| r.get("totalTokens").and_then(|v| v.as_f64()))
            .sum::<f64>() as i64;
        let total_cost_estimate: f64 = records.iter()
            .filter_map(|r| {
                let tokens = r.get("totalTokens").and_then(|v| v.as_f64())?;
                // 粗略估算: 每千token约0.002元
                Some(tokens * 0.002 / 1000.0)
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

    /// 详细使用记录
    async fn records(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;
        let provider = utils::param_string(params, "provider");

        let (sql, p) = if provider.is_empty() {
            (
                "SELECT id, llm_provider, llm_model, operation_type, \
                 promptTokens, completionTokens, totalTokens, cacheHit, latencyMs, \
                 stockCode, createTime \
                 FROM llm_usage ORDER BY create_time DESC LIMIT :limit"
                    .to_string(),
                vec![("limit".to_string(), Value::from(limit))],
            )
        } else {
            (
                "SELECT id, llm_provider, llm_model, operation_type, \
                 promptTokens, completionTokens, totalTokens, cacheHit, latencyMs, \
                 stockCode, createTime \
                 FROM llm_usage WHERE llm_provider = :provider ORDER BY create_time DESC LIMIT :limit"
                    .to_string(),
                vec![
                    ("provider".to_string(), Value::from(provider.as_str())),
                    ("limit".to_string(), Value::from(limit)),
                ],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询使用记录失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(Value::Array(results))
    }
}
