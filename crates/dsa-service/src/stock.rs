use deck::sqlite::{DataTable, SelectExecutor};
use deck::QueryExecutor;
use deck::TableService;
use dsa_core::models::db::WatchlistStock as WatchlistStockModel;
use dsa_core::utils;

use qta_crawler::{Basic, EastMoney, History, Real, Stock as QtaStock, QQ};
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct Stock {
    request: RequestParameter,
}

impl DataTable<WatchlistStockModel> for Stock {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}

impl TableService<WatchlistStockModel> for Stock {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}

impl Stock {
    pub fn new(param: &RequestParameter) -> Self {
        Stock {
            request: param.clone(),
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "search" => self.search().await,
            "quote" => self.get_quote().await,
            "quotes" => self.get_quotes().await,
            "kline" => self.get_kline().await,
            "history" => self.get_history().await,
            "info" => self.get_info().await,
            "watchlist" => self.get_watchlist().await,
            "watchlist_add" => self.watchlist_add().await,
            "watchlist_remove" => self.watchlist_remove().await,
            "watchlist_update" => self.watchlist_update().await,
            "watchlist_sync" => self.watchlist_sync().await,
            "spot" => self.get_spot().await,
            "industries" => self.get_industries().await,
            "concepts" => self.get_concepts().await,
            _ => Err(error!("stock不支持方法: {}", method)),
        }
    }

    async fn search(&self) -> Result<Value> {
        let params = self.value();
        let query = utils::param_string(&params, "query");

        if query.is_empty() {
            return Ok(value!([]));
        }

        let code = query.replace("sh", "").replace("sz", "");
        if code.len() == 6 && code.chars().all(|c| c.is_numeric()) {
            let prefix = utils::market_prefix(&code);
            let result = Real::new()
                .get_price(&format!("{}{}", prefix, code))
                .await
                .map_err(|e| tube::Error::from(format!("搜索行情失败: {}", e)))?;
            Ok(value!([result]))
        } else {
            self.fuzzy_search(&query).await
        }
    }

    async fn fuzzy_search(&self, query: &str) -> Result<Value> {
        let url = format!(
            "https://suggest3.sinajs.cn/suggest/type=11,12,15,21,22,23,24,25,26&key={}",
            urlencoding::encode(query)
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        let resp = client
            .get(&url)
            .header("Referer", "https://finance.sina.com.cn")
            .send()
            .await;

        match resp {
            Ok(response) => {
                let body = response.text().await.unwrap_or_default();
                let results = Self::parse_sina_suggest(&body);
                Ok(Value::Array(results))
            }
            Err(e) => {
                tracing::warn!("[股票搜索] 新浪搜索请求失败: {}", e);
                Ok(value!([]))
            }
        }
    }

    fn parse_sina_suggest(body: &str) -> Vec<Value> {
        let json_str = if body.starts_with("var suggest_value=\"") && body.ends_with("\";") {
            &body[20..body.len() - 2]
        } else {
            body
        };

        if json_str.is_empty() {
            return vec![];
        }

        let mut results = Vec::new();
        for piece in json_str.split(';') {
            let parts: Vec<&str> = piece.split(',').collect();
            if parts.len() < 10 {
                continue;
            }

            let active = parts.get(8).map(|s| *s).unwrap_or("0");
            if active != "1" {
                continue;
            }

            let type_id = parts.get(1).map(|s| *s).unwrap_or("");
            let is_stock = matches!(type_id, "11" | "12" | "15");
            let is_fund = matches!(type_id, "21" | "22" | "23" | "24" | "25" | "26");

            let raw_code = parts.get(3).map(|s| *s).unwrap_or_default();
            let name = parts.get(4).map(|s| *s).unwrap_or_default();

            if raw_code.is_empty() || name.is_empty() {
                continue;
            }

            let (market, code) = if raw_code.starts_with("sh") || raw_code.starts_with("sz") {
                (&raw_code[..2], &raw_code[2..])
            } else if raw_code.starts_with("bj") {
                ("bj", &raw_code[2..])
            } else {
                ("", raw_code)
            };

            let stock_type = if is_stock {
                "stock"
            } else if is_fund {
                "fund"
            } else {
                "other"
            };

            results.push(value!({
                "code": code,
                "name": name,
                "market": market,
                "type": stock_type,
                "symbol": raw_code,
            }));

            if results.len() >= 15 {
                break;
            }
        }

        results
    }

    async fn get_quote(&self) -> Result<Value> {
        let code = Self::param_code(&self.value());
        let pure_code = code
            .trim_start_matches("sh")
            .trim_start_matches("sz")
            .trim_start_matches("bj");
        let prefix = utils::market_prefix(pure_code);
        let symbol = format!("{}{}", prefix, pure_code);

        let qq = QQ::new();
        let quote = qq
            .get_realtime_quote(&symbol)
            .await
            .map_err(|e| tube::Error::from(format!("获取行情失败: {}", e)))?;

        Ok(quote)
    }

    async fn get_quotes(&self) -> Result<Value> {
        let params = self.value();
        let codes_val = utils::param_string(&params, "codes");
        if codes_val.is_empty() {
            return Ok(value!([]));
        }

        let real = Real::new();
        let mut results = Vec::new();
        for code in codes_val.split(',') {
            let code = code.trim();
            let pure_code = code
                .trim_start_matches("sh")
                .trim_start_matches("sz")
                .trim_start_matches("bj");
            if pure_code.len() >= 6 {
                let prefix = utils::market_prefix(pure_code);
                if let Ok(v) = real.get_price(&format!("{}{}", prefix, pure_code)).await {
                    results.push(v);
                }
            }
        }
        Ok(Value::Array(results))
    }

    async fn get_kline(&self) -> Result<Value> {
        let params = self.value();
        let code = Self::param_code(&params);
        let period = utils::param_string(&params, "period");
        let period_str = if period.is_empty() { "daily" } else { &period };

        let pure_code = code
            .trim_start_matches("sh")
            .trim_start_matches("sz")
            .trim_start_matches("bj");
        let prefix = utils::market_prefix(pure_code);
        let prefixed_code = format!("{}{}", prefix, pure_code);

        let scale: u32 = match period_str {
            "weekly" => 1200,
            "monthly" => 5200,
            _ => 240,
        };
        let datalen: u32 = 500;

        if let Ok(raw) = History::get_price(&prefixed_code, scale, "5,10,20,30,60", datalen).await {
            if let Some(arr) = raw.as_array() {
                if !arr.is_empty() {
                    let results: Vec<Value> = arr.iter().map(|item| {
                        value!({
                            "date": item.get("day").and_then(|v| v.as_str()).unwrap_or_default(),
                            "open": item.get("open").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "close": item.get("close").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "high": item.get("high").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "low": item.get("low").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "volume": item.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
                            "amount": item.get("ma_price5").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        })
                    }).collect();
                    return Ok(Value::Array(results));
                }
            }
        }

        let em = EastMoney::new();
        for i in 0..2 {
            match em
                .stock_zh_a_hist(pure_code, Some(period_str), None, None, Some("qfq"))
                .await
            {
                Ok(raw) => {
                    let results: Vec<Value> = raw.iter().map(|item| {
                        value!({
                            "date": item.get("日期").and_then(|v| v.as_str()).unwrap_or_default(),
                            "open": item.get("开盘").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "close": item.get("收盘").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "high": item.get("最高").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "low": item.get("最低").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "volume": item.get("成交量").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
                            "amount": item.get("成交额").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        })
                    }).collect();
                    if !results.is_empty() {
                        return Ok(Value::Array(results));
                    }
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500 * (i as u64 + 1))).await;
                }
            }
        }

        Err(error!("获取K线失败: 所有数据源均不可用"))
    }

    async fn get_history(&self) -> Result<Value> {
        let params = self.value();
        let code = Self::param_code(&params);
        let days = params.get("days").and_then(|v| v.as_f64()).unwrap_or(120.0) as i64;

        let result = History::get_163_hist(&code, days).await;
        Ok(result)
    }

    async fn get_info(&self) -> Result<Value> {
        let code = Self::param_code(&self.value());
        let detail = QtaStock::get_detail(&code)
            .await
            .map_err(|e| tube::Error::from(format!("获取股票信息失败: {}", e)))?;
        Ok(detail)
    }

    async fn get_watchlist(&self) -> Result<Value> {
        let db_results = self.query_enabled_list()?;

        if !db_results.is_empty() {
            let codes: Vec<String> = db_results
                .iter()
                .filter_map(|r| {
                    r.get("stockCode")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();

            let real = Real::new();
            let mut quote_map = std::collections::HashMap::new();
            for code in &codes {
                let prefix = utils::market_prefix(code);
                if let Ok(v) = real.get_price(&format!("{}{}", prefix, code)).await {
                    quote_map.insert(code.clone(), v);
                }
            }

            let merged: Vec<Value> = db_results
                .iter()
                .map(|db_item| {
                    let code = db_item
                        .get("stockCode")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    let prefixed = format!("{}{}", utils::market_prefix(code.as_str()), code);
                    if let Some(quote) = quote_map
                        .get(prefixed.as_str())
                        .or_else(|| quote_map.get(code.as_str()))
                    {
                        let mut merged = db_item.clone();
                        if let Value::Object(ref mut map) = merged {
                            if let Value::Object(ref quote_map_inner) = quote {
                                for k in quote_map_inner.keys() {
                                    if !map.contains_key(k.as_str()) {
                                        map.insert(
                                            k.clone(),
                                            quote_map_inner
                                                .get(k.as_str())
                                                .cloned()
                                                .unwrap_or(Value::Null),
                                        );
                                    }
                                }
                            }
                        }
                        merged
                    } else {
                        db_item.clone()
                    }
                })
                .collect();

            return Ok(Value::Array(merged));
        }

        let has_any_rows = self.count_all()?;
        if has_any_rows > 0 {
            return Ok(value!([]));
        }

        let conf = dsa_core::get_global_config();
        let watchlist = &conf.stock.watchlist;
        if watchlist.is_empty() {
            return Ok(value!([]));
        }

        let real = Real::new();
        let mut results = Vec::new();
        for (idx, code) in watchlist.iter().enumerate() {
            let prefix = utils::market_prefix(code);
            let full = format!("{}{}", prefix, code);
            let mut item = value!({
                "id": 0,
                "stockCode": code,
                "stockName": "",
                "groupName": "default",
                "sortOrder": idx as i64 + 1,
                "market": "",
                "remark": "",
                "enabled": 1,
            });
            if let Ok(v) = real.get_price(&full).await {
                if let Value::Object(ref mut map) = item {
                    if let Value::Object(ref quote) = v {
                        for k in quote.keys() {
                            if !map.contains_key(k.as_str()) {
                                map.insert(
                                    k.clone(),
                                    quote.get(k.as_str()).cloned().unwrap_or(Value::Null),
                                );
                            }
                        }
                    }
                }
            }
            results.push(item);
        }
        Ok(Value::Array(results))
    }

    async fn watchlist_add(&self) -> Result<Value> {
        let params = self.value();
        let code = utils::param_string(&params, "code");
        if code.is_empty() {
            return Err(error!("请提供股票代码"));
        }

        if let Some(_) = self.find_by_code(&code)? {
            return Err(error!("股票 {} 已在自选列表中", code));
        }

        let name = utils::param_string(&params, "name");
        let group = utils::param_string(&params, "group");
        let group_val = if group.is_empty() { "default" } else { &group };
        let sort_order = utils::param_i64(&params, "sort_order") as i32;

        let result = self.add_stock(&code, &name, group_val, sort_order)?;

        Ok(value!({"id": result, "stockCode": code}))
    }

    async fn watchlist_remove(&self) -> Result<Value> {
        let params = self.value();
        let id = utils::param_i64(&params, "id");

        if id > 0 {
            self.soft_remove(id)?;
            return Ok(value!({"id": id}));
        }

        let code = utils::param_string(&params, "stock_code");
        if !code.is_empty() {
            if let Some(_) = self.find_existing_by_code(&code)? {
                self.soft_remove_by_code(&code)?;
            } else {
                self.hard_remove_by_code(&code)?;
            }
            return Ok(value!({"stockCode": code}));
        }

        Err(error!("请提供id或stock_code"))
    }

    async fn watchlist_update(&self) -> Result<Value> {
        let params = self.value();
        let id = utils::param_i64(&params, "id");
        if id == 0 {
            return Err(error!("请提供ID"));
        }

        let mut data = value!({});
        if let Value::Object(ref mut map) = data {
            let name = utils::param_string(&params, "name");
            if !name.is_empty() {
                map.insert("stock_name".to_string(), Value::from(name));
            }
            let group = utils::param_string(&params, "group");
            if !group.is_empty() {
                map.insert("group_name".to_string(), Value::from(group));
            }
            let remark = utils::param_string(&params, "remark");
            if !remark.is_empty() {
                map.insert("remark".to_string(), Value::from(remark));
            }
            if let Some(so) = params
                .get("sort_order")
                .and_then(|v| v.as_f64())
                .map(|v| v as i32)
            {
                map.insert("sort_order".to_string(), Value::from(so));
            }
        }

        if data.is_object() && data.as_object().map_or(true, |m| m.is_empty()) {
            return Ok(value!({"message": "无更新内容"}));
        }

        self.update_fields(id, &data)?;
        Ok(value!({"id": id}))
    }

    async fn watchlist_sync(&self) -> Result<Value> {
        let params = self.value();
        let stocks = match params.get("stocks") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => vec![],
        };

        self.disable_all()?;

        let mut count = 0i64;
        for item in &stocks {
            let code: String = item
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if code.is_empty() {
                continue;
            }
            let name: String = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if let Some(_) = self.find_existing_by_code(&code)? {
                self.re_enable_with_name(&code, &name, count as i32)?;
            } else {
                self.add_stock(&code, &name, "default", count as i32)?;
            }
            count += 1;
        }

        Ok(value!({"count": count}))
    }

    async fn get_spot(&self) -> Result<Value> {
        let em = EastMoney::new();
        let data = em
            .stock_zh_a_spot()
            .await
            .map_err(|e| tube::Error::from(format!("获取全市场行情失败: {}", e)))?;
        Ok(Value::Array(data))
    }

    async fn get_industries(&self) -> Result<Value> {
        let basic = Basic::new();
        let data = basic
            .get_industry()
            .await
            .map_err(|e| tube::Error::from(format!("获取行业列表失败: {}", e)))?;
        Ok(data.into())
    }

    async fn get_concepts(&self) -> Result<Value> {
        let basic = Basic::new();
        let data = basic
            .get_concept()
            .await
            .map_err(|e| tube::Error::from(format!("获取概念列表失败: {}", e)))?;
        Ok(data.into())
    }

    fn param_code(params: &Value) -> String {
        utils::param_string(params, "code")
    }

    fn query_enabled_list(&self) -> Result<Vec<Value>> {
        self.select()
            .r#where(conds![{ "enabled" = 1 }])
            .order(ord!("sort_order ASC, id ASC"))
            .query_values()
    }

    fn count_all(&self) -> Result<i64> {
        let connector = self
            .get_connector()
            .ok_or_else(|| error!("数据库连接未初始化"))?;
        let sql = "SELECT COUNT(*) as cnt FROM watchlist_stocks";
        let rows = dsa_core::db::query_rows(sql, vec![], &connector)
            .map_err(|e| tube::Error::from(format!("查询watchlist总数失败: {}", e)))?;
        Ok(dsa_core::db::first_row_i64(&rows, "cnt"))
    }

    fn find_by_code(&self, code: &str) -> Result<Option<Value>> {
        let res = self
            .select()
            .r#where(conds![{ "stock_code" = code }, { "enabled" = 1 }])
            .one()?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    fn add_stock(&self, code: &str, name: &str, group: &str, sort: i32) -> Result<Value> {
        let data = value!({
            "stock_code": code,
            "stock_name": name,
            "market": "cn",
            "group_name": group,
            "sort_order": sort,
            "enabled": 1,
            "remark": "",
        });
        self.insert().data(&data).execute()
    }

    fn soft_remove(&self, id: i64) -> Result<Value> {
        self.update()
            .data(&value!({ "enabled": 0, "modify_time": chrono::Local::now().naive_local() }))
            .r#where(conds![{ "id" = id }])
            .execute()
    }

    fn hard_remove_by_code(&self, code: &str) -> Result<Value> {
        self.delete()
            .r#where(conds![{ "stock_code" = code }])
            .execute()
    }

    fn soft_remove_by_code(&self, code: &str) -> Result<Value> {
        self.update()
            .data(&value!({ "enabled": 0, "modify_time": chrono::Local::now().naive_local() }))
            .r#where(conds![{ "stock_code" = code }])
            .execute()
    }

    fn update_fields(&self, id: i64, data: &Value) -> Result<Value> {
        let mut d = data.clone();
        d["id"] = value!(id);
        d["modify_time"] = value!(chrono::Local::now().naive_local());
        self.update()
            .data(&d)
            .r#where(conds![{ "id" = id }])
            .execute()
    }

    fn disable_all(&self) -> Result<Value> {
        self.update()
            .data(&value!({ "enabled": 0, "modify_time": chrono::Local::now().naive_local() }))
            .r#where(conds![{ "enabled" = 1 }])
            .execute()
    }

    fn find_existing_by_code(&self, code: &str) -> Result<Option<Value>> {
        let res = self
            .select()
            .r#where(conds![{ "stock_code" = code }])
            .one()?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    fn re_enable_with_name(&self, code: &str, name: &str, sort: i32) -> Result<Value> {
        self.update()
            .data(&value!({
                "stock_name": name,
                "enabled": 1,
                "sort_order": sort,
                "modify_time": chrono::Local::now().naive_local()
            }))
            .r#where(conds![{ "stock_code" = code }])
            .execute()
    }
}
