use dsa_core::db::{execute, query_rows, row_get_f64, row_get_string};
use dsa_core::utils;
use tube::{Result, Value};
use tube_web::RequestParameter;

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

pub struct Intelligence {
    request: RequestParameter,
    client: reqwest::Client,
}

impl Intelligence {
    pub fn new(param: &RequestParameter) -> Self {
        Intelligence {
            request: param.clone(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "sources" => self.sources().await,
            "source_create" => self.source_create().await,
            "source_update" => self.source_update().await,
            "source_delete" => self.source_delete().await,
            "source_test" => self.source_test().await,
            "source_fetch" => self.source_fetch().await,
            "fetch_enabled" => self.fetch_enabled().await,
            "items" => self.items().await,
            "templates" => self.templates().await,
            "defaults" => self.defaults().await,
            _ => Err(tube::Error::from(format!(
                "intelligence不支持方法: {}",
                method
            ))),
        }
    }

    fn params(&self) -> &Value {
        &self.request.value
    }

    async fn sources(&self) -> Result<Value> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, name, source_type, url_template, config_json, scope_type, \
             scope_value, market, enabled, fetch_interval, create_time, modify_time \
             FROM intelligence_sources ORDER BY id";
        let rows = query_rows(sql, vec![], &connector)
            .map_err(|e| tube::Error::from(format!("查询情报源失败: {}", e)))?;
        Ok(Value::Array(rows))
    }

    async fn source_create(&self) -> Result<Value> {
        let params = self.params();
        let name = utils::param_string(params, "name");
        if name.is_empty() {
            return Err(tube::Error::from("请提供名称".to_string()));
        }

        let source_type = utils::param_string(params, "type");
        if source_type.is_empty() {
            return Err(tube::Error::from("请提供类型".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
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

        let result = execute(
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
        .map_err(|e| tube::Error::from(format!("创建情报源失败: {}", e)))?;

        Ok(value!({"id": result as i64, "name": name}))
    }

    async fn source_update(&self) -> Result<Value> {
        let params = self.params();
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(tube::Error::from("请提供ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
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

        execute(&sql, p, &connector)
            .map_err(|e| tube::Error::from(format!("更新情报源失败: {}", e)))?;

        Ok(value!({"id": id}))
    }

    async fn source_delete(&self) -> Result<Value> {
        let params = self.params();
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(tube::Error::from("请提供ID".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "UPDATE intelligence_sources SET enabled = 0, modify_time = NOW() WHERE id = :id";
        execute(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| tube::Error::from(format!("删除情报源失败: {}", e)))?;

        Ok(value!({"id": id}))
    }

    async fn source_test(&self) -> Result<Value> {
        let params = self.params();
        let url = utils::param_string(params, "url");
        if url.is_empty() {
            return Err(tube::Error::from("请提供URL".to_string()));
        }

        if !is_safe_url(&url) {
            return Err(tube::Error::from("不安全的URL: 不允许访问内部地址".to_string()));
        }

        match self.client.get(&url).send().await {
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

    async fn source_fetch(&self) -> Result<Value> {
        let params = self.params();
        let source_id = params
            .get("sourceId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if source_id == 0 {
            return Err(tube::Error::from("请提供sourceId".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, name, source_type, url_template, config_json, market \
             FROM intelligence_sources WHERE id = :id AND enabled = 1";
        let rows = query_rows(
            sql,
            vec![("id".to_string(), Value::from(source_id))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询情报源失败: {}", e)))?;

        if rows.is_empty() {
            return Err(tube::Error::from(format!(
                "情报源不存在或未启用: {}",
                source_id
            )));
        }

        let row = &rows[0];
        let source_type = row_get_string(row, "sourceType");
        let url_template = row_get_string(row, "urlTemplate");
        let market = row_get_string(row, "market");

        let items = match source_type.as_str() {
            "rss" | "atom" => self.fetch_rss(source_id, &url_template, &market).await?,
            "api" => self.fetch_api(source_id, &url_template, &market).await?,
            _ => vec![],
        };

        Self::update_source_fetch_status(source_id, "success", items.len() as i64);

        Ok(value!({
            "sourceId": source_id,
            "fetchedCount": items.len() as i64,
        }))
    }

    async fn fetch_enabled(&self) -> Result<Value> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, name, source_type, url_template, config_json, market \
             FROM intelligence_sources WHERE enabled = 1";
        let rows = query_rows(sql, vec![], &connector)
            .map_err(|e| tube::Error::from(format!("查询情报源失败: {}", e)))?;

        let mut total_fetched = 0i64;
        for row in &rows {
            let source_id: i64 = row_get_f64(row, "id") as i64;
            let source_type = row_get_string(row, "sourceType");
            let url_template = row_get_string(row, "urlTemplate");
            let market = row_get_string(row, "market");

            let items = match source_type.as_str() {
                "rss" | "atom" => self.fetch_rss(source_id, &url_template, &market).await?,
                "api" => self.fetch_api(source_id, &url_template, &market).await?,
                _ => vec![],
            };
            let count = items.len() as i64;
            Self::update_source_fetch_status(source_id, "success", count);
            total_fetched += count;
        }

        Ok(value!({
            "sourcesProcessed": rows.len() as i64,
            "totalFetched": total_fetched,
        }))
    }

    async fn items(&self) -> Result<Value> {
        let params = self.params();
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
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

        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| tube::Error::from(format!("查询情报条目失败: {}", e)))?;

        Ok(Value::Array(rows))
    }

    async fn templates(&self) -> Result<Value> {
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

    async fn defaults(&self) -> Result<Value> {
        let templates = self.templates().await?;
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

            match self.source_create_with(&create_params).await {
                Ok(_) => created += 1,
                Err(_) => {}
            }
        }

        Ok(value!({
            "created": created, "total": items.len() as i64
        }))
    }

    async fn source_create_with(&self, params: &Value) -> Result<Value> {
        let name = utils::param_string(params, "name");
        if name.is_empty() {
            return Err(tube::Error::from("请提供名称".to_string()));
        }

        let source_type = utils::param_string(params, "type");
        if source_type.is_empty() {
            return Err(tube::Error::from("请提供类型".to_string()));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
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

        let result = execute(
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
        .map_err(|e| tube::Error::from(format!("创建情报源失败: {}", e)))?;

        Ok(value!({"id": result as i64, "name": name}))
    }

    async fn fetch_rss(&self, source_id: i64, url: &str, market: &str) -> Result<Vec<Value>> {
        if !is_safe_url(url) {
            return Err(tube::Error::from("不安全的URL: 不允许访问内部地址".to_string()));
        }

        let response = self.client.get(url).send().await.map_err(|e| {
            tube::Error::from(format!("RSS抓取失败: {}", e))
        })?;

        let body = response
            .text()
            .await
            .map_err(|e| tube::Error::from(format!("RSS读取失败: {}", e)))?;

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let mut items = Vec::new();

        for item_str in Self::extract_xml_items(&body) {
            let title = Self::extract_xml_tag(&item_str, "title");
            let link = Self::extract_xml_tag(&item_str, "link");
            let description = Self::extract_xml_tag(&item_str, "description");

            if title.is_empty() {
                continue;
            }

            let check_sql = "SELECT id FROM intelligence_items WHERE source_url = :url LIMIT 1";
            let existing = query_rows(
                check_sql,
                vec![("url".to_string(), Value::from(link.as_str()))],
                &connector,
            )
            .map_err(|e| tube::Error::from(format!("去重检查失败: {}", e)))?;

            if !existing.is_empty() {
                continue;
            }

            let scope_value = Self::extract_stock_codes(&title, &description);
            let scope_type = if scope_value.is_empty() { "all" } else { "stock" };
            let insert_sql = "INSERT INTO intelligence_items \
                 (source_id, title, content, source_url, scope_type, scope_value, market, \
                  published_at, fetched_at, create_time) \
                 VALUES (:sid, :title, :content, :url, :scope_type, :scope_value, :market, NOW(), NOW(), NOW())";
            if let Err(e) = execute(
                insert_sql,
                vec![
                    ("sid".to_string(), Value::from(source_id)),
                    ("title".to_string(), Value::from(title.as_str())),
                    ("content".to_string(), Value::from(description.as_str())),
                    ("url".to_string(), Value::from(link.as_str())),
                    ("scope_type".to_string(), Value::from(scope_type)),
                    ("scope_value".to_string(), Value::from(scope_value)),
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

    async fn fetch_api(&self, source_id: i64, url: &str, market: &str) -> Result<Vec<Value>> {
        if !is_safe_url(url) {
            return Err(tube::Error::from("不安全的URL: 不允许访问内部地址".to_string()));
        }

        let response = self.client.get(url).send().await.map_err(|e| {
            tube::Error::from(format!("API抓取失败: {}", e))
        })?;

        let body = response
            .text()
            .await
            .map_err(|e| tube::Error::from(format!("API读取失败: {}", e)))?;

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let mut items = Vec::new();

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

                let check_sql = "SELECT id FROM intelligence_items WHERE source_url = :url LIMIT 1";
                let existing = query_rows(
                    check_sql,
                    vec![("url".to_string(), Value::from(link.to_string()))],
                    &connector,
                )
                .map_err(|e| tube::Error::from(format!("去重检查失败: {}", e)))?;

                if !existing.is_empty() {
                    continue;
                }

                let insert_sql = "INSERT INTO intelligence_items \
                     (source_id, title, content, source_url, scope_type, scope_value, market, \
                      published_at, fetched_at, create_time) \
                     VALUES (:sid, :title, :content, :url, 'all', '', :market, NOW(), NOW(), NOW())";
                if let Err(e) = execute(
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

    fn update_source_fetch_status(source_id: i64, status: &str, _fetched_count: i64) {
        let connector = match utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => return,
        };
        let sql = "UPDATE intelligence_sources SET last_fetched_at = NOW(), last_status = :status, \
              modify_time = NOW() WHERE id = :id";
        let _ = execute(sql, vec![
            ("status".to_string(), Value::from(status.to_string())),
            ("id".to_string(), Value::from(source_id)),
        ], &connector);
    }

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

    fn extract_stock_codes(title: &str, content: &str) -> String {
        let text = format!("{} {}", title, content);
        let mut codes: Vec<String> = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        for i in 0..chars.len().saturating_sub(5) {
            let slice: String = chars[i..i + 6].iter().collect();
            if slice.chars().all(|c| c.is_ascii_digit()) {
                let first = slice.chars().next().unwrap_or('0');
                if first == '6' || first == '0' || first == '3' || first == '8' || first == '4' {
                    if !codes.contains(&slice) {
                        codes.push(slice);
                    }
                    if codes.len() >= 5 { break; }
                }
            }
        }
        codes.join(",")
    }
}
