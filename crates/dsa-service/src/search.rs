use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct Search {
    request: RequestParameter,
    client: reqwest::Client,
}

impl Search {
    pub fn new(param: &RequestParameter) -> Self {
        Search {
            request: param.clone(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "search" => self.search().await,
            "stock_news" => self.stock_news().await,
            "providers" => self.providers().await,
            _ => Err(tube::Error::from(format!(
                "search不支持方法: {}",
                method
            ))),
        }
    }

    fn params(&self) -> &Value {
        &self.request.value
    }

    async fn search(&self) -> Result<Value> {
        let params = self.params();
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let provider = params
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if query.is_empty() {
            return Err(tube::Error::from("请提供搜索关键词".to_string()));
        }

        let conf = dsa_core::get_global_config();
        let resolved_provider = if provider.is_empty() {
            conf.search.default_provider.clone()
        } else {
            provider.to_string()
        };

        tracing::info!("[搜索] provider={} query={}", resolved_provider, query);

        let results = match resolved_provider.as_str() {
            "serper" => self.search_serper(&query, &conf).await?,
            "google" => self.search_google(&query, &conf).await?,
            "bing" => self.search_bing(&query, &conf).await?,
            _ => {
                return Err(tube::Error::from(format!(
                    "不支持的搜索提供商: {}",
                    resolved_provider
                )));
            }
        };

        Ok(value!({
            "query": query,
            "provider": resolved_provider,
            "results": results,
        }))
    }

    async fn stock_news(&self) -> Result<Value> {
        let params = self.params();
        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0) as usize;

        if code.is_empty() {
            return Err(tube::Error::from("请提供股票代码".to_string()));
        }

        let conf = dsa_core::get_global_config();
        let search_query = if name.is_empty() {
            format!("{} 股票 最新新闻", code)
        } else {
            format!("{} {} 股票 最新新闻", code, name)
        };

        let mut all_results: Vec<Value> = Vec::new();

        if let Ok(results) = self.search_serper(&search_query, &conf).await {
            all_results.extend(results);
        }

        if all_results.len() < limit {
            if let Ok(rss_results) = self.fetch_eastmoney_news(&code).await {
                all_results.extend(rss_results);
            }
        }

        if all_results.len() < limit {
            if let Ok(rss_results) = self.fetch_sina_news(&code).await {
                all_results.extend(rss_results);
            }
        }

        all_results.truncate(limit);

        self.save_news_to_db(&code, &all_results);

        Ok(value!({
            "code": code,
            "results": all_results,
        }))
    }

    fn save_news_to_db(&self, code: &str, results: &[Value]) {
        let connector = match dsa_core::utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => return,
        };
        for item in results.iter().take(5) {
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default();
            let url = item.get("url").and_then(|v| v.as_str()).unwrap_or_default();
            let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or_default();
            let source = item.get("source").and_then(|v| v.as_str()).unwrap_or_default();
            if title.is_empty() {
                continue;
            }
            let check = "SELECT id FROM news_intel WHERE source_url = :url LIMIT 1";
            if let Ok(existing) = deck_mysql::Helper::query_rows(
                check,
                vec![("url".to_string(), Value::from(url.to_string()))],
                &connector,
            ) {
                if !existing.is_empty() {
                    continue;
                }
            }
            let sql = "INSERT INTO news_intel \
                 (stockCode, title, summary, sourceUrl, source, status, createTime) \
                 VALUES (:code, :title, :summary, :url, :source, 1, NOW())";
            let _ = deck_mysql::Helper::execute(
                sql,
                vec![
                    ("code".to_string(), Value::from(code.to_string())),
                    ("title".to_string(), Value::from(title.to_string())),
                    ("summary".to_string(), Value::from(snippet.to_string())),
                    ("url".to_string(), Value::from(url.to_string())),
                    ("source".to_string(), Value::from(source.to_string())),
                ],
                &connector,
            );
        }
    }

    async fn providers(&self) -> Result<Value> {
        let conf = dsa_core::get_global_config();
        let serper_key = Self::resolve_key(&conf.search.serper_api_key, &conf.search.serper_api_key_env);
        let google_key = Self::resolve_key(&conf.search.google_api_key, &conf.search.google_api_key_env);
        let bing_key = Self::resolve_key(&conf.search.bing_api_key, &conf.search.bing_api_key_env);

        Ok(value!([
            {"id": "serper", "name": "Serper (Google SERP)", "enabled": !serper_key.is_empty()},
            {"id": "google", "name": "Google Custom Search", "enabled": !google_key.is_empty() && !conf.search.google_cx.is_empty()},
            {"id": "bing", "name": "Bing Search", "enabled": !bing_key.is_empty()},
        ]))
    }

    async fn search_serper(
        &self,
        query: &str,
        conf: &dsa_core::config::AppConfig,
    ) -> Result<Vec<Value>> {
        let api_key = Self::resolve_key(&conf.search.serper_api_key, &conf.search.serper_api_key_env);
        if api_key.is_empty() {
            tracing::warn!("[搜索] Serper API Key 未配置, 跳过");
            return Ok(vec![]);
        }

        let resp = self
            .client
            .post("https://google.serper.dev/search")
            .header("X-API-KEY", &api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "q": query, "gl": "cn", "hl": "zh-cn" }))
            .send()
            .await
            .map_err(|e| tube::Error::from(format!("Serper请求失败: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!("[搜索] Serper返回错误 status={} body={}", status, body);
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| tube::Error::from(format!("Serper响应解析失败: {}", e)))?;

        let empty_vec = vec![];
        let organic = json
            .get("organic")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let results: Vec<Value> = organic
            .iter()
            .filter_map(|item| {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let link = item.get("link").and_then(|v| v.as_str()).unwrap_or("");
                let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
                if title.is_empty() && link.is_empty() {
                    return None;
                }
                Some(value!({
                    "title": title,
                    "link": link,
                    "snippet": snippet,
                }))
            })
            .collect();

        Ok(results)
    }

    async fn search_google(
        &self,
        query: &str,
        conf: &dsa_core::config::AppConfig,
    ) -> Result<Vec<Value>> {
        let api_key = Self::resolve_key(&conf.search.google_api_key, &conf.search.google_api_key_env);
        let cx = &conf.search.google_cx;
        if api_key.is_empty() || cx.is_empty() {
            tracing::warn!("[搜索] Google Custom Search API Key 或 CX 未配置, 跳过");
            return Ok(vec![]);
        }

        let url = format!(
            "https://www.googleapis.com/customsearch/v1?key={}&cx={}&q={}&lr=lang_zh-CN",
            api_key, cx,
            urlencoding::encode(query)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| tube::Error::from(format!("Google搜索请求失败: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!("[搜索] Google返回错误 status={} body={}", status, body);
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| tube::Error::from(format!("Google搜索响应解析失败: {}", e)))?;

        let empty_vec = vec![];
        let items = json
            .get("items")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let results: Vec<Value> = items
            .iter()
            .filter_map(|item| {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default();
                let link = item.get("link").and_then(|v| v.as_str()).unwrap_or_default();
                let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or_default();
                if title.is_empty() {
                    return None;
                }
                Some(value!({
                    "title": title,
                    "url": link,
                    "snippet": snippet,
                    "source": "google",
                }))
            })
            .collect();

        Ok(results)
    }

    async fn search_bing(
        &self,
        query: &str,
        conf: &dsa_core::config::AppConfig,
    ) -> Result<Vec<Value>> {
        let api_key = Self::resolve_key(&conf.search.bing_api_key, &conf.search.bing_api_key_env);
        if api_key.is_empty() {
            tracing::warn!("[搜索] Bing Search API Key 未配置, 跳过");
            return Ok(vec![]);
        }

        let url = format!(
            "https://api.bing.microsoft.com/v7.0/search?q={}&mkt=zh-CN",
            urlencoding::encode(query)
        );

        let resp = self
            .client
            .get(&url)
            .header("Ocp-Apim-Subscription-Key", &api_key)
            .send()
            .await
            .map_err(|e| tube::Error::from(format!("Bing搜索请求失败: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!("[搜索] Bing返回错误 status={} body={}", status, body);
            return Ok(vec![]);
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| tube::Error::from(format!("Bing搜索响应解析失败: {}", e)))?;

        let empty_vec = vec![];
        let web_pages = json
            .get("webPages")
            .and_then(|w| w.get("value"))
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let results: Vec<Value> = web_pages
            .iter()
            .filter_map(|item| {
                let title = item.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                let link = item.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or_default();
                if title.is_empty() {
                    return None;
                }
                Some(value!({
                    "title": title,
                    "url": link,
                    "snippet": snippet,
                    "source": "bing",
                }))
            })
            .collect();

        Ok(results)
    }

    async fn fetch_eastmoney_news(&self, code: &str) -> Result<Vec<Value>> {
        let url = format!(
            "https://search-api-web.eastmoney.com/search/jsonp?cb=cb&param=%7B%22uid%22%3A%22%22%2C%22keyword%22%3A%22{}%22%2C%22type%22%3A%5B%22cmsArticleWebOld%22%5D%2C%22client%22%3A%22web%22%2C%22clientVersion%22%3A%22curr%22%2C%22param%22%3A%7B%22cmsArticleWebOld%22%3A%7B%22searchScope%22%3A%22default%22%2C%22sort%22%3A%22default%22%2C%22pageIndex%22%3A1%2C%22pageSize%22%3A10%2C%22preTag%22%3A%22%22%2C%22postTag%22%3A%22%22%7D%7D%7D",
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
                tracing::warn!("[搜索] 东方财富新闻请求失败: {}", e);
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

    async fn fetch_sina_news(&self, code: &str) -> Result<Vec<Value>> {
        let market_prefix = if code.starts_with('6') || code.starts_with('9') {
            "sh"
        } else {
            "sz"
        };
        let url = format!(
            "https://feed.mix.sina.com.cn/api/roll/get?pageid=153&lid=2514&num=10&versionNumber=1.2.4&page=1&keyword={}{}",
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
                tracing::warn!("[搜索] 新浪财经新闻请求失败: {}", e);
                Ok(vec![])
            }
        }
    }

    fn resolve_key(direct: &str, env_name: &str) -> String {
        if !direct.is_empty() {
            return direct.to_string();
        }
        if !env_name.is_empty() {
            if let Ok(key) = std::env::var(env_name) {
                return key;
            }
        }
        String::new()
    }
}
