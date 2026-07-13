//! Unified database helper — dispatches to MySQL or SQLite based on config

use deck_connector::Connector;
use deck_connector::DatabaseType;
use tube::{Result, Value};

pub fn execute(sql: &str, param: Vec<(String, Value)>, connector: &Connector) -> Result<u64> {
    match connector.db_type {
        DatabaseType::Sqlite => {
            #[cfg(feature = "sqlite")]
            {
                deck_sqlite::Helper::execute(sql, param, connector)
            }
            #[cfg(not(feature = "sqlite"))]
            {
                let _ = sql;
                let _ = param;
                let _ = connector;
                Err(tube::error!("SQLite not available"))
            }
        }
        _ => {
            #[cfg(feature = "mysql")]
            {
                deck_mysql::Helper::execute(sql, param, connector)
            }
            #[cfg(not(feature = "mysql"))]
            {
                let _ = sql;
                let _ = param;
                let _ = connector;
                Err(tube::error!("MySQL not available"))
            }
        }
    }
}

pub fn query_rows(sql: &str, param: Vec<(String, Value)>, connector: &Connector) -> Result<Vec<Value>> {
    match connector.db_type {
        DatabaseType::Sqlite => {
            #[cfg(feature = "sqlite")]
            {
                deck_sqlite::Helper::query(
                    sql,
                    param,
                    |row, attrs| deck_sqlite::DataRow::to_value(row, attrs),
                    connector,
                    &None,
                )
            }
            #[cfg(not(feature = "sqlite"))]
            {
                let _ = sql;
                let _ = param;
                let _ = connector;
                Err(tube::error!("SQLite not available"))
            }
        }
        _ => {
            #[cfg(feature = "mysql")]
            {
                let rows = deck_mysql::Helper::query_rows(sql, param, connector)?;
                Ok(rows.iter().map(|r| deck_mysql::DataRow::to_value2(r)).collect())
            }
            #[cfg(not(feature = "mysql"))]
            {
                let _ = sql;
                let _ = param;
                let _ = connector;
                Err(tube::error!("MySQL not available"))
            }
        }
    }
}

pub fn get_db_connector() -> std::result::Result<Connector, crate::DsaError> {
    let conf = crate::get_global_config();
    let db_type = &conf.database.db_type;
    deck_connector::get_connector("default", db_type)
        .ok_or_else(|| crate::DsaError::Database(format!("{}连接未初始化", db_type)))
}

pub fn row_get_string(row: &Value, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}

pub fn row_get_f64(row: &Value, key: &str) -> f64 {
    row.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

pub fn row_get_i64(row: &Value, key: &str) -> i64 {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

pub fn row_get_value(row: &Value, key: &str) -> Value {
    row.get(key).cloned().unwrap_or(Value::Null)
}

pub fn first_row_value(rows: &[Value]) -> Value {
    rows.first().cloned().unwrap_or(Value::Null)
}

pub fn first_row_string(rows: &[Value], key: &str) -> String {
    rows.first()
        .and_then(|r| r.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

pub fn first_row_i64(rows: &[Value], key: &str) -> i64 {
    rows.first()
        .and_then(|r| r.get(key))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}
