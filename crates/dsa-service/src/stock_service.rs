//! 股票数据服务 - 整合 qta_crawler 多源数据

use dsa_core::models::db::WatchlistStock as WatchlistStockModel;
use dsa_core::{DsaError, DsaResult, utils};
use deck::{DataTable, QueryExecutor, SelectExecutor, TableService,
    Condition, SortDirect, OrderExpr};
use qta_crawler::{Basic, EastMoney, History, QQ, Real, Stock};
use tube::Value;

const DATASOURCE_KEY: &str = "default";

fn eq_cond(field: &str, val: impl Into<Value>) -> Condition {
    Condition::new(field, val.into())
}

fn eq_cond_not_param(field: &str, val: impl Into<Value>) -> Condition {
    Condition::new(field, val.into()).not_param()
}

fn order_by(field: &str, direct: &str) -> Vec<OrderExpr> {
    vec![OrderExpr::new(field, SortDirect::get_instance(direct))]
}

struct WatchlistTable {
    data: Value,
}

impl DataTable<WatchlistStockModel> for WatchlistTable {
    fn datasource_key(&self) -> String {
        DATASOURCE_KEY.to_owned()
    }
}

impl TableService<WatchlistStockModel> for WatchlistTable {
    fn value(&self) -> Value {
        self.data.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        ((0, 0, 0), (0, 0), (0, 0))
    }
}

impl WatchlistTable {
    fn new(data: &Value) -> Self {
        WatchlistTable { data: data.clone() }
    }

    fn query_enabled_list(&self) -> Result<Vec<Value>, DsaError> {
        self.select()
            .r#where(vec![eq_cond("enabled", 1)])
            .order(order_by("sortOrder", "asc"))
            .order_str("sortOrder asc, id asc")
            .query_values()
            .map_err(|e| DsaError::Database(format!("查询自选股失败: {}", e)))
    }

    fn find_by_code(&self, code: &str) -> Result<Option<Value>, DsaError> {
        let res = self.select()
            .r#where(vec![eq_cond("stockCode", code), eq_cond("enabled", 1)])
            .one()
            .map_err(|e| DsaError::Database(format!("查重失败: {}", e)))?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    fn add_stock(&self, code: &str, name: &str, group: &str, sort: i32) -> Result<Value, DsaError> {
        let data = value!({
            "stockCode": code,
            "stockName": name,
            "market": "cn",
            "groupName": group,
            "sortOrder": sort,
            "enabled": 1,
            "remark": "",
        });
        self.insert().data(&data).execute()
            .map_err(|e| DsaError::Database(format!("添加自选股失败: {}", e)))
    }

    fn soft_remove(&self, id: i64) -> Result<Value, DsaError> {
        self.update()
            .data(&value!({ "enabled": 0, "modifyTime": chrono::Local::now().naive_local() }))
            .r#where(vec![eq_cond("id", id)])
            .execute()
            .map_err(|e| DsaError::Database(format!("删除自选股失败: {}", e)))
    }

    fn hard_remove_by_code(&self, code: &str) -> Result<Value, DsaError> {
        self.delete()
            .r#where(vec![eq_cond("stockCode", code)])
            .execute()
            .map_err(|e| DsaError::Database(format!("删除自选股失败: {}", e)))
    }

    fn update_fields(&self, id: i64, data: &Value) -> Result<Value, DsaError> {
        let mut d = data.clone();
        d["id"] = value!(id);
        d["modifyTime"] = value!(chrono::Local::now().naive_local());
        self.update()
            .data(&d)
            .r#where(vec![eq_cond("id", id)])
            .execute()
            .map_err(|e| DsaError::Database(format!("更新自选股失败: {}", e)))
    }

    fn disable_all(&self) -> Result<Value, DsaError> {
        self.update()
            .data(&value!({ "enabled": 0, "modifyTime": chrono::Local::now().naive_local() }))
            .r#where(vec![eq_cond("enabled", 1)])
            .execute()
            .map_err(|e| DsaError::Database(format!("同步自选股-禁用失败: {}", e)))
    }

    fn find_existing_by_code(&self, code: &str) -> Result<Option<Value>, DsaError> {
        let res = self.select()
            .r#where(vec![eq_cond("stockCode", code)])
            .one()
            .map_err(|e| DsaError::Database(format!("同步自选股-查重失败: {}", e)))?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    fn re_enable_with_name(&self, code: &str, name: &str, sort: i32) -> Result<Value, DsaError> {
        self.update()
            .data(&value!({
                "stockName": name,
                "enabled": 1,
                "sortOrder": sort,
                "modifyTime": chrono::Local::now().naive_local()
            }))
            .r#where(vec![eq_cond("stockCode", code)])
            .execute()
            .map_err(|e| DsaError::Database(format!("同步自选股-更新失败: {}", e)))
    }
}

/// 股票数据服务
pub struct StockService {}

impl StockService {
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "search" => self.search(params).await,
            "quote" => self.get_quote(params).await,
            "quotes" => self.get_quotes(params).await,
            "kline" => self.get_kline(params).await,
            "history" => self.get_history(params).await,
            "info" => self.get_info(params).await,
            "watchlist" => self.get_watchlist(params).await,
            "watchlist_add" => self.watchlist_add(params).await,
            "watchlist_remove" => self.watchlist_remove(params).await,
            "watchlist_update" => self.watchlist_update(params).await,
            "watchlist_sync" => self.watchlist_sync(params).await,
            "spot" => self.get_spot(params).await,
            "industries" => self.get_industries().await,
            "concepts" => self.get_concepts().await,
            _ => Err(DsaError::ApiRouting(format!("stock不支持方法: {}", method))),
        }
    }

    async fn search(&self, params: &Value) -> DsaResult<Value> {
        let query = utils::param_string(params, "query");

        if query.is_empty() {
            return Ok(value!([]));
        }

        let code = query.replace("sh", "").replace("sz", "");
        if code.len() == 6 && code.chars().all(|c| c.is_numeric()) {
            let prefix = utils::market_prefix(&code);
            let result = Real::new()
                .get_price(&format!("{}{}", prefix, code))
                .await
                .map_err(|e| DsaError::StockData(format!("搜索行情失败: {}", e)))?;
            Ok(value!([result]))
        } else {
            self.fuzzy_search(&query).await
        }
    }

    async fn fuzzy_search(&self, query: &str) -> DsaResult<Value> {
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

            let stock_type = if is_stock { "stock" } else if is_fund { "fund" } else { "other" };

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

    async fn get_quote(&self, params: &Value) -> DsaResult<Value> {
        let code = Self::param_code(params);
        let prefix = utils::market_prefix(&code);
        let symbol = format!("{}{}", prefix, code);

        let qq = QQ::new();
        let quote = qq
            .get_realtime_quote(&symbol)
            .await
            .map_err(|e| DsaError::StockData(format!("获取行情失败: {}", e)))?;

        Ok(quote)
    }

    async fn get_quotes(&self, params: &Value) -> DsaResult<Value> {
        let codes_val = utils::param_string(params, "codes");
        if codes_val.is_empty() {
            return Ok(value!([]));
        }

        let real = Real::new();
        let mut results = Vec::new();
        for code in codes_val.split(',') {
            let code = code.trim();
            if code.len() >= 6 {
                let prefix = utils::market_prefix(code);
                if let Ok(v) = real.get_price(&format!("{}{}", prefix, code)).await {
                    results.push(v);
                }
            }
        }
        Ok(Value::Array(results))
    }

    async fn get_kline(&self, params: &Value) -> DsaResult<Value> {
        let code = Self::param_code(params);
        let period = utils::param_string(params, "period");
        let period_str = if period.is_empty() { "daily" } else { &period };
        let start_date = utils::param_string(params, "startDate");
        let end_date = utils::param_string(params, "endDate");
        let adjust = utils::param_string(params, "adjust");
        let adjust_str = if adjust.is_empty() { "qfq" } else { &adjust };

        let em = EastMoney::new();
        let data = em
            .stock_zh_a_hist(
                &code,
                Some(period_str),
                if start_date.is_empty() {
                    None
                } else {
                    Some(start_date.as_str())
                },
                if end_date.is_empty() {
                    None
                } else {
                    Some(end_date.as_str())
                },
                Some(adjust_str),
            )
            .await
            .map_err(|e| DsaError::StockData(format!("获取K线失败: {}", e)))?;

        Ok(Value::Array(data))
    }

    async fn get_history(&self, params: &Value) -> DsaResult<Value> {
        let code = Self::param_code(params);
        let days = params.get("days").and_then(|v| v.as_f64()).unwrap_or(120.0) as i64;

        let result = History::get_163_hist(&code, days).await;
        Ok(result)
    }

    async fn get_info(&self, params: &Value) -> DsaResult<Value> {
        let code = Self::param_code(params);
        let detail = Stock::get_detail(&code)
            .await
            .map_err(|e| DsaError::StockData(format!("获取股票信息失败: {}", e)))?;
        Ok(detail)
    }

    async fn get_watchlist(&self, _params: &Value) -> DsaResult<Value> {
        let table = WatchlistTable::new(&value!({}));
        let db_results = table.query_enabled_list()?;

        if !db_results.is_empty() {
            let codes: Vec<String> = db_results
                .iter()
                .filter_map(|r| r.get("stockCode").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .collect();

            let real = Real::new();
            let mut quote_map = std::collections::HashMap::new();
            for code in &codes {
                let prefix = utils::market_prefix(code);
                if let Ok(v) = real.get_price(&format!("{}{}", prefix, code)).await {
                    if let Some(c) = v.get("code").and_then(|c| c.as_str()) {
                        quote_map.insert(c.to_string(), v);
                    }
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
                    if let Some(quote) = quote_map.get(prefixed.as_str()).or_else(|| quote_map.get(code.as_str())) {
                        let mut merged = db_item.clone();
                        if let Value::Object(ref mut map) = merged {
                            if let Value::Object(ref quote_map_inner) = quote {
                                for k in quote_map_inner.keys() {
                                    if !map.contains_key(k.as_str()) {
                                        map.insert(k.clone(), quote_map_inner.get(k.as_str()).cloned().unwrap_or(Value::Null));
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
                                map.insert(k.clone(), quote.get(k.as_str()).cloned().unwrap_or(Value::Null));
                            }
                        }
                    }
                }
            }
            results.push(item);
        }
        Ok(Value::Array(results))
    }

    async fn watchlist_add(&self, params: &Value) -> DsaResult<Value> {
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(DsaError::Validation("请提供股票代码".to_string()));
        }

        let table = WatchlistTable::new(params);

        if let Some(_) = table.find_by_code(&code)? {
            return Err(DsaError::Validation(format!(
                "股票 {} 已在自选列表中",
                code
            )));
        }

        let name = utils::param_string(params, "name");
        let group = utils::param_string(params, "group");
        let group_val = if group.is_empty() { "default" } else { &group };
        let sort_order = utils::param_i64(params, "sortOrder") as i32;

        let result = table.add_stock(&code, &name, group_val, sort_order)?;

        Ok(value!({"id": result, "stockCode": code}))
    }

    async fn watchlist_remove(&self, params: &Value) -> DsaResult<Value> {
        let table = WatchlistTable::new(params);
        let id = utils::param_i64(params, "id");

        if id > 0 {
            table.soft_remove(id)?;
            return Ok(value!({"id": id}));
        }

        let code = utils::param_string(params, "stockCode");
        if !code.is_empty() {
            table.hard_remove_by_code(&code)?;
            return Ok(value!({"stockCode": code}));
        }

        Err(DsaError::Validation("请提供id或stockCode".to_string()))
    }

    async fn watchlist_update(&self, params: &Value) -> DsaResult<Value> {
        let id = utils::param_i64(params, "id");
        if id == 0 {
            return Err(DsaError::Validation("请提供ID".to_string()));
        }

        let table = WatchlistTable::new(params);
        let mut data = value!({});
        if let Value::Object(ref mut map) = data {
            let name = utils::param_string(params, "name");
            if !name.is_empty() {
                map.insert("stockName".to_string(), Value::from(name));
            }
            let group = utils::param_string(params, "group");
            if !group.is_empty() {
                map.insert("groupName".to_string(), Value::from(group));
            }
            let remark = utils::param_string(params, "remark");
            if !remark.is_empty() {
                map.insert("remark".to_string(), Value::from(remark));
            }
            if let Some(so) = params.get("sortOrder").and_then(|v| v.as_f64()).map(|v| v as i32) {
                map.insert("sortOrder".to_string(), Value::from(so));
            }
        }

        if data.is_object() && data.as_object().map_or(true, |m| m.is_empty()) {
            return Ok(value!({"message": "无更新内容"}));
        }

        table.update_fields(id, &data)?;
        Ok(value!({"id": id}))
    }

    async fn watchlist_sync(&self, params: &Value) -> DsaResult<Value> {
        let stocks = match params.get("stocks") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => vec![],
        };

        let table = WatchlistTable::new(params);
        table.disable_all()?;

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

            if let Some(_) = table.find_existing_by_code(&code)? {
                table.re_enable_with_name(&code, &name, count as i32)?;
            } else {
                table.add_stock(&code, &name, "default", count as i32)?;
            }
            count += 1;
        }

        Ok(value!({"count": count}))
    }

    async fn get_spot(&self, _params: &Value) -> DsaResult<Value> {
        let em = EastMoney::new();
        let data = em
            .stock_zh_a_spot()
            .await
            .map_err(|e| DsaError::StockData(format!("获取全市场行情失败: {}", e)))?;
        Ok(Value::Array(data))
    }

    async fn get_industries(&self) -> DsaResult<Value> {
        let basic = Basic::new();
        let data = basic
            .get_industry()
            .await
            .map_err(|e| DsaError::StockData(format!("获取行业列表失败: {}", e)))?;
        Ok(data.into())
    }

    async fn get_concepts(&self) -> DsaResult<Value> {
        let basic = Basic::new();
        let data = basic
            .get_concept()
            .await
            .map_err(|e| DsaError::StockData(format!("获取概念列表失败: {}", e)))?;
        Ok(data.into())
    }

    fn param_code(params: &Value) -> String {
        utils::param_string(params, "code")
    }
}
