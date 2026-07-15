//! 组合工具 - 组合快照与资金流向

use deck_mysql::{DataRow, Helper};
use dsa_core::{utils, DsaError, DsaResult};
use tube::Value;

use super::registry::{ToolParameter, ToolRegistry};

pub struct PortfolioTools;

impl PortfolioTools {
    pub fn new() -> Self {
        Self
    }

    /// 获取组合快照
    pub fn get_portfolio_snapshot(_code: &str) -> Value {
        let connector = match utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => {
                let mut m = tube::Map::new();
                m.insert("totalAssets".to_string(), Value::Null);
                m.insert("positions".to_string(), Value::Array(vec![]));
                m.insert("dataAvailable".to_string(), Value::from(false));
                m.insert("message".to_string(), Value::from("数据库连接失败"));
                return Value::Object(m);
            }
        };
        let sql = "SELECT p.stock_code, p.stock_name, p.quantity, p.avg_cost, p.current_price, \
             p.market_value, p.unrealized_pnl, p.unrealized_pnl_pct, a.initial_capital \
             FROM portfolio_positions p \
             JOIN portfolio_accounts a ON p.account_id = a.id \
             WHERE p.status = 1 AND a.status >= 1";
        match Helper::query_rows(sql, vec![], &connector) {
            Ok(rows) => {
                let positions: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
                let total_mv: f64 = positions
                    .iter()
                    .filter_map(|p| p.get("market_value").and_then(|v| v.as_f64()))
                    .sum();
                let initial_opt: Option<f64> = rows
                    .first()
                    .map(|r| r.get_value(8).as_f64().unwrap_or(0.0))
                    .filter(|v| *v > 0.0);
                match initial_opt {
                    Some(initial) => {
                        let cash = initial
                            - positions
                                .iter()
                                .filter_map(|p| p.get("avg_cost").and_then(|v| v.as_f64()))
                                .zip(
                                    positions
                                        .iter()
                                        .filter_map(|p| p.get("quantity").and_then(|v| v.as_f64())),
                                )
                                .map(|(cost, qty)| cost * qty)
                                .sum::<f64>();
                        value!({
                            "totalAssets": total_mv + cash.max(0.0),
                            "cash": cash.max(0.0),
                            "marketValue": total_mv,
                            "positionCount": positions.len() as i64,
                            "positions": positions,
                            "dataAvailable": true,
                        })
                    }
                    None => {
                        let mut m = tube::Map::new();
                        m.insert(
                            "totalAssets".to_string(),
                            if total_mv > 0.0 {
                                Value::from(total_mv)
                            } else {
                                Value::Null
                            },
                        );
                        m.insert("marketValue".to_string(), Value::from(total_mv));
                        m.insert(
                            "positionCount".to_string(),
                            Value::from(positions.len() as i64),
                        );
                        m.insert("positions".to_string(), Value::Array(positions));
                        m.insert("dataAvailable".to_string(), Value::from(total_mv > 0.0));
                        m.insert(
                            "message".to_string(),
                            Value::from(if total_mv <= 0.0 {
                                "无持仓数据"
                            } else {
                                "无账户初始资金数据，仅显示持仓市值"
                            }),
                        );
                        Value::Object(m)
                    }
                }
            }
            Err(_) => {
                let mut m = tube::Map::new();
                m.insert("totalAssets".to_string(), Value::Null);
                m.insert("positions".to_string(), Value::Array(vec![]));
                m.insert("dataAvailable".to_string(), Value::from(false));
                m.insert("message".to_string(), Value::from("查询失败"));
                Value::Object(m)
            }
        }
    }

    /// 获取资金流向 (capital flow from eastmoney)
    pub async fn get_capital_flow(code: &str) -> DsaResult<Value> {
        let pure_code = code
            .trim_start_matches("sh")
            .trim_start_matches("sz")
            .trim_start_matches("bj")
            .trim_start_matches("SH")
            .trim_start_matches("SZ")
            .trim_start_matches("BJ");
        let prefix = utils::market_prefix(pure_code);
        let symbol = format!("{}{}", prefix, pure_code);

        let qq = qta_crawler::QQ::new();
        match qq.get_capital_flow(&symbol).await {
            Ok(data) => {
                let parse_f64 = |key: &str| -> f64 {
                    data.get(key)
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0)
                };
                Ok(value!({
                    "code": code,
                    "mainNetInflow": parse_f64("main_net_inflow"),
                    "mainInflow": parse_f64("main_inflow"),
                    "mainOutflow": parse_f64("main_outflow"),
                    "mainNetInflowRatio": parse_f64("main_net_inflow_ratio"),
                    "retailNetInflow": parse_f64("retail_net_inflow"),
                    "retailInflow": parse_f64("retail_inflow"),
                    "retailOutflow": parse_f64("retail_outflow"),
                    "retailNetInflowRatio": parse_f64("retail_net_inflow_ratio"),
                    "name": data.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    "date": data.get("date").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                }))
            }
            Err(e) => Err(DsaError::StockData(format!("获取资金流向失败: {}", e))),
        }
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
