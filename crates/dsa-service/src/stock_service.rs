//! 股票数据服务 - 整合 qta_crawler 多源数据

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use qta_crawler::{Basic, EastMoney, History, QQ, Real, Stock};
use tube::Value;

/// 股票数据服务
pub struct StockService {}

impl StockService {
    /// 创建股票服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: search, quote, quotes, kline, history, info, watchlist, watchlist_add, watchlist_remove, watchlist_update, watchlist_sync, spot, industries, concepts
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
        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, stock_code, stock_name, market, group_name, sort_order, enabled, remark, create_time, modify_time \
             FROM watchlist_stocks WHERE enabled = 1 ORDER BY sort_order, id";
        let rows = Helper::query_rows(sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询自选股失败: {}", e)))?;

        let db_results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();

        if !db_results.is_empty() {
            let codes: Vec<String> = db_results
                .iter()
                .filter_map(|r| r.get("stock_code").and_then(|v| v.as_str()).map(|s| s.to_string()))
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
                        .get("stock_code")
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

        let connector = utils::get_db_connector()?;

        let check_sql = "SELECT id FROM watchlist_stocks WHERE stock_code = :code AND enabled = 1 LIMIT 1";
        let existing = Helper::query_rows(
            check_sql,
            vec![("code".to_string(), Value::from(code.as_str()))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查重失败: {}", e)))?;

        if !existing.is_empty() {
            return Err(DsaError::Validation(format!(
                "股票 {} 已在自选列表中",
                code
            )));
        }

        let name = utils::param_string(params, "name");
        let group = utils::param_string(params, "group");
        let group_val = if group.is_empty() { "default" } else { &group };
        let sort_order = params
            .get("sortOrder")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i32;

        let sql = "INSERT INTO watchlist_stocks \
             (stock_code, stock_name, market, group_name, sort_order, enabled, remark, create_time, modify_time) \
             VALUES (:code, :name, 'cn', :group, :sort, 1, '', NOW(), NOW())";

        let result = Helper::execute(
            sql,
            vec![
                ("code".to_string(), Value::from(code.as_str())),
                ("name".to_string(), Value::from(name.as_str())),
                ("group".to_string(), Value::from(group_val)),
                ("sort".to_string(), Value::from(sort_order)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("添加自选股失败: {}", e)))?;

        Ok(value!({"id": result as i64, "stockCode": code}))
    }

    async fn watchlist_remove(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        if id > 0 {
            let sql = "UPDATE watchlist_stocks SET enabled = 0, modify_time = NOW() WHERE id = :id";
            Helper::execute(
                sql,
                vec![("id".to_string(), Value::from(id))],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("删除自选股失败: {}", e)))?;
            return Ok(value!({"id": id}));
        }

        let code = utils::param_string(params, "stockCode");
        if !code.is_empty() {
            let sql = "DELETE FROM watchlist_stocks WHERE stock_code = :code";
            Helper::execute(
                sql,
                vec![("code".to_string(), Value::from(code.as_str()))],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("删除自选股失败: {}", e)))?;
            return Ok(value!({"stockCode": code}));
        }

        Err(DsaError::Validation("请提供id或stockCode".to_string()))
    }

    async fn watchlist_update(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let name = utils::param_string(params, "name");
        let group = utils::param_string(params, "group");
        let remark = utils::param_string(params, "remark");
        let sort_order = params
            .get("sortOrder")
            .and_then(|v| v.as_f64())
            .map(|v| v as i32);

        let mut sets = Vec::new();
        let mut p: Vec<(String, Value)> = Vec::new();
        p.push(("id".to_string(), Value::from(id)));

        if !name.is_empty() {
            sets.push("stockName = :name".to_string());
            p.push(("name".to_string(), Value::from(name.as_str())));
        }
        if !group.is_empty() {
            sets.push("groupName = :group".to_string());
            p.push(("group".to_string(), Value::from(group.as_str())));
        }
        if let Some(so) = sort_order {
            sets.push("sortOrder = :sort".to_string());
            p.push(("sort".to_string(), Value::from(so)));
        }
        if !remark.is_empty() {
            sets.push("remark = :remark".to_string());
            p.push(("remark".to_string(), Value::from(remark.as_str())));
        }

        if sets.is_empty() {
            return Ok(value!({"message": "无更新内容"}));
        }

        sets.push("modify_time = NOW()".to_string());
        let sql = format!(
            "UPDATE watchlist_stocks SET {} WHERE id = :id",
            sets.join(", ")
        );

        Helper::execute(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("更新自选股失败: {}", e)))?;

        Ok(value!({"id": id}))
    }

    async fn watchlist_sync(&self, params: &Value) -> DsaResult<Value> {
        let stocks = match params.get("stocks") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => vec![],
        };

        let connector = utils::get_db_connector()?;

        let disable_sql = "UPDATE watchlist_stocks SET enabled = 0, modify_time = NOW() WHERE enabled = 1";
        Helper::execute(disable_sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("同步自选股-禁用失败: {}", e)))?;

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

            let check_sql = "SELECT id FROM watchlist_stocks WHERE stock_code = :code LIMIT 1";
            let existing = Helper::query_rows(
                check_sql,
                vec![("code".to_string(), Value::from(code.as_str()))],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("同步自选股-查重失败: {}", e)))?;

            if !existing.is_empty() {
                let update_sql = "UPDATE watchlist_stocks SET stock_name = :name, enabled = 1, sort_order = :sort, modify_time = NOW() WHERE stock_code = :code";
                Helper::execute(
                    update_sql,
                    vec![
                        ("name".to_string(), Value::from(name.as_str())),
                        ("sort".to_string(), Value::from(count as i32)),
                        ("code".to_string(), Value::from(code.as_str())),
                    ],
                    &connector,
                )
                .map_err(|e| DsaError::Database(format!("同步自选股-更新失败: {}", e)))?;
            } else {
                let insert_sql = "INSERT INTO watchlist_stocks \
                     (stock_code, stock_name, market, group_name, sort_order, enabled, remark, create_time, modify_time) \
                     VALUES (:code, :name, 'cn', 'default', :sort, 1, '', NOW(), NOW())";
                Helper::execute(
                    insert_sql,
                    vec![
                        ("code".to_string(), Value::from(code.as_str())),
                        ("name".to_string(), Value::from(name.as_str())),
                        ("sort".to_string(), Value::from(count as i32)),
                    ],
                    &connector,
                )
                .map_err(|e| DsaError::Database(format!("同步自选股-插入失败: {}", e)))?;
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
