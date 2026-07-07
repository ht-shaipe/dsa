//! 社交情绪服务 - 市场情绪指标、个股情绪分析、热门题材

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use tube::Value;

/// 社交情绪服务
pub struct SocialSentimentService {
    client: reqwest::Client,
}

impl SocialSentimentService {
    /// 创建社交情绪服务实例
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// 请求分发 - 可用方法: market_sentiment, stock_sentiment, hot_topics
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "market_sentiment" => self.market_sentiment(params).await,
            "stock_sentiment" => self.stock_sentiment(params).await,
            "hot_topics" => self.hot_topics(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "social_sentiment不支持方法: {}",
                method
            ))),
        }
    }

    /// 市场情绪 - 基于主要指数涨跌计算恐惧贪婪指数
    async fn market_sentiment(&self, _params: &Value) -> DsaResult<Value> {
        let url = "https://push2.eastmoney.com/api/qt/ulist.np/get?fltt=2&fields=f2,f3,f12,f14&secids=1.000001,0.399001,0.399006";
        let resp = self.client.get(url).send().await
            .map_err(|e| DsaError::StockData(format!("获取市场情绪失败: {}", e)))?;
        let body: Value = resp.json().await
            .unwrap_or(value!({"data": {"diff": []}}));
        // 根据市场涨跌计算恐惧贪婪指数
        let diff: Vec<Value> = body["data"]["diff"].as_array()
            .map(|a| a.clone())
            .unwrap_or_default();
        let changes: Vec<f64> = diff.iter()
            .filter_map(|item| item.get("f3").and_then(|v| v.as_f64()))
            .collect();
        let avg_change = if changes.is_empty() { 0.0_f64 } else { changes.iter().sum::<f64>() / changes.len() as f64 };
        // 恐惧贪婪: 0=极度恐惧, 100=极度贪婪
        let fear_greed = (50.0_f64 + avg_change * 10.0_f64).min(100.0_f64).max(0.0_f64);
        let sentiment = if fear_greed >= 75.0 { "greed" }
            else if fear_greed >= 60.0 { "optimistic" }
            else if fear_greed >= 40.0 { "neutral" }
            else if fear_greed >= 25.0 { "fearful" }
            else { "extreme_fear" };
        Ok(value!({
            "fearGreedIndex": fear_greed,
            "sentiment": sentiment,
            "marketChanges": changes,
        }))
    }

    /// 个股情绪 - 基于新闻情绪分析
    async fn stock_sentiment(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(DsaError::Validation("请提供股票代码".to_string()));
        }
        // 从数据库查询新闻情绪
        let connector = utils::get_db_connector()?;
        let sql = format!("SELECT sentiment, importance, title FROM news_intel WHERE stock_code = '{}' ORDER BY published_at DESC LIMIT 10", code);
        let rows = Helper::query_rows(&sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询新闻情绪失败: {}", e)))?;
        let items: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        let avg_sentiment = items.iter()
            .filter_map(|i| i.get("sentiment").and_then(|v| v.as_f64()))
            .sum::<f64>() / items.len().max(1) as f64;
        Ok(value!({
            "code": code,
            "sentiment_score": avg_sentiment,
            "newsCount": items.len() as i64,
            "recentNews": items,
        }))
    }

    /// 热门题材 - 从东方财富获取热门概念板块
    async fn hot_topics(&self, _params: &Value) -> DsaResult<Value> {
        let url = "https://push2.eastmoney.com/api/qt/clist/get?pn=1&pz=10&po=1&np=1&fltt=2&invt=2&fid=f3&fs=m:90+t:2&fields=f2,f3,f12,f14";
        let resp = self.client.get(url).send().await
            .map_err(|e| DsaError::StockData(format!("获取热门题材失败: {}", e)))?;
        let body: Value = resp.json().await
            .unwrap_or(value!({"data": {"diff": []}}));
        let diff: Vec<Value> = body["data"]["diff"].as_array()
            .map(|a| a.clone())
            .unwrap_or_default();
        let topics: Vec<Value> = diff.iter().map(|item| {
            value!({
                "name": item.get("f14").and_then(|v| v.as_str()).unwrap_or_default(),
                "change": item.get("f3").and_then(|v| v.as_f64()).unwrap_or(0.0),
                "code": item.get("f12").and_then(|v| v.as_str()).unwrap_or_default(),
            })
        }).collect();
        Ok(Value::Array(topics))
    }
}
