//! 筹码工具 - 筹码分布分析

use dsa_core::utils;
use deck::DataRow;
use deck_mysql::Helper;
use tube::Value;

use super::registry::{ToolParameter, ToolRegistry};

pub struct ChipTools;

impl ChipTools {
    pub fn new() -> Self { Self }

    /// 获取筹码分布 (simplified: compute from kline volume distribution)
    pub fn get_chip_distribution(code: &str) -> Value {
        let connector = match utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => {
                let mut m = tube::Map::new();
                m.insert("code".to_string(), Value::from(code.to_string()));
                m.insert("concentration".to_string(), Value::Null);
                m.insert("dataAvailable".to_string(), Value::from(false));
                m.insert("message".to_string(), Value::from("DB连接失败"));
                return Value::Object(m);
            }
        };
        let sql = format!("SELECT close, volume FROM stock_daily WHERE stock_code = '{}' AND status >= 1 ORDER BY trade_date DESC LIMIT 20", code);
        match Helper::query_rows(&sql, vec![], &connector) {
            Ok(rows) => {
                if rows.is_empty() {
                    let mut m = tube::Map::new();
                    m.insert("code".to_string(), Value::from(code.to_string()));
                    m.insert("concentration".to_string(), Value::Null);
                    m.insert("dataAvailable".to_string(), Value::from(false));
                    m.insert("message".to_string(), Value::from("无K线数据"));
                    return Value::Object(m);
                }
                let total_vol: f64 = rows.iter().map(|r| r.get_value(1).as_f64().unwrap_or(0.0)).sum();
                if total_vol <= 0.0 {
                    let mut m = tube::Map::new();
                    m.insert("code".to_string(), Value::from(code.to_string()));
                    m.insert("concentration".to_string(), Value::Null);
                    m.insert("dataAvailable".to_string(), Value::from(false));
                    m.insert("message".to_string(), Value::from("成交量数据为零"));
                    return Value::Object(m);
                }
                let weighted_price: f64 = rows.iter()
                    .map(|r| r.get_value(0).as_f64().unwrap_or(0.0) * r.get_value(1).as_f64().unwrap_or(0.0))
                    .sum::<f64>() / total_vol;
                let mut vols: Vec<f64> = rows.iter().map(|r| r.get_value(1).as_f64().unwrap_or(0.0)).collect();
                vols.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                let top3_vol: f64 = vols.iter().take(3).sum();
                let concentration = (top3_vol / total_vol * 100.0).min(100.0);
                value!({
                    "code": code,
                    "concentration": concentration,
                    "weightedPrice": weighted_price,
                    "totalVolume": total_vol,
                    "dataAvailable": true,
                })
            }
            Err(_) => {
                let mut m = tube::Map::new();
                m.insert("code".to_string(), Value::from(code.to_string()));
                m.insert("concentration".to_string(), Value::Null);
                m.insert("dataAvailable".to_string(), Value::from(false));
                m.insert("message".to_string(), Value::from("查询失败"));
                Value::Object(m)
            }
        }
    }
}

/// 异步封装: 获取筹码集中度
async fn concentration(params: &Value) -> dsa_core::DsaResult<Value> {
    let code = params
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    Ok(ChipTools::get_chip_distribution(&code))
}

/// 注册筹码工具到 ToolRegistry
pub fn register(registry: &mut ToolRegistry) {
    registry.register(
        "get_chip_concentration",
        "Analyze chip concentration for a stock",
        vec![ToolParameter {
            name: "code".into(),
            param_type: "string".into(),
            description: "Stock code".into(),
            required: true,
            default_value: None,
        }],
        |params| Box::pin(async move { concentration(&params).await }),
    );
}
