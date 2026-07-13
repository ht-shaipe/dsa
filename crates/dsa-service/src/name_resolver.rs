use dsa_core::db::{query_rows, row_get_f64, row_get_string};
use dsa_core::utils;
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct NameResolver {
    request: RequestParameter,
}

impl NameResolver {
    pub fn new(param: &RequestParameter) -> Self {
        NameResolver { request: param.clone() }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "resolve" => self.resolve().await,
            "search" => self.search().await,
            _ => Err(tube::Error::from(format!("name_resolver unsupported method: {}", method))),
        }
    }

    fn params(&self) -> &Value { &self.request.value }

    async fn resolve(&self) -> Result<Value> {
        let params = self.params();
        let query = utils::param_string(params, "query");
        if query.is_empty() {
            return Err(tube::Error::from("Please provide query"));
        }

        if query.len() == 6 && query.chars().all(|c| c.is_ascii_digit()) {
            let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
            let sql = "SELECT stock_code, stock_name, close \
                 FROM stock_daily \
                 WHERE stock_code = :code AND status = 1 \
                 ORDER BY trade_date DESC LIMIT 1";
            let rows = query_rows(
                sql,
                vec![("code".to_string(), Value::from(query.as_str()))],
                &connector,
            )
            .map_err(|e| tube::Error::from(format!("Query stock by code failed: {}", e)))?;

            if !rows.is_empty() {
                let r = &rows[0];
                let code = row_get_string(r, "stockCode");
                let name = row_get_string(r, "stockName");
                let price: f64 = row_get_f64(r, "close");
                return Ok(value!({
                    "code": code,
                    "name": name,
                    "price": price,
                    "type": "exact_code",
                }));
            }

            return Ok(value!({
                "code": query, "type": "code_no_data"
            }));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT DISTINCT stock_code, stock_name, close \
             FROM stock_daily \
             WHERE stock_name LIKE :kw AND status = 1 \
             ORDER BY trade_date DESC LIMIT 10";
        let rows = query_rows(
            sql,
            vec![("kw".to_string(), Value::from(format!("%{}%", query)))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("Search stock by name failed: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| {
            let code = row_get_string(r, "stockCode");
            let name = row_get_string(r, "stockName");
            let price: f64 = row_get_f64(r, "close");
            value!({"code": code, "name": name, "price": price})
        }).collect();

        Ok(value!({
            "matches": results,
            "type": "name_search",
        }))
    }

    async fn search(&self) -> Result<Value> {
        let params = self.params();
        let keyword = utils::param_string(params, "keyword");
        if keyword.is_empty() {
            return Err(tube::Error::from("Please provide keyword"));
        }

        let limit = utils::param_i64(params, "limit").max(1).min(50) as i64;
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;

        let sql = "SELECT DISTINCT stock_code, stock_name, close \
             FROM stock_daily \
             WHERE (stock_code LIKE :kw OR stock_name LIKE :kw2) AND status = 1 \
             ORDER BY trade_date DESC LIMIT :limit";
        let rows = query_rows(
            sql,
            vec![
                ("kw".to_string(), Value::from(format!("%{}%", keyword))),
                ("kw2".to_string(), Value::from(format!("%{}%", keyword))),
                ("limit".to_string(), Value::from(limit)),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("Fuzzy search failed: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| {
            let code = row_get_string(r, "stockCode");
            let name = row_get_string(r, "stockName");
            let price: f64 = row_get_f64(r, "close");
            value!({"code": code, "name": name, "price": price})
        }).collect();

        Ok(Value::Array(results))
    }
}
