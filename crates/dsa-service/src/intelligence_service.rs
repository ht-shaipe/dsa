//! 情报源服务 - RSS/API 数据源管理与抓取

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use tube::Value;

/// URL安全检查 - 阻止对内部地址的请求(SSRF防护)
fn is_safe_url(url: &str) -> bool {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return false;
    }
    let host_part = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    
    if host_part == "localhost" || host_part == "127.0.0.1" || host_part == "0.0.0.0" 
       || host_part == "::1" || host_part.starts_with("10.")
       || host_part.starts_with("192.168.") || host_part.starts_with("169.254.") {
        return false;
    }
    if host_part.starts_with("172.") {
        if let Some(second) = host_part.split('.').nth(1) {
            if let Ok(n) = second.parse::<u8>() {
                if n >= 16 && n <= 31 {
                    return false;
                }
            }
        }
    }
    true
}

/// 情报源服务
pub struct IntelligenceService {}

impl IntelligenceService {
    /// 创建情报源服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: sources, source_create, source_update, source_delete, source_test, source_fetch, fetch_enabled, items, templates, defaults
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "sources" => self.sources(params).await,
            "source_create" => self.source_create(params).await,
            "source_update" => self.source_update(params).await,
            "source_delete" => self.source_delete(params).await,
            "source_test" => self.source_test(params).await,
            "source_fetch" => self.source_fetch(params).await,
            "fetch_enabled" => self.fetch_enabled(params).await,
            "items" => self.items(params).await,
            "templates" => self.templates(params).await,
            "defaults" => self.defaults(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "intelligence不支持方法: {}",
                method
            ))),
        }
    }

    /// 列出情报源
    async fn sources(&self, _params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, name, source_type, url_template, config_json, scope_type, \
             scope_value, market, enabled, fetch_interval, create_time, modify_time \
             FROM intelligence_sources ORDER BY id";
        let rows = Helper::query_rows(sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询情报源失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(Value::Array(results))
    }

    /// 创建情报源
    async fn source_create(&self, params: &Value) -> DsaResult<Value> {
        let name = utils::param_string(params, "name");
        if name.is_empty() {
            return Err(DsaError::Validation("请提供名称".to_string()));
        }

        let source_type = utils::param_string(params, "type");
        if source_type.is_empty() {
            return Err(DsaError::Validation("请提供类型".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let url_template = utils::param_string(params, "url");
        let config_json = utils::param_string(params, "config");
        let market = utils::param_string(params, "market");
        let market_val = if market.is_empty() { "cn" } else { &market };
        let fetch_interval = params
            .get("fetchInterval")
            .and_then(|v| v.as_f64())
            .unwrap_or(60.0) as i32;
        let enabled = params
            .get("enabled")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as i8;

        let sql = "INSERT INTO intelligence_sources \
             (name, source_type, url_template, config_json, scope_type, scope_value, market, \
              enabled, fetch_interval, create_time, modify_time) \
             VALUES (:name, :type, :url, :config, 'all', '', :market, :enabled, :interval, NOW(), NOW())";

        let result = Helper::execute(
            sql,
            vec![
                ("name".to_string(), Value::from(name.as_str())),
                ("type".to_string(), Value::from(source_type.as_str())),
                ("url".to_string(), Value::from(url_template.as_str())),
                ("config".to_string(), Value::from(config_json.as_str())),
                ("market".to_string(), Value::from(market_val)),
                ("enabled".to_string(), Value::from(enabled)),
                ("interval".to_string(), Value::from(fetch_interval)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("创建情报源失败: {}", e)))?;

        Ok(value!({"id": result as i64, "name": name}))
    }

    /// 更新情报源
    async fn source_update(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let name = utils::param_string(params, "name");
        let url_template = utils::param_string(params, "url");
        let config_json = utils::param_string(params, "config");
        let enabled = params
            .get("enabled")
            .and_then(|v| v.as_f64())
            .map(|v| v as i8);
        let fetch_interval = params
            .get("fetchInterval")
            .and_then(|v| v.as_f64())
            .map(|v| v as i32);

        // 构建动态更新
        let mut sets = Vec::new();
        let mut p: Vec<(String, Value)> = Vec::new();
        p.push(("id".to_string(), Value::from(id)));

        if !name.is_empty() {
            sets.push("name = :name".to_string());
            p.push(("name".to_string(), Value::from(name.as_str())));
        }
        if !url_template.is_empty() {
            sets.push("url_template = :url".to_string());
            p.push(("url".to_string(), Value::from(url_template.as_str())));
        }
        if !config_json.is_empty() {
            sets.push("config_json = :config".to_string());
            p.push(("config".to_string(), Value::from(config_json.as_str())));
        }
        if let Some(e) = enabled {
            sets.push("enabled = :enabled".to_string());
            p.push(("enabled".to_string(), Value::from(e)));
        }
        if let Some(fi) = fetch_interval {
            sets.push("fetch_interval = :fi".to_string());
            p.push(("fi".to_string(), Value::from(fi)));
        }

        if sets.is_empty() {
            return Ok(value!({"message": "无更新内容"}));
        }

        sets.push("modify_time = NOW()".to_string());
        let sql = format!(
            "UPDATE intelligence_sources SET {} WHERE id = :id",
            sets.join(", ")
        );

        Helper::execute(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("更新情报源失败: {}", e)))?;

        Ok(value!({"id": id}))
    }

    /// 删除情报源
    async fn source_delete(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        // 软删除: 设置enabled=0
        let sql = "UPDATE intelligence_sources SET enabled = 0, modify_time = NOW() WHERE id = :id";
        Helper::execute(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| DsaError::Database(format!("删除情报源失败: {}", e)))?;

        Ok(value!({"id": id}))
    }

    /// 测试情报源连通性
    async fn source_test(&self, params: &Value) -> DsaResult<Value> {
        let url = utils::param_string(params, "url");
        if url.is_empty() {
            return Err(DsaError::Validation("请提供URL".to_string()));
        }

        if !is_safe_url(&url) {
            return Err(DsaError::Validation("不安全的URL: 不允许访问内部地址".to_string()));
        }

        match reqwest::get(&url).await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                Ok(value!({
                    "reachable": true,
                    "httpStatus": status as i64,
                }))
            }
            Err(e) => Ok(value!({
                "reachable": false,
                "error": e.to_string(),
            })),
        }
    }

    /// 抓取单个情报源
    async fn source_fetch(&self, params: &Value) -> DsaResult<Value> {
        let source_id = params
            .get("sourceId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if source_id == 0 {
            return Err(DsaError::Validation("请提供sourceId".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, name, source_type, url_template, config_json, market \
             FROM intelligence_sources WHERE id = :id AND enabled = 1";
        let rows = Helper::query_rows(
            sql,
            vec![("id".to_string(), Value::from(source_id))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询情报源失败: {}", e)))?;

        if rows.is_empty() {
            return Err(DsaError::Validation(format!(
                "情报源不存在或未启用: {}",
                source_id
            )));
        }

        let row = &rows[0];
        let source_type = row.get_string(2);
        let url_template = row.get_string(3);
        let market = row.get_string(5);

        let items = match source_type.as_str() {
            "rss" | "atom" => self.fetch_rss(source_id, &url_template, &market).await?,
            "api" => self.fetch_api(source_id, &url_template, &market).await?,
            _ => vec![],
        };

        Ok(value!({
            "sourceId": source_id,
            "fetchedCount": items.len() as i64,
        }))
    }

    /// 抓取所有启用的源
    async fn fetch_enabled(&self, _params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, name, source_type, url_template, config_json, market \
             FROM intelligence_sources WHERE enabled = 1";
        let rows = Helper::query_rows(sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询情报源失败: {}", e)))?;

        let mut total_fetched = 0i64;
        for row in &rows {
            let source_id: i64 = row.get_value(0).as_f64().unwrap_or(0.0) as i64;
            let source_type = row.get_string(2);
            let url_template = row.get_string(3);
            let market = row.get_string(5);

            let items = match source_type.as_str() {
                "rss" | "atom" => self.fetch_rss(source_id, &url_template, &market).await?,
                "api" => self.fetch_api(source_id, &url_template, &market).await?,
                _ => vec![],
            };
            total_fetched += items.len() as i64;
        }

        Ok(value!({
            "sourcesProcessed": rows.len() as i64,
            "totalFetched": total_fetched,
        }))
    }

    /// 查询情报条目
    async fn items(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let source_id = params
            .get("sourceId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let (sql, p) = if source_id > 0 {
            (
                "SELECT id, source_id, title, content, source_url, scope_type, scope_value, \
                 market, published_at, fetched_at, create_time \
                 FROM intelligence_items WHERE source_id = :sid \
                 ORDER BY fetched_at DESC LIMIT :limit"
                    .to_string(),
                vec![
                    ("sid".to_string(), Value::from(source_id)),
                    ("limit".to_string(), Value::from(limit)),
                ],
            )
        } else {
            (
                "SELECT id, source_id, title, content, source_url, scope_type, scope_value, \
                 market, published_at, fetched_at, create_time \
                 FROM intelligence_items \
                 ORDER BY fetched_at DESC LIMIT :limit"
                    .to_string(),
                vec![("limit".to_string(), Value::from(limit))],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询情报条目失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(Value::Array(results))
    }

    /// 返回内置模板
    async fn templates(&self, _params: &Value) -> DsaResult<Value> {
        let templates = vec![
            value!({
                "name": "eastmoney_news",
                "type": "rss",
                "url": "http://feed.eastmoney.com/news.xml",
                "market": "cn",
                "description": "东方财富新闻"
            }),
            value!({
                "name": "sina_finance",
                "type": "rss",
                "url": "https://finance.sina.com.cn/7x24/",
                "market": "cn",
                "description": "新浪财经"
            }),
            value!({
                "name": "cls_telegraph",
                "type": "api",
                "url": "https://www.cls.cn/api/telegraph",
                "market": "cn",
                "description": "财联社电报"
            }),
        ];

        Ok(Value::Array(templates))
    }

    /// 从模板创建默认源
    async fn defaults(&self, _params: &Value) -> DsaResult<Value> {
        let templates = self.templates(_params).await?;
        let items = tube::Value::as_array(&templates.get("data").cloned().unwrap_or(Value::Null))
            .unwrap_or_default();

        let mut created = 0i64;
        for tmpl in &items {
            let name = tmpl.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let source_type = tmpl.get("type").and_then(|v| v.as_str()).unwrap_or_default();
            let url = tmpl.get("url").and_then(|v| v.as_str()).unwrap_or_default();
            let market = tmpl.get("market").and_then(|v| v.as_str()).unwrap_or_else(|| "cn".to_string());

            let create_params = value!({
                "name": name,
                "type": source_type,
                "url": url,
                "market": market,
            });

            match self.source_create(&create_params).await {
                Ok(_) => created += 1,
                Err(_) => {} // 已存在则跳过
            }
        }

        Ok(value!({
            "created": created, "total": items.len() as i64
        }))
    }

    /// 抓取RSS/Atom源
    async fn fetch_rss(&self, source_id: i64, url: &str, market: &str) -> DsaResult<Vec<Value>> {
        if !is_safe_url(url) {
            return Err(DsaError::Validation("不安全的URL: 不允许访问内部地址".to_string()));
        }

        let response = reqwest::get(url).await.map_err(|e| {
            DsaError::Database(format!("RSS抓取失败: {}", e))
        })?;

        let body = response
            .text()
            .await
            .map_err(|e| DsaError::Database(format!("RSS读取失败: {}", e)))?;

        let connector = utils::get_db_connector()?;
        let mut items = Vec::new();

        // 简单XML解析: 提取 <item> 中的 <title> 和 <link>
        for item_str in Self::extract_xml_items(&body) {
            let title = Self::extract_xml_tag(&item_str, "title");
            let link = Self::extract_xml_tag(&item_str, "link");
            let description = Self::extract_xml_tag(&item_str, "description");

            if title.is_empty() {
                continue;
            }

            // 去重: 根据sourceUrl
            let check_sql = "SELECT id FROM intelligence_items WHERE source_url = :url LIMIT 1";
            let existing = Helper::query_rows(
                check_sql,
                vec![("url".to_string(), Value::from(link.as_str()))],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("去重检查失败: {}", e)))?;

            if !existing.is_empty() {
                continue;
            }

            let insert_sql = "INSERT INTO intelligence_items \
                 (source_id, title, content, source_url, scope_type, scope_value, market, \
                  published_at, fetched_at, create_time) \
                 VALUES (:sid, :title, :content, :url, 'all', '', :market, NOW(), NOW(), NOW())";
            if let Err(e) = Helper::execute(
                insert_sql,
                vec![
                    ("sid".to_string(), Value::from(source_id)),
                    ("title".to_string(), Value::from(title.as_str())),
                    ("content".to_string(), Value::from(description.as_str())),
                    ("url".to_string(), Value::from(link.as_str())),
                    ("market".to_string(), Value::from(market)),
                ],
                &connector,
            ) {
                tracing::warn!("插入情报条目失败: {}", e);
            }

            items.push(value!({"title": title, "link": link}));
        }

        Ok(items)
    }

    /// 抓取API源 (简单GET, 返回JSON数组)
    async fn fetch_api(&self, source_id: i64, url: &str, market: &str) -> DsaResult<Vec<Value>> {
        if !is_safe_url(url) {
            return Err(DsaError::Validation("不安全的URL: 不允许访问内部地址".to_string()));
        }

        let response = reqwest::get(url).await.map_err(|e| {
            DsaError::Database(format!("API抓取失败: {}", e))
        })?;

        let body = response
            .text()
            .await
            .map_err(|e| DsaError::Database(format!("API读取失败: {}", e)))?;

        let connector = utils::get_db_connector()?;
        let mut items = Vec::new();

        // 尝试解析为JSON数组
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
            for item in &arr {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let link = item
                    .get("url")
                    .or_else(|| item.get("link"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let content = item
                    .get("content")
                    .or_else(|| item.get("description"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                if title.is_empty() {
                    continue;
                }

                // 去重
                let check_sql = "SELECT id FROM intelligence_items WHERE source_url = :url LIMIT 1";
                let existing = Helper::query_rows(
                    check_sql,
                    vec![("url".to_string(), Value::from(link.to_string()))],
                    &connector,
                )
                .map_err(|e| DsaError::Database(format!("去重检查失败: {}", e)))?;

                if !existing.is_empty() {
                    continue;
                }

                let insert_sql = "INSERT INTO intelligence_items \
                     (source_id, title, content, source_url, scope_type, scope_value, market, \
                      published_at, fetched_at, create_time) \
                     VALUES (:sid, :title, :content, :url, 'all', '', :market, NOW(), NOW(), NOW())";
                if let Err(e) = Helper::execute(
                    insert_sql,
                    vec![
                        ("sid".to_string(), Value::from(source_id)),
                        ("title".to_string(), Value::from(title.to_string())),
                        ("content".to_string(), Value::from(content.to_string())),
                        ("url".to_string(), Value::from(link.to_string())),
                        ("market".to_string(), Value::from(market.to_string())),
                    ],
                    &connector,
                ) {
                    tracing::warn!("插入情报条目失败: {}", e);
                }

                items.push(value!({"title": title.to_string(), "link": link.to_string()}));
            }
        }

        Ok(items)
    }

    /// 从XML文本中提取 <item>...</item> 片段
    fn extract_xml_items(xml: &str) -> Vec<String> {
        let mut items = Vec::new();
        let lower = xml.to_lowercase();
        let mut start = 0;

        while let Some(s) = lower[start..].find("<item") {
            let abs_start = start + s;
            if let Some(e) = lower[abs_start..].find("</item>") {
                items.push(xml[abs_start..abs_start + e + 7].to_string());
                start = abs_start + e + 7;
            } else {
                break;
            }
        }

        // Atom格式: <entry>...</entry>
        start = 0;
        while let Some(s) = lower[start..].find("<entry") {
            let abs_start = start + s;
            if let Some(e) = lower[abs_start..].find("</entry>") {
                items.push(xml[abs_start..abs_start + e + 8].to_string());
                start = abs_start + e + 8;
            } else {
                break;
            }
        }

        items
    }

    /// 从XML片段中提取标签内容
    fn extract_xml_tag(fragment: &str, tag: &str) -> String {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        if let Some(start) = fragment.find(&open) {
            let content_start = start + open.len();
            if let Some(end) = fragment[content_start..].find(&close) {
                return fragment[content_start..content_start + end]
                    .trim()
                    .to_string();
            }
        }
        String::new()
    }
}
