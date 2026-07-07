//! 历史工具 - 分析上下文从历史记录

use dsa_core::utils;
use deck_mysql::{DataRow, Helper};
use tube::Value;

use super::registry::{ToolParameter, ToolRegistry};

pub struct HistoryTools;

impl HistoryTools {
    pub fn new() -> Self { Self }

    /// 获取历史分析上下文
    pub fn get_analysis_context(code: &str, limit: i64) -> Value {
        let connector = match utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => return value!({"data": []}),
        };
        let sql = format!("SELECT id, stockCode, stockName, sentimentScore, operationAdvice, \
             trendPrediction, idealBuy, secondaryBuy, stopLoss, takeProfit, \
             analysisSummary, createTime \
             FROM analysis_history WHERE stockCode = '{}' AND status >= 1 \
             ORDER BY createTime DESC LIMIT {}", code, limit);
        match Helper::query_rows(&sql, vec![], &connector) {
            Ok(rows) => {
                let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
                value!({"data": results})
            }
            Err(_) => value!({"data": []}),
        }
    }
}

/// 异步封装: 获取历史分析上下文
async fn analysis_context(params: &Value) -> dsa_core::DsaResult<Value> {
    let code = params
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let limit = params
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(10);
    Ok(HistoryTools::get_analysis_context(&code, limit))
}

/// 注册历史工具到 ToolRegistry
pub fn register(registry: &mut ToolRegistry) {
    registry.register(
        "get_analysis_context",
        "Get historical analysis context for a stock",
        vec![
            ToolParameter {
                name: "code".into(),
                param_type: "string".into(),
                description: "Stock code".into(),
                required: true,
                default_value: None,
            },
            ToolParameter {
                name: "limit".into(),
                param_type: "integer".into(),
                description: "Number of records to return".into(),
                required: false,
                default_value: Some("10".into()),
            },
        ],
        |params| Box::pin(async move { analysis_context(&params).await }),
    );
}
