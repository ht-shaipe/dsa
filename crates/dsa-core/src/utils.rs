//! 共享工具函数 - 参数提取、市场前缀、数据库连接、行情数据获取

use crate::models::KlineBar;
use crate::DsaError;
use deck_connector::{get_connector, Connector};
use qta_crawler::{EastMoney, History, QQ, Real};
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

/// 获取默认数据库连接器（根据配置自动选择MySQL/SQLite）
pub fn get_db_connector() -> Result<Connector, DsaError> {
    crate::db::get_db_connector()
}

/// 从东方财富获取K线数据并写入数据库，失败时回退到新浪接口
pub async fn fetch_kline(code: &str, period: &str) -> Result<Vec<KlineBar>, DsaError> {
    match fetch_kline_sina(code, period).await {
        Ok(bars) if !bars.is_empty() => {
            save_kline_to_db(code, &bars);
            return Ok(bars);
        }
        _ => {}
    }

    let em = EastMoney::new();
    for i in 0..2 {
        match em.stock_zh_a_hist(code, Some(period), None, None, Some("qfq")).await {
            Ok(raw) => {
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
                return Ok(bars);
            }
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_millis(500 * (i as u64 + 1))).await;
            }
        }
    }

    Err(DsaError::StockData("获取K线数据失败".to_string()))
}

/// 从新浪财经获取K线数据（作为东方财富的备选数据源）
async fn fetch_kline_sina(code: &str, period: &str) -> Result<Vec<KlineBar>, DsaError> {
    let scale: u32 = match period {
        "weekly" => 1200,
        "monthly" => 5200,
        _ => 240,
    };
    let ma = "5,10,20,30,60";
    let length: u32 = 500;

    match History::get_price(code, scale, ma, length).await {
        Ok(raw) => {
            let arr = raw.as_array().ok_or_else(|| DsaError::StockData("新浪K线数据格式异常".to_string()))?;
            let mut bars = Vec::with_capacity(arr.len());
            for item in arr {
                bars.push(KlineBar {
                    date: item.get("day").and_then(|v| v.as_str()).unwrap_or_default(),
                    open: item.get("open").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    high: item.get("high").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    low: item.get("low").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    close: item.get("close").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    volume: item.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
                    amount: 0.0,
                });
            }
            if bars.is_empty() {
                return Err(DsaError::StockData("新浪K线返回空数据".to_string()));
            }
            save_kline_to_db(code, &bars);
            Ok(bars)
        }
        Err(e) => Err(DsaError::StockData(format!("获取K线数据失败(东方财富和新浪均不可用): {}", e))),
    }
}

pub fn save_kline_to_db(code: &str, bars: &[KlineBar]) {
    save_kline_to_db_impl(code, bars, 5)
}

pub fn save_all_kline_to_db(code: &str, bars: &[KlineBar]) {
    save_kline_to_db_impl(code, bars, bars.len())
}

fn save_kline_to_db_impl(code: &str, bars: &[KlineBar], max_count: usize) {
    let connector = match get_db_connector() {
        Ok(c) => c,
        Err(_) => return,
    };
    let is_sqlite = crate::get_global_config().database.is_sqlite();
    let now_expr = if is_sqlite { "datetime('now')" } else { "NOW()" };

    let stock_name = match crate::db::query_rows(
        "SELECT stock_name FROM stock_daily WHERE stock_code = :code AND stock_name != '' AND status >= 1 LIMIT 1",
        vec![("code".to_string(), Value::from(code.to_string()))],
        &connector,
    ) {
        Ok(rows) => crate::db::first_row_string(&rows, "stockName"),
        Err(_) => String::new(),
    };

    let sql = if is_sqlite {
        format!(
            "INSERT INTO stock_daily \
             (stock_code, stock_name, trade_date, open, high, low, close, volume, amount, status, create_time) \
             VALUES (:code, :name, :date, :open, :high, :low, :close, :vol, :amt, 1, {}) \
             ON CONFLICT(stock_code, trade_date) DO UPDATE SET \
             stock_name=CASE WHEN excluded.stock_name != '' THEN excluded.stock_name ELSE stock_daily.stock_name END, \
             open=excluded.open, high=excluded.high, low=excluded.low, \
             close=excluded.close, volume=excluded.volume, amount=excluded.amount",
            now_expr
        )
    } else {
        format!(
            "INSERT INTO stock_daily \
             (stock_code, stock_name, trade_date, open, high, low, close, volume, amount, status, create_time) \
             VALUES (:code, :name, :date, :open, :high, :low, :close, :vol, :amt, 1, {}) \
             ON DUPLICATE KEY UPDATE \
             stock_name=IF(VALUES(stock_name)!='', VALUES(stock_name), stock_name), \
             open=VALUES(open), high=VALUES(high), low=VALUES(low), \
             close=VALUES(close), volume=VALUES(volume), amount=VALUES(amount)",
            now_expr
        )
    };

    for bar in bars.iter().rev().take(max_count) {
        let _ = crate::db::execute(
            &sql,
            vec![
                ("code".to_string(), Value::from(code.to_string())),
                ("name".to_string(), Value::from(stock_name.as_str())),
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
    let pure_code = code.trim_start_matches("sh").trim_start_matches("sz").trim_start_matches("bj");
    let prefix = market_prefix(pure_code);
    let symbol = format!("{}{}", prefix, pure_code);

    let conf = crate::get_global_config();
    if conf.stock.realtime_source_priority.is_empty() {
        let qq = QQ::new();
        return qq.get_realtime_quote(&symbol)
            .await
            .map_err(|e| DsaError::StockData(format!("获取实时行情失败: {}", e)));
    }

    for source in &conf.stock.realtime_source_priority {
        let result: Result<Value, DsaError> = match source.as_str() {
            "tencent" | "qq" => {
                let qq = QQ::new();
                qq.get_realtime_quote(&symbol).await
                    .map_err(|e| DsaError::StockData(format!("{}", e)))
            }
            "sina" | "real" => {
                let real = Real::new();
                real.get_price(&symbol).await
                    .map_err(|e| DsaError::StockData(format!("{}", e)))
            }
            "eastmoney" => {
                let em = EastMoney::new();
                em.stock_zh_a_spot().await
                    .map(|spot: Vec<Value>| {
                        spot.iter()
                            .find(|item| {
                                let item_code: String = item.get("代码").or_else(|| item.get("code"))
                                    .and_then(|v| v.as_str()).unwrap_or_default();
                                item_code == code
                            })
                            .cloned()
                            .unwrap_or_else(|| tube::value!({}))
                    })
                    .map_err(|e| DsaError::StockData(format!("{}", e)))
            }
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

    let qq = QQ::new();
    qq.get_realtime_quote(&symbol)
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
    record_llm_usage_with_cache(provider, model, operation_type, prompt_tokens, completion_tokens, latency_ms, stock_code, 0)
}

pub fn record_llm_usage_with_cache(
    provider: &str,
    model: &str,
    operation_type: &str,
    prompt_tokens: i32,
    completion_tokens: i32,
    latency_ms: i64,
    stock_code: &str,
    cache_hit: i32,
) {
    let connector = match get_db_connector() {
        Ok(c) => c,
        Err(_) => return,
    };
    let sql = "INSERT INTO llm_usage \
         (llm_provider, llm_model, operation_type, prompt_tokens, completion_tokens, total_tokens, \
          cache_hit, latency_ms, stock_code, create_time) \
         VALUES (:provider, :model, :op, :pt, :ct, :tt, :cache, :latency, :code, NOW())";
    let total = prompt_tokens + completion_tokens;
    let _ = crate::db::execute(
        sql,
        vec![
            ("provider".to_string(), Value::from(provider.to_string())),
            ("model".to_string(), Value::from(model.to_string())),
            ("op".to_string(), Value::from(operation_type.to_string())),
            ("pt".to_string(), Value::from(prompt_tokens)),
            ("ct".to_string(), Value::from(completion_tokens)),
            ("tt".to_string(), Value::from(total)),
            ("cache".to_string(), Value::from(cache_hit)),
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
         (session_id, role, content, llm_provider, llm_model, prompt_tokens, completion_tokens, create_time) \
         VALUES (:sid, :role, :content, :provider, :model, :pt, :ct, NOW())";
    let _ = crate::db::execute(
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
