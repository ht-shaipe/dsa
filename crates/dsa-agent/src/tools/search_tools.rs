//! 搜索工具 - 新闻/网页搜索

use dsa_core::DsaResult;
use tube::Value;

pub struct SearchTools {
    client: reqwest::Client,
}

impl SearchTools {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// 搜索股票新闻 - 聚合东方财富 + 新浪财经
    pub async fn search_stock_news(&self, query: &str) -> DsaResult<Value> {
        let mut all_results: Vec<Value> = Vec::new();

        if let Ok(items) = self.fetch_eastmoney_news(query).await {
            all_results.extend(items);
        }

        if let Ok(items) = self.fetch_sina_news(query).await {
            all_results.extend(items);
        }

        all_results.truncate(10);

        let total = all_results.len() as i64;

        Ok(value!({
            "query": query,
            "results": Value::from(all_results),
            "total": total,
        }))
    }

    async fn fetch_eastmoney_news(&self, code: &str) -> DsaResult<Vec<Value>> {
        let url = format!(
            "https://search-api-web.eastmoney.com/search/jsonp?cb=cb&param=%7B%22uid%22%3A%22%22%2C%22keyword%22%3A%22{}%22%2C%22type%22%3A%5B%22cmsArticleWebOld%22%5D%2C%22client%22%3A%22web%22%2C%22clientVersion%22%3A%22curr%22%2C%22param%22%3A%7B%22cmsArticleWebOld%22%3A%7B%22searchScope%22%3A%22default%22%2C%22sort%22%3A%22default%22%2C%22pageIndex%22%3A1%2C%22pageSize%22%3A5%2C%22preTag%22%3A%22%22%2C%22postTag%22%3A%22%22%7D%7D%7D",
            urlencoding::encode(code)
        );

        let resp = self
            .client
            .get(&url)
            .header("Referer", "https://so.eastmoney.com/")
            .send()
            .await;

        match resp {
            Ok(response) => {
                if !response.status().is_success() {
                    return Ok(vec![]);
                }
                let body = response.text().await.unwrap_or_default();
                Ok(self.parse_eastmoney_jsonp(&body))
            }
            Err(e) => {
                tracing::warn!("[搜索工具] 东方财富请求失败: {}", e);
                Ok(vec![])
            }
        }
    }

    fn parse_eastmoney_jsonp(&self, body: &str) -> Vec<Value> {
        let json_str = if body.starts_with("cb(") && body.ends_with(')') {
            &body[3..body.len() - 1]
        } else {
            body
        };

        let json: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        let empty_vec = vec![];
        let articles = json
            .get("result")
            .and_then(|r| r.get("cmsArticleWebOld"))
            .and_then(|r| r.get("list"))
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        articles
            .iter()
            .filter_map(|item| {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default();
                let url = item
                    .get("url")
                    .or_else(|| item.get("articleUrl"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let content = item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if title.is_empty() {
                    return None;
                }
                Some(value!({
                    "title": title,
                    "url": url,
                    "snippet": content,
                    "source": "eastmoney",
                }))
            })
            .collect()
    }

    async fn fetch_sina_news(&self, code: &str) -> DsaResult<Vec<Value>> {
        let market_prefix = if code.starts_with('6') || code.starts_with('9') {
            "sh"
        } else {
            "sz"
        };
        let url = format!(
            "https://feed.mix.sina.com.cn/api/roll/get?pageid=153&lid=2514&num=5&versionNumber=1.2.4&page=1&keyword={}{}",
            market_prefix, code
        );

        let resp = self.client.get(&url).send().await;

        match resp {
            Ok(response) => {
                if !response.status().is_success() {
                    return Ok(vec![]);
                }
                let json: serde_json::Value = match response.json().await {
                    Ok(v) => v,
                    Err(_) => return Ok(vec![]),
                };

                let empty_vec = vec![];
                let data = json
                    .get("result")
                    .and_then(|r| r.get("data"))
                    .and_then(|v| v.as_array())
                    .unwrap_or(&empty_vec);

                let results: Vec<Value> = data
                    .iter()
                    .filter_map(|item| {
                        let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default();
                        let url = item.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                        let intro = item.get("intro").and_then(|v| v.as_str()).unwrap_or_default();
                        if title.is_empty() {
                            return None;
                        }
                        Some(value!({
                            "title": title,
                            "url": url,
                            "snippet": intro,
                            "source": "sina",
                        }))
                    })
                    .collect();

                Ok(results)
            }
            Err(e) => {
                tracing::warn!("[搜索工具] 新浪财经请求失败: {}", e);
                Ok(vec![])
            }
        }
    }
}
