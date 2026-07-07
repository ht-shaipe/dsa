//! Name resolver service - stock name/code lookup and fuzzy search

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use tube::Value;

/// 名称解析服务
pub struct NameResolverService;

impl NameResolverService {
    /// 创建名称解析服务实例
    pub fn new() -> Self {
        Self
    }

    /// 请求分发 - 可用方法: resolve, search
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "resolve" => self.resolve(params).await,
            "search" => self.search(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "name_resolver unsupported method: {}",
                method
            ))),
        }
    }

    /// Resolve name to code or vice versa
    async fn resolve(&self, params: &Value) -> DsaResult<Value> {
        let query = utils::param_string(params, "query");
        if query.is_empty() {
            return Err(DsaError::Validation("Please provide query".to_string()));
        }

        // If it looks like a 6-digit stock code, return directly
        if query.len() == 6 && query.chars().all(|c| c.is_ascii_digit()) {
            let connector = utils::get_db_connector()?;
            let sql = "SELECT stockCode, stockName, close \
                 FROM stock_daily \
                 WHERE stockCode = :code AND status = 1 \
                 ORDER BY tradeDate DESC LIMIT 1";
            let rows = Helper::query_rows(
                sql,
                vec![("code".to_string(), Value::from(query.as_str()))],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("Query stock by code failed: {}", e)))?;

            if !rows.is_empty() {
                let r = &rows[0];
                let code = r.get_string(0);
                let name = r.get_string(1);
                let price: f64 = r.get_value(2).as_f64().unwrap_or(0.0);
                return Ok(value!({
                    "status": "ok",
                    "data": {
                        "code": code,
                        "name": name,
                        "price": price,
                        "type": "exact_code",
                    }
                }));
            }

            return Ok(value!({
                "status": "ok",
                "data": {"code": query, "type": "code_no_data"}
            }));
        }

        // Otherwise search by name
        let connector = utils::get_db_connector()?;
        let sql = "SELECT DISTINCT stockCode, stockName, close \
             FROM stock_daily \
             WHERE stockName LIKE :kw AND status = 1 \
             ORDER BY tradeDate DESC LIMIT 10";
        let rows = Helper::query_rows(
            sql,
            vec![("kw".to_string(), Value::from(format!("%{}%", query)))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("Search stock by name failed: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| {
            let code = r.get_string(0);
            let name = r.get_string(1);
            let price: f64 = r.get_value(2).as_f64().unwrap_or(0.0);
            value!({"code": code, "name": name, "price": price})
        }).collect();

        Ok(value!({
            "status": "ok",
            "data": {
                "matches": results,
                "type": "name_search",
            }
        }))
    }

    /// Fuzzy search stocks by keyword
    async fn search(&self, params: &Value) -> DsaResult<Value> {
        let keyword = utils::param_string(params, "keyword");
        if keyword.is_empty() {
            return Err(DsaError::Validation("Please provide keyword".to_string()));
        }

        let limit = utils::param_i64(params, "limit").max(1).min(50) as i64;
        let connector = utils::get_db_connector()?;

        // Search by code or name
        let sql = "SELECT DISTINCT stockCode, stockName, close \
             FROM stock_daily \
             WHERE (stockCode LIKE :kw OR stockName LIKE :kw2) AND status = 1 \
             ORDER BY tradeDate DESC LIMIT :limit";
        let rows = Helper::query_rows(
            sql,
            vec![
                ("kw".to_string(), Value::from(format!("%{}%", keyword))),
                ("kw2".to_string(), Value::from(format!("%{}%", keyword))),
                ("limit".to_string(), Value::from(limit)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("Fuzzy search failed: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| {
            let code = r.get_string(0);
            let name = r.get_string(1);
            let price: f64 = r.get_value(2).as_f64().unwrap_or(0.0);
            value!({"code": code, "name": name, "price": price})
        }).collect();

        Ok(value!({"status": "ok", "data": results}))
    }
}
