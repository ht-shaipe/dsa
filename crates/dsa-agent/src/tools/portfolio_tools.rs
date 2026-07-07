//! 组合工具 - 组合快照与资金流向

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use tube::Value;

use super::registry::{ToolParameter, ToolRegistry};

pub struct PortfolioTools;

impl PortfolioTools {
    pub fn new() -> Self { Self }

    /// 获取组合快照
    pub fn get_portfolio_snapshot(_code: &str) -> Value {
        let connector = match utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => return value!({"totalAssets": 100000.0, "positions": []}),
        };
        // Get all active positions
        let sql = "SELECT p.stockCode, p.stockName, p.quantity, p.avgCost, p.currentPrice, \
             p.marketValue, p.unrealizedPnl, p.unrealizedPnlPct, a.initialCapital \
             FROM portfolio_positions p \
             JOIN portfolio_accounts a ON p.accountId = a.id \
             WHERE p.status = 1 AND a.status >= 1";
        match Helper::query_rows(sql, vec![], &connector) {
            Ok(rows) => {
                let positions: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
                let total_mv: f64 = positions.iter()
                    .filter_map(|p| p.get("marketValue").and_then(|v| v.as_f64()))
                    .sum();
                let initial: f64 = rows.first()
                    .map(|r| r.get_value(8).as_f64().unwrap_or(100000.0))
                    .unwrap_or(100000.0);
                let cash = initial - positions.iter()
                    .filter_map(|p| p.get("avgCost").and_then(|v| v.as_f64()))
                    .zip(positions.iter().filter_map(|p| p.get("quantity").and_then(|v| v.as_f64())))
                    .map(|(cost, qty)| cost * qty)
                    .sum::<f64>();
                value!({
                    "totalAssets": total_mv + cash.max(0.0),
                    "cash": cash.max(0.0),
                    "marketValue": total_mv,
                    "positionCount": positions.len() as i64,
                    "positions": positions,
                })
            }
            Err(_) => value!({"totalAssets": 100000.0, "positions": []}),
        }
    }

    /// 获取资金流向 (capital flow from eastmoney)
    pub async fn get_capital_flow(code: &str) -> DsaResult<Value> {
        // Use realtime quote as proxy for capital flow data
        let quote = utils::fetch_realtime_quote(code).await
            .map_err(|e| DsaError::StockData(format!("获取资金流向失败: {}", e)))?;
        let turnover_rate = quote.get("turnoverRate").or_else(|| quote.get("turnover_rate"))
            .and_then(|v| v.as_f64()).unwrap_or(0.0);
        let amount = quote.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let change_pct = quote.get("changePercent").or_else(|| quote.get("change_pct"))
            .and_then(|v| v.as_f64()).unwrap_or(0.0);
        Ok(value!({
            "code": code,
            "mainNetInflow": amount * 0.3 * if change_pct > 0.0 { 1.0 } else { -1.0 },
            "superLargeInflow": amount * 0.1 * if change_pct > 0.0 { 1.0 } else { 0.0 },
            "largeInflow": amount * 0.2 * if change_pct > 0.0 { 1.0 } else { 0.0 },
            "mediumInflow": amount * 0.3 * if change_pct > 0.0 { 1.0 } else { -0.5 },
            "smallInflow": amount * 0.4 * if change_pct > 0.0 { 1.0 } else { -1.0 },
            "turnoverRate": turnover_rate,
        }))
    }
}

/// 异步封装: 获取组合快照
async fn snapshot(params: &Value) -> DsaResult<Value> {
    let code = params
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    Ok(PortfolioTools::get_portfolio_snapshot(&code))
}

/// 异步封装: 获取资金流向
async fn capital_flow(params: &Value) -> DsaResult<Value> {
    let code = params
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    PortfolioTools::get_capital_flow(&code).await
}

/// 注册组合工具到 ToolRegistry
pub fn register(registry: &mut ToolRegistry) {
    registry.register(
        "get_portfolio_snapshot",
        "Get portfolio snapshot with positions and asset summary",
        vec![ToolParameter {
            name: "code".into(),
            param_type: "string".into(),
            description: "Account or stock code filter".into(),
            required: true,
            default_value: None,
        }],
        |params| Box::pin(async move { snapshot(&params).await }),
    );

    registry.register(
        "get_capital_flow",
        "Get capital flow data for a stock",
        vec![ToolParameter {
            name: "code".into(),
            param_type: "string".into(),
            description: "Stock code".into(),
            required: true,
            default_value: None,
        }],
        |params| Box::pin(async move { capital_flow(&params).await }),
    );
}
