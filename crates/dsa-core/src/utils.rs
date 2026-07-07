//! 共享工具函数 - 参数提取、市场前缀、数据库连接、行情数据获取

use crate::models::KlineBar;
use crate::DsaError;
use deck_connector::{get_connector, Connector};
use qta_crawler::{EastMoney, QQ, Real};
use tube::Value;

/// 从参数中提取字符串值，不存在则返回空字符串
pub fn param_string(params: &Value, key: &str) -> String {
    params.get(key).and_then(|v| v.as_str()).unwrap_or_default()
}

/// 从参数中提取字符串值，不存在则返回默认值
pub fn param_string_default(params: &Value, key: &str, default: &str) -> String {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default.to_string())
}

/// 从参数中提取整数值，不存在则返回0
pub fn param_i64(params: &Value, key: &str) -> i64 {
    params.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0) as i64
}

/// 从参数中提取浮点数值，不存在则返回0.0
pub fn param_f64(params: &Value, key: &str) -> f64 {
    params.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

/// 根据股票代码返回市场前缀（sh/sz/bj）
pub fn market_prefix(code: &str) -> &'static str {
    if code.starts_with('6') || code.starts_with('9') {
        "sh"
    } else if code.starts_with('0') || code.starts_with('3') {
        "sz"
    } else if code.starts_with('8') || code.starts_with('4') {
        "bj"
    } else {
        "sh"
    }
}

/// 获取默认MySQL数据库连接器
pub fn get_db_connector() -> Result<Connector, DsaError> {
    get_connector("default", "mysql")
        .ok_or_else(|| DsaError::Database("MySQL连接未初始化".to_string()))
}

/// 从东方财富获取K线数据并写入数据库
pub async fn fetch_kline(code: &str, period: &str) -> Result<Vec<KlineBar>, DsaError> {
    let em = EastMoney::new();
    let raw = em
        .stock_zh_a_hist(code, Some(period), None, None, Some("qfq"))
        .await
        .map_err(|e| DsaError::StockData(format!("获取K线数据失败: {}", e)))?;

    let mut bars = Vec::with_capacity(raw.len());
    for item in &raw {
        bars.push(KlineBar {
            date: item.get("日期").and_then(|v| v.as_str()).unwrap_or_default(),
            open: item.get("开盘").and_then(|v| v.as_f64()).unwrap_or(0.0),
            high: item.get("最高").and_then(|v| v.as_f64()).unwrap_or(0.0),
            low: item.get("最低").and_then(|v| v.as_f64()).unwrap_or(0.0),
            close: item.get("收盘").and_then(|v| v.as_f64()).unwrap_or(0.0),
            volume: item.get("成交量").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
            amount: item.get("成交额").and_then(|v| v.as_f64()).unwrap_or(0.0),
        });
    }

    save_kline_to_db(code, &bars);

    Ok(bars)
}

fn save_kline_to_db(code: &str, bars: &[KlineBar]) {
    let connector = match get_db_connector() {
        Ok(c) => c,
        Err(_) => return,
    };
    for bar in bars.iter().rev().take(5) {
        let sql = "INSERT INTO stock_daily \
             (stockCode, stockName, tradeDate, open, high, low, close, volume, amount, status, createTime) \
             VALUES (:code, '', :date, :open, :high, :low, :close, :vol, :amt, 1, NOW()) \
             ON DUPLICATE KEY UPDATE \
             open=VALUES(open), high=VALUES(high), low=VALUES(low), \
             close=VALUES(close), volume=VALUES(volume), amount=VALUES(amount)";
        let _ = deck::Helper::execute(
            sql,
            vec![
                ("code".to_string(), Value::from(code.to_string())),
                ("date".to_string(), Value::from(bar.date.as_str())),
                ("open".to_string(), Value::from(bar.open)),
                ("high".to_string(), Value::from(bar.high)),
                ("low".to_string(), Value::from(bar.low)),
                ("close".to_string(), Value::from(bar.close)),
                ("vol".to_string(), Value::from(bar.volume)),
                ("amt".to_string(), Value::from(bar.amount)),
            ],
            &connector,
        );
    }
}

/// 获取实时行情数据，按配置的数据源优先级依次尝试
pub async fn fetch_realtime_quote(code: &str) -> Result<Value, DsaError> {
    let prefix = market_prefix(code);
    let symbol = format!("{}{}", prefix, code);

    let conf = crate::get_global_config();
    if conf.stock.realtime_source_priority.is_empty() {
        return QQ::new()
            .get_realtime_quote(&symbol)
            .await
            .map_err(|e| DsaError::StockData(format!("获取实时行情失败: {}", e)));
    }

    for source in &conf.stock.realtime_source_priority {
        let result = match source.as_str() {
            "tencent" | "qq" => QQ::new().get_realtime_quote(&symbol).await,
            "sina" | "real" => Real::new().get_price(&symbol).await,
            "eastmoney" => EastMoney::new().stock_zh_a_spot().await
                .map(|spot: Vec<Value>| {
                    spot.iter()
                        .find(|item| {
                            let item_code: String = item.get("代码").or_else(|| item.get("code"))
                                .and_then(|v| v.as_str()).unwrap_or_default();
                            item_code == code
                        })
                        .cloned()
                        .unwrap_or_else(|| tube::value!({}))
                }),
            _ => continue,
        };
        if let Ok(val) = result {
            let has_price = val.get("price").is_some()
                || val.get("最新价").is_some()
                || val.get("current_price").is_some();
            if has_price {
                return Ok(val);
            }
        }
    }

    QQ::new()
        .get_realtime_quote(&symbol)
        .await
        .map_err(|e| DsaError::StockData(format!("获取实时行情失败: {}", e)))
}

/// 从新浪实时接口获取价格数据
pub async fn fetch_realtime_price(symbol: &str) -> Result<Value, DsaError> {
    Real::new()
        .get_price(symbol)
        .await
        .map_err(|e| DsaError::StockData(format!("获取实时价格失败: {}", e)))
}

/// 获取大盘环境摘要（上证指数涨跌幅）
pub async fn fetch_market_context() -> Option<String> {
    match Real::new().get_price("sh000001").await {
        Ok(v) => {
            let name = v.get("name").and_then(|v| v.as_str()).unwrap_or_else(|| "上证指数".to_string());
            let chg = v.get("changePercent").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Some(format!("大盘: {} 涨跌{:.2}%", name, chg))
        }
        Err(_) => None,
    }
}

/// 记录LLM调用量到数据库
pub fn record_llm_usage(
    provider: &str,
    model: &str,
    operation_type: &str,
    prompt_tokens: i32,
    completion_tokens: i32,
    latency_ms: i64,
    stock_code: &str,
) {
    let connector = match get_db_connector() {
        Ok(c) => c,
        Err(_) => return,
    };
    let sql = "INSERT INTO llm_usage \
         (llmProvider, llmModel, operationType, promptTokens, completionTokens, totalTokens, \
          cacheHit, latencyMs, stockCode, createTime) \
         VALUES (:provider, :model, :op, :pt, :ct, :tt, 0, :latency, :code, NOW())";
    let total = prompt_tokens + completion_tokens;
    let _ = deck::Helper::execute(
        sql,
        vec![
            ("provider".to_string(), Value::from(provider.to_string())),
            ("model".to_string(), Value::from(model.to_string())),
            ("op".to_string(), Value::from(operation_type.to_string())),
            ("pt".to_string(), Value::from(prompt_tokens)),
            ("ct".to_string(), Value::from(completion_tokens)),
            ("tt".to_string(), Value::from(total)),
            ("latency".to_string(), Value::from(latency_ms)),
            ("code".to_string(), Value::from(stock_code.to_string())),
        ],
        &connector,
    );
}

/// 记录对话消息到数据库
pub fn record_conversation_message(
    session_id: &str,
    role: &str,
    content: &str,
    provider: &str,
    model: &str,
    prompt_tokens: i32,
    completion_tokens: i32,
) {
    let connector = match get_db_connector() {
        Ok(c) => c,
        Err(_) => return,
    };
    let sql = "INSERT INTO conversation_messages \
         (sessionId, role, content, llmProvider, llmModel, promptTokens, completionTokens, createTime) \
         VALUES (:sid, :role, :content, :provider, :model, :pt, :ct, NOW())";
    let _ = deck::Helper::execute(
        sql,
        vec![
            ("sid".to_string(), Value::from(session_id.to_string())),
            ("role".to_string(), Value::from(role.to_string())),
            ("content".to_string(), Value::from(content.to_string())),
            ("provider".to_string(), Value::from(provider.to_string())),
            ("model".to_string(), Value::from(model.to_string())),
            ("pt".to_string(), Value::from(prompt_tokens)),
            ("ct".to_string(), Value::from(completion_tokens)),
        ],
        &connector,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tube::value;

    #[test]
    fn test_param_string_exists() {
        let v = value!({"name": "test", "code": "600519"});
        assert_eq!(param_string(&v, "name"), "test");
        assert_eq!(param_string(&v, "code"), "600519");
    }

    #[test]
    fn test_param_string_missing() {
        let v = value!({"name": "test"});
        assert_eq!(param_string(&v, "missing"), "");
    }

    #[test]
    fn test_param_string_default_exists() {
        let v = value!({"name": "test"});
        assert_eq!(param_string_default(&v, "name", "fallback"), "test");
    }

    #[test]
    fn test_param_string_default_missing() {
        let v = value!({});
        assert_eq!(param_string_default(&v, "missing", "fallback"), "fallback");
    }

    #[test]
    fn test_param_i64() {
        let v = value!({"count": 42, "zero": 0});
        assert_eq!(param_i64(&v, "count"), 42);
        assert_eq!(param_i64(&v, "zero"), 0);
        assert_eq!(param_i64(&v, "missing"), 0);
    }

    #[test]
    fn test_param_f64() {
        let v = value!({"price": 12.5});
        assert_eq!(param_f64(&v, "price"), 12.5);
        assert_eq!(param_f64(&v, "missing"), 0.0);
    }

    #[test]
    fn test_market_prefix_sh() {
        assert_eq!(market_prefix("600519"), "sh");
        assert_eq!(market_prefix("688001"), "sh");
        assert_eq!(market_prefix("900001"), "sh");
    }

    #[test]
    fn test_market_prefix_sz() {
        assert_eq!(market_prefix("000001"), "sz");
        assert_eq!(market_prefix("002594"), "sz");
        assert_eq!(market_prefix("300750"), "sz");
    }

    #[test]
    fn test_market_prefix_bj() {
        assert_eq!(market_prefix("830001"), "bj");
        assert_eq!(market_prefix("430001"), "bj");
    }

    #[test]
    fn test_market_prefix_default() {
        assert_eq!(market_prefix("123456"), "sh");
    }
}
