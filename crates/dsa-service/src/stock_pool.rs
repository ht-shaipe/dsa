use deck::sqlite::{DataTable, SelectExecutor};
use deck::QueryExecutor;
use deck::TableService;
use dsa_core::models::db::StockPool as StockPoolModel;
use dsa_core::utils;
use qta_crawler::EastMoney;
use tube::{Result, Value};
use tube_net::AsyncClient;
use tube_web::RequestParameter;

use crate::system::{broadcast_task_status, wait_if_paused, DATA_SYNC_STATUS};

pub struct StockPool {
    request: RequestParameter,
}

impl DataTable<StockPoolModel> for StockPool {
    fn datasource_key(&self) -> String {
        crate::DATASOURCE_KEY.to_owned()
    }
}

impl TableService<StockPoolModel> for StockPool {
    fn value(&self) -> Value {
        self.request.value.clone()
    }
    fn authorizer(&self) -> ((i8, u64, u64), (i8, u64), (i8, u64)) {
        self.request.get_auth_user()
    }
}

impl StockPool {
    pub fn new(param: &RequestParameter) -> Self {
        StockPool {
            request: param.clone(),
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "list" => self.list().await,
            "add" => self.add().await,
            "remove" => self.remove().await,
            "batch_remove" => self.batch_remove().await,
            "init_pool" => self.init_pool().await,
            "count" => self.count().await,
            _ => Err(error!("stock_pool不支持方法: {}", method)),
        }
    }

    async fn list(&self) -> Result<Value> {
        let params = self.value();
        let search = utils::param_string(&params, "search");
        let page = utils::param_i64(&params, "page").max(1) as u32;
        let page_size = utils::param_i64(&params, "page_size").max(1).min(200) as u32;
        // status: 不传或 < 0 时不过滤， >= 0 时按精确值过滤
        // param_i64 不存在时返回 0，用 has_status 区分
        let has_status = params.get("status").is_some();
        let status_filter = utils::param_i64(&params, "status");

        let connector = dsa_core::db::get_db_connector()
            .map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let mut where_clauses = vec!["1=1".to_string()];
        let mut sql_params: Vec<(String, Value)> = vec![];

        if !search.is_empty() {
            where_clauses.push("(stock_code LIKE :search OR stock_name LIKE :search)".to_string());
            let like_val = Value::from(format!("%{}%", search));
            sql_params.push(("search".to_string(), like_val));
        }

        if has_status && status_filter >= 0 {
            where_clauses.push("status = :status".to_string());
            sql_params.push(("status".to_string(), Value::from(status_filter)));
        }

        let where_sql = where_clauses.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) as cnt FROM stock_pool WHERE {}", where_sql);
        let count_rows = dsa_core::db::query_rows(&count_sql, sql_params.clone(), &connector)
            .map_err(|e| tube::Error::from(format!("查询股票池总数失败: {}", e)))?;
        let total = dsa_core::db::first_row_i64(&count_rows, "cnt");

        let offset = (page - 1) * page_size;
        let list_sql = format!(
            "SELECT * FROM stock_pool WHERE {} ORDER BY market_id DESC, stock_code ASC LIMIT :limit OFFSET :offset",
            where_sql
        );
        sql_params.push(("limit".to_string(), Value::from(page_size as i64)));
        sql_params.push(("offset".to_string(), Value::from(offset as i64)));

        let rows = dsa_core::db::query_rows(&list_sql, sql_params, &connector)
            .map_err(|e| tube::Error::from(format!("查询股票池列表失败: {}", e)))?;

        Ok(value!({
            "list": rows,
            "total": total,
            "page": page,
            "page_size": page_size,
        }))
    }

    async fn add(&self) -> Result<Value> {
        let params = self.value();
        let code = utils::param_string(&params, "code");
        if code.is_empty() {
            return Err(error!("请提供股票代码"));
        }

        if let Some(_) = self.find_by_code(&code)? {
            return Err(error!("股票 {} 已在股票池中", code));
        }

        let name = utils::param_string(&params, "name");
        let market_id = if code.starts_with('6') { 1 } else { 0 };
        let industry = utils::param_string(&params, "industry");

        let data = value!({
            "stock_code": code.clone(),
            "stock_name": name,
            "market": "cn",
            "market_id": market_id,
            "industry": industry,
            "status": 1,
        });
        let result = self.insert().data(&data).execute()?;

        Ok(value!({"id": result, "stockCode": code}))
    }

    async fn remove(&self) -> Result<Value> {
        let params = self.value();
        let id = utils::param_i64(&params, "id");

        if id > 0 {
            self.delete().r#where(conds![{ "id" = id }]).execute()?;
            return Ok(value!({"id": id}));
        }

        let code = utils::param_string(&params, "stock_code");
        if !code.is_empty() {
            self.delete().r#where(conds![{ "stock_code" = code.clone() }]).execute()?;
            return Ok(value!({"stockCode": code}));
        }

        Err(error!("请提供id或stock_code"))
    }

    async fn batch_remove(&self) -> Result<Value> {
        let params = self.value();
        let ids = match params.get("ids") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_i64())
                .collect::<Vec<i64>>(),
            _ => vec![],
        };

        if ids.is_empty() {
            return Err(error!("请提供要删除的ID列表"));
        }

        let connector = dsa_core::db::get_db_connector()
            .map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;

        let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!(":id{}", i)).collect();
        let sql = format!("DELETE FROM stock_pool WHERE id IN ({})", placeholders.join(", "));
        let sql_params: Vec<(String, Value)> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (format!("id{}", i), Value::from(id)))
            .collect();

        dsa_core::db::execute(&sql, sql_params, &connector)
            .map_err(|e| tube::Error::from(format!("批量删除失败: {}", e)))?;

        Ok(value!({"deleted": ids.len() as i64}))
    }

    async fn init_pool(&self) -> Result<Value> {
        {
            let st = DATA_SYNC_STATUS.lock().unwrap();
            if st.running {
                return Ok(
                    value!({"message": "已有任务在运行中，请等待完成后再试", "progress": st.done, "total": st.total}),
                );
            }
        }

        let params = self.value();
        let boards = self.parse_boards(&params);
        let exclude_st = self.parse_bool_param(&params, "exclude_st", true);
        let exclude_delisting = self.parse_bool_param(&params, "exclude_delisting", true);
        let exclude_new = self.parse_bool_param(&params, "exclude_new", true);

        {
            let mut st = DATA_SYNC_STATUS.lock().unwrap();
            st.running = true;
            st.paused = false;
            st.total = 0;
            st.done = 0;
            st.failed = 0;
            st.phase = "preparing".to_string();
            st.task_name = "init_stock_pool".to_string();
            st.current_code = String::new();
            st.current_name = String::new();
        }
        broadcast_task_status();

        std::thread::spawn(move || {
            log::info!("股票池初始化后台线程启动");
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime for init_stock_pool");

                rt.block_on(async {
                    log::info!("股票池初始化: 开始获取A股列表");

                    let mut spot_codes: Vec<(String, String, i8)> = Vec::new();

                    // 主数据源：新浪财经（分页拉取全量 A 股）
                    match Self::fetch_stock_list_simple().await {
                        Ok(items) if !items.is_empty() => {
                            log::info!("新浪API获取到 {} 条A股", items.len());
                            spot_codes = items;
                        }
                        Ok(_) => {
                            log::warn!("新浪API返回空列表");
                        }
                        Err(e) => {
                            log::warn!("新浪API失败: {}", e);
                        }
                    }

                    // 如果主数据源失败，尝试东方财富爬虫作为 fallback
                    if spot_codes.is_empty() {
                        log::info!("尝试东方财富爬虫作为 fallback");
                        let em = EastMoney::new();
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(180),
                            em.stock_zh_a_spot(),
                        ).await {
                            Ok(Ok(full_list)) if !full_list.is_empty() => {
                                log::info!("东方财富爬虫获取到 {} 条", full_list.len());
                                spot_codes = full_list.iter().filter_map(|s| {
                                    let code: String = s.get("代码").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    let name: String = s.get("名称").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                    if code.is_empty() { return None; }
                                    let market_id: i8 = if code.starts_with('6') { 1 } else { 0 };
                                    Some((code, name, market_id))
                                }).collect();
                            }
                            Ok(Ok(_)) => {
                                log::error!("东方财富爬虫也返回空");
                            }
                            Ok(Err(e2)) => {
                                log::error!("东方财富爬虫失败: {}", e2);
                            }
                            Err(_) => {
                                log::error!("东方财富爬虫超时");
                            }
                        }
                    }

                    if spot_codes.is_empty() {
                        log::warn!("股票池初始化: 行情数据为空，可能网络问题");
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.phase = "done".to_string();
                        broadcast_task_status();
                        return;
                    }

                    if !wait_if_paused() {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.phase = "done".to_string();
                        broadcast_task_status();
                        return;
                    }

                    let filtered: Vec<(String, String, i8)> = spot_codes
                        .iter()
                        .filter(|(code, name, _)| Self::should_include(code, name, &boards, exclude_st, exclude_delisting, exclude_new))
                        .cloned()
                        .collect();

                    let total = filtered.len() as u32;
                    log::info!("股票池初始化: 过滤后 {} 只股票，开始写入", total);
                    {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.total = total;
                        st.phase = "writing".to_string();
                    }
                    broadcast_task_status();

                    let conf = dsa_core::get_global_config();
                    let is_sqlite = conf.database.is_sqlite();
                    let connector = match dsa_core::db::get_db_connector() {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("股票池初始化DB连接失败: {}", e);
                            let mut st = DATA_SYNC_STATUS.lock().unwrap();
                            st.running = false;
                            st.phase = "done".to_string();
                            broadcast_task_status();
                            return;
                        }
                    };

                    let mut inserted: u64 = 0;
                    let mut updated: u64 = 0;
                    let batch_size = 100;
                    let mut batch_idx = 0;

                    for chunk in filtered.chunks(batch_size) {
                        if !wait_if_paused() { break; }

                        batch_idx += 1;
                        for (code, name, market_id) in chunk {
                            {
                                let mut st = DATA_SYNC_STATUS.lock().unwrap();
                                st.current_code = code.clone();
                                st.current_name = name.clone();
                            }

                            if is_sqlite {
                                let sql = "INSERT OR IGNORE INTO stock_pool (stock_code, stock_name, market, market_id, industry, status) \
                                           VALUES (:code, :name, 'cn', :market_id, '', 1)";
                                let sql_params = vec![
                                    ("code".to_string(), Value::from(code.clone())),
                                    ("name".to_string(), Value::from(name.clone())),
                                    ("market_id".to_string(), Value::from(*market_id as i64)),
                                ];
                                let affected = dsa_core::db::execute(sql, sql_params, &connector).unwrap_or(0);
                                if affected > 0 {
                                    inserted += affected;
                                } else {
                                    let upd = "UPDATE stock_pool SET stock_name = :name, market_id = :market_id, status = 1 \
                                               WHERE stock_code = :code AND (stock_name != :name OR status != 1)";
                                    let upd_params = vec![
                                        ("code".to_string(), Value::from(code.clone())),
                                        ("name".to_string(), Value::from(name.clone())),
                                        ("market_id".to_string(), Value::from(*market_id as i64)),
                                    ];
                                    let upd_affected = dsa_core::db::execute(upd, upd_params, &connector).unwrap_or(0);
                                    updated += upd_affected;
                                }
                            } else {
                                let sql = "INSERT INTO stock_pool (stock_code, stock_name, market, market_id, industry, status) \
                                           VALUES (:code, :name, 'cn', :market_id, '', 1) \
                                           ON DUPLICATE KEY UPDATE stock_name = VALUES(stock_name), market_id = VALUES(market_id), status = 1";
                                let sql_params = vec![
                                    ("code".to_string(), Value::from(code.clone())),
                                    ("name".to_string(), Value::from(name.clone())),
                                    ("market_id".to_string(), Value::from(*market_id as i64)),
                                ];
                                let affected = dsa_core::db::execute(sql, sql_params, &connector).unwrap_or(0);
                                if affected > 1 {
                                    updated += 1;
                                } else if affected == 1 {
                                    inserted += 1;
                                }
                            }

                            {
                                let mut st = DATA_SYNC_STATUS.lock().unwrap();
                                st.done += 1;
                            }
                        }

                        broadcast_task_status();
                        if batch_idx % 10 == 0 {
                            log::debug!("股票池初始化进度: 批次{} 已写入{}条", batch_idx, inserted + updated);
                        }
                    }

                    {
                        let mut st = DATA_SYNC_STATUS.lock().unwrap();
                        st.running = false;
                        st.phase = "done".to_string();
                        st.current_code = String::new();
                        st.current_name = String::new();
                    }
                    broadcast_task_status();

                    log::info!(
                        "股票池初始化完成: 新增 {} 更新 {} 总计 {}",
                        inserted, updated, inserted + updated
                    );
                });
            }));

            if let Err(e) = result {
                log::error!("股票池初始化线程panic: {:?}", e);
                let mut st = DATA_SYNC_STATUS.lock().unwrap();
                st.running = false;
                st.phase = "done".to_string();
                broadcast_task_status();
            }
        });

        Ok(value!({
            "message": "股票池初始化已启动",
        }))
    }

    async fn count(&self) -> Result<Value> {
        let connector = dsa_core::db::get_db_connector()
            .map_err(|e| tube::Error::from(format!("DB连接失败: {}", e)))?;
        let total = self.pool_count_internal(&connector)?;
        Ok(value!({"total": total}))
    }

    fn pool_count_internal(&self, connector: &deck_connector::Connector) -> Result<i64> {
        let sql = "SELECT COUNT(*) as cnt FROM stock_pool WHERE status = 1";
        let rows = dsa_core::db::query_rows(sql, vec![], connector)
            .map_err(|e| tube::Error::from(format!("查询股票池数量失败: {}", e)))?;
        Ok(dsa_core::db::first_row_i64(&rows, "cnt"))
    }

    fn find_by_code(&self, code: &str) -> Result<Option<Value>> {
        let res = self
            .select()
            .r#where(conds![{ "stock_code" = code }])
            .one()?;
        Ok(if res.is_null() { None } else { Some(res) })
    }

    fn parse_boards(&self, params: &Value) -> Vec<String> {
        match params.get("boards") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => {
                let conf = dsa_core::get_global_config();
                conf.data_sync.boards.clone()
            }
        }
    }

    fn parse_bool_param(&self, params: &Value, key: &str, default: bool) -> bool {
        match params.get(key) {
            Some(Value::Bool(b)) => *b,
            Some(v) if v.is_number() => v.as_f64().unwrap_or(0.0) != 0.0,
            _ => default,
        }
    }

    fn should_include(
        code: &str,
        name: &str,
        boards: &[String],
        exclude_st: bool,
        exclude_delisting: bool,
        exclude_new: bool,
    ) -> bool {
        let board_match = boards.iter().any(|b| match b.as_str() {
            "sh_main" => code.starts_with('6') && !code.starts_with("68"),
            "sh_kj" => code.starts_with("68"),
            "sz_main" => code.starts_with('0') && !code.starts_with("03"),
            "sz_gem" => code.starts_with("30"),
            "bj_main" => code.starts_with('8') || code.starts_with("4") || code.starts_with("920"),
            _ => false,
        });

        if !board_match {
            return false;
        }

        if exclude_st {
            let name_upper = name.to_uppercase();
            if name_upper.contains("ST") || name_upper.contains("*ST") || name_upper.contains("退") {
                return false;
            }
        }

        if exclude_delisting {
            let name_upper = name.to_uppercase();
            if name_upper.contains("退市") || name_upper.contains("退") {
                return false;
            }
        }

        if exclude_new {
        }

        true
    }

    /// 从新浪财经获取 A 股全部股票列表（分页）
    /// 新浪每页最多 100 条，需分页拉取
    async fn fetch_stock_list_simple() -> Result<Vec<(String, String, i8)>> {
        let mut headers = std::collections::HashMap::new();
        headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36".to_string());
        headers.insert("Referer".to_string(), "https://finance.sina.com.cn/".to_string());

        let client = AsyncClient::new("").add_headers(headers).timeout(15000);

        // 先获取总数
        let count_url = "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeStockCount?node=hs_a";
        let count_resp = client.get(count_url).await
            .map_err(|e| tube::Error::from(format!("获取股票总数失败: {}", e)))?;
        let total: usize = count_resp.trim_matches('"').parse().unwrap_or(0);
        if total == 0 {
            return Err(tube::Error::from("新浪API返回股票总数为0"));
        }
        log::info!("新浪API报告A股总数: {}", total);

        // 分页拉取，每页100条
        let pages = (total + 99) / 100;
        let mut all_items = Vec::with_capacity(total);

        for page in 1..=pages {
            let url = format!(
                "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData?page={}&num=100&sort=symbol&asc=1&node=hs_a&_s_r_a=auto",
                page
            );

            let resp = match client.get(&url).await {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("新浪API第{}页请求失败: {}, 跳过", page, e);
                    continue;
                }
            };

            let arr = match Value::from_str(&resp) {
                Ok(v) if v.is_array() => v,
                _ => {
                    log::warn!("新浪API第{}页解析失败, 跳过", page);
                    continue;
                }
            };

            for item in arr.as_array().unwrap() {
                let code = item.get("code").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                if code.is_empty() { continue; }
                let market_id: i8 = if code.starts_with('6') { 1 } else { 0 };
                all_items.push((code, name, market_id));
            }
        }

        log::info!("新浪API共获取到 {} 条股票记录 (期望 {})", all_items.len(), total);
        Ok(all_items)
    }
}
