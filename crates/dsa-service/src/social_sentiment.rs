use dsa_core::db::query_rows;
use tube::{Result, Value};
use tube_web::RequestParameter;
use dsa_core::utils;

pub struct SocialSentiment {
    request: RequestParameter,
    client: reqwest::Client,
}

impl SocialSentiment {
    pub fn new(param: &RequestParameter) -> Self {
        SocialSentiment {
            request: param.clone(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "market_sentiment" => self.market_sentiment().await,
            "stock_sentiment" => self.stock_sentiment().await,
            "hot_topics" => self.hot_topics().await,
            _ => Err(tube::Error::from(format!(
                "social_sentiment不支持方法: {}",
                method
            ))),
        }
    }

    fn params(&self) -> &Value {
        &self.request.value
    }

    async fn market_sentiment(&self) -> Result<Value> {
        let url = "https://push2.eastmoney.com/api/qt/ulist.np/get?fltt=2&fields=f2,f3,f12,f14&secids=1.000001,0.399001,0.399006";
        let resp = self.client.get(url).send().await
            .map_err(|e| tube::Error::from(format!("获取市场情绪失败: {}", e)))?;
        let body: Value = resp.json().await
            .unwrap_or(value!({"data": {"diff": []}}));
        let diff: Vec<Value> = body["data"]["diff"].as_array()
            .map(|a| a.clone())
            .unwrap_or_default();
        let changes: Vec<f64> = diff.iter()
            .filter_map(|item| item.get("f3").and_then(|v| v.as_f64()))
            .collect();
        let avg_change = if changes.is_empty() { 0.0_f64 } else { changes.iter().sum::<f64>() / changes.len() as f64 };
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

    async fn stock_sentiment(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(tube::Error::msg("请提供股票代码"));
        }
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = format!("SELECT sentiment_label, sentiment_score, title FROM news_intel WHERE stock_code = '{}' ORDER BY published_at DESC LIMIT 10", code);
        let rows = query_rows(&sql, vec![], &connector)
            .map_err(|e| tube::Error::msg(format!("查询新闻情绪出错: {}", e)))?;
        let items: Vec<Value> = rows;
        let avg_sentiment = items.iter()
            .filter_map(|i| i.get("sentimentScore").and_then(|v| v.as_f64()))
            .sum::<f64>() / items.len().max(1) as f64;
        Ok(value!({
            "code": code,
            "sentimentScore": avg_sentiment,
            "newsCount": items.len() as i64,
            "recentNews": items,
        }))
    }

    async fn hot_topics(&self) -> Result<Value> {
        let url = "https://push2.eastmoney.com/api/qt/clist/get?pn=1&pz=10&po=1&np=1&fltt=2&invt=2&fid=f3&fs=m:90+t:2&fields=f2,f3,f12,f14";
        let resp = self.client.get(url).send().await
            .map_err(|e| tube::Error::from(format!("获取热门题材失败: {}", e)))?;
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
