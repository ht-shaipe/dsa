//! AlphaSift 筛选服务 - 策略筛选、热点追踪

use dsa_core::{DsaError, DsaResult, utils};
use qta_crawler::{Complex, EastMoney};
use tube::Value;

/// AlphaSift筛选服务
pub struct ScreeningService {}

impl ScreeningService {
    /// 创建AlphaSift筛选服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: status, strategies, hotspots, hotspot_detail, screen
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "status" => self.status().await,
            "strategies" => self.strategies().await,
            "hotspots" => self.hotspots().await,
            "hotspot_detail" => self.hotspot_detail(params).await,
            "screen" => self.screen(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "screening不支持方法: {}",
                method
            ))),
        }
    }

    async fn status(&self) -> DsaResult<Value> {
        Ok(value!({"enabled": true, "installed": true, "version": "0.1.0"}))
    }

    async fn strategies(&self) -> DsaResult<Value> {
        Ok(value!([
            {"id": "dual_low", "name": "双低策略", "description": "低价+低市盈率筛选"},
            {"id": "breakout", "name": "突破策略", "description": "价格突破均线压力位"},
            {"id": "value", "name": "价值策略", "description": "低PB+高ROE筛选"},
            {"id": "momentum", "name": "动量策略", "description": "近期涨幅领先股票"},
        ]))
    }

    async fn hotspots(&self) -> DsaResult<Value> {
        let data = Complex::get_hot_stock()
            .await
            .map_err(|e| DsaError::StockData(format!("获取热点失败: {}", e)))?;
        let limited = if let Value::Array(arr) = data {
            Value::Array(arr.into_iter().take(12).collect())
        } else {
            data
        };
        Ok(limited)
    }

    async fn hotspot_detail(&self, params: &Value) -> DsaResult<Value> {
        let topic_val = utils::param_string(params, "topic");
        if topic_val.is_empty() {
            return Err(DsaError::Validation("请提供热点主题".to_string()));
        }

        let hot_stocks = Complex::get_hot_stock()
            .await
            .map_err(|e| DsaError::StockData(format!("获取热点数据失败: {}", e)))?;

        let matched: Vec<Value> = if let Value::Array(arr) = &hot_stocks {
            arr.iter()
                .filter(|item| {
                    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                    let desc = item.get("description").and_then(|v| v.as_str()).unwrap_or_default();
                    let reason = item.get("reason").and_then(|v| v.as_str()).unwrap_or_default();
                    name.contains(topic_val.as_str())
                        || desc.contains(topic_val.as_str())
                        || reason.contains(topic_val.as_str())
                })
                .cloned()
                .collect()
        } else {
            vec![]
        };

        let total_stocks = matched.len() as i64;

        Ok(value!({
            "topic": topic_val.clone(),
            "description": format!("{}相关热门股票", topic_val),
            "stocks": matched,
            "totalStocks": total_stocks,
        }))
    }

    async fn screen(&self, params: &Value) -> DsaResult<Value> {
        let strategy = params
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "dual_low".to_string());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(20.0) as usize;

        let em = EastMoney::new();
        let spot = em
            .stock_zh_a_spot()
            .await
            .map_err(|e| DsaError::StockData(format!("获取行情失败: {}", e)))?;

        let results: Vec<Value> = match strategy.as_str() {
            "dual_low" => self.filter_dual_low(&spot, limit),
            "breakout" => self.filter_breakout(&spot, limit),
            "value" => self.filter_value(&spot, limit),
            "momentum" => self.filter_momentum(&spot, limit),
            _ => spot.into_iter().take(limit).collect(),
        };
        let count = results.len() as i64;
        Ok(value!({"strategy": strategy, "count": count, "results": results}))
    }

    fn filter_dual_low(&self, stocks: &[Value], limit: usize) -> Vec<Value> {
        stocks
            .iter()
            .filter(|s| {
                let price = s.get("最新价").or_else(|| s.get("price"))
                    .and_then(|v| v.as_f64()).unwrap_or(999.0);
                let pe = s.get("市盈率-动态").or_else(|| s.get("pe"))
                    .and_then(|v| v.as_f64()).unwrap_or(999.0);
                let code: String = s.get("代码").or_else(|| s.get("code"))
                    .and_then(|v| v.as_str()).unwrap_or_default();
                price > 0.0 && price < 20.0 && pe > 0.0 && pe < 30.0
                    && !code.starts_with('8') && !code.starts_with('4')
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn filter_breakout(&self, stocks: &[Value], limit: usize) -> Vec<Value> {
        stocks
            .iter()
            .filter(|s| {
                let change_pct = s.get("涨跌幅").or_else(|| s.get("change_pct"))
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                let turnover = s.get("换手率").or_else(|| s.get("turnover_rate"))
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                let volume_ratio = s.get("量比").or_else(|| s.get("volume_ratio"))
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                change_pct > 3.0 && change_pct < 9.8 && turnover > 3.0 && volume_ratio > 2.0
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn filter_value(&self, stocks: &[Value], limit: usize) -> Vec<Value> {
        stocks
            .iter()
            .filter(|s| {
                let pb = s.get("市净率").or_else(|| s.get("pb"))
                    .and_then(|v| v.as_f64()).unwrap_or(999.0);
                let roe = s.get("roe").or_else(|| s.get("ROE"))
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                let pe = s.get("市盈率-动态").or_else(|| s.get("pe"))
                    .and_then(|v| v.as_f64()).unwrap_or(999.0);
                pb > 0.0 && pb < 2.0 && roe > 10.0 && pe > 0.0 && pe < 20.0
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn filter_momentum(&self, stocks: &[Value], limit: usize) -> Vec<Value> {
        let mut ranked: Vec<&Value> = stocks
            .iter()
            .filter(|s| {
                let change_pct = s.get("涨跌幅").or_else(|| s.get("change_pct"))
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                let code: String = s.get("代码").or_else(|| s.get("code"))
                    .and_then(|v| v.as_str()).unwrap_or_default();
                change_pct > 0.0 && !code.starts_with('8') && !code.starts_with('4')
            })
            .collect();
        ranked.sort_by(|a, b| {
            let ca = a.get("涨跌幅").or_else(|| a.get("change_pct"))
                .and_then(|v| v.as_f64()).unwrap_or(0.0);
            let cb = b.get("涨跌幅").or_else(|| b.get("change_pct"))
                .and_then(|v| v.as_f64()).unwrap_or(0.0);
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked.into_iter().take(limit).cloned().collect()
    }
}
