//! Database schema migration utility
//!
//! Supports both MySQL and SQLite via `Dialect` enum.
//! Uses DataModel trait metadata (Class, Attribute) to generate
//! `CREATE TABLE IF NOT EXISTS` SQL and tracks applied migrations
//! in a `schema_migrations` table.

use deck_connector::Connector;
use deck_connector::DatabaseType;
use deck_model::{Attribute, Class, DataModel};
use tracing::{info, warn};

use crate::models::db::*;

#[cfg(feature = "sqlite")]
use rusqlite::Connection;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dialect {
    Mysql,
    Sqlite,
}

impl Dialect {
    pub fn from_connector(connector: &Connector) -> Self {
        match connector.db_type {
            DatabaseType::Sqlite => Dialect::Sqlite,
            _ => Dialect::Mysql,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run all pending migrations. Creates the `schema_migrations` tracking table
/// first, then iterates every model and ensures its table exists.
/// Uses a fast-path check: if the recorded schema version matches the current
/// model set hash, all migrations are skipped without querying per-version.
pub fn run_migrations(connector: &Connector) {
    let dialect = Dialect::from_connector(connector);
    info!("run_migrations called, dialect={:?}", dialect);

    match dialect {
        Dialect::Mysql => run_migrations_mysql(connector),
        Dialect::Sqlite => run_migrations_sqlite(connector),
    }
}

/// Compute a version hash from the current model definitions and alter migrations.
/// This changes whenever models or alter migrations are added/modified.
fn compute_schema_hash() -> String {
    let models = collect_models();
    let mut parts: Vec<String> = Vec::new();
    for (label, cls) in &models {
        parts.push(format!("{}:{}", label, cls.table_name));
        for attr in &cls.attributes {
            parts.push(format!("{}:{}:{}", attr.name, attr.data_type, attr.alias));
        }
    }
    for (version, _sql) in collect_alter_migrations_sqlite() {
        parts.push(version.to_string());
    }
    for (version, _sql) in collect_alter_migrations_mysql() {
        parts.push(version.to_string());
    }
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    parts.join("|").hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ---------------------------------------------------------------------------
// MySQL migration path
// ---------------------------------------------------------------------------

#[cfg(feature = "mysql")]
fn run_migrations_mysql(connector: &Connector) {
    let current_hash = compute_schema_hash();
    if is_schema_current_mysql(connector, &current_hash) {
        info!(
            "schema is up-to-date (hash={}), skipping all migrations",
            &current_hash[..8]
        );
        return;
    }

    let migration_sql = create_table_sql(&SchemaMigration::class(), Dialect::Mysql);
    execute_ddl_mysql(connector, &migration_sql);

    let models: Vec<(&str, Class)> = collect_models();

    for (label, cls) in &models {
        if *label == "schema_migrations" {
            continue;
        }
        let version = format!("v0_{}_create", label);
        if is_migration_applied_mysql(connector, &version) {
            info!("migration `{}` already applied, skipping", version);
            continue;
        }
        let sql = create_table_sql(cls, Dialect::Mysql);
        info!("creating table `{}` …", cls.table_name);
        execute_ddl_mysql(connector, &sql);
        record_migration_mysql(
            connector,
            &version,
            &format!("create table {}", cls.table_name),
        );
    }

    info!("all mysql table migrations completed");

    run_column_migrations_mysql(connector);
    run_alter_migrations_mysql(connector);

    record_schema_hash_mysql(connector, &current_hash);
    info!("schema hash updated to {}", &current_hash[..8]);
}

#[cfg(not(feature = "mysql"))]
fn run_migrations_mysql(_connector: &Connector) {
    warn!("mysql feature not enabled, skipping mysql migrations");
}

// ---------------------------------------------------------------------------
// SQLite migration path
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
fn run_migrations_sqlite(connector: &Connector) {
    let current_hash = compute_schema_hash();
    if is_schema_current_sqlite(connector, &current_hash) {
        info!(
            "schema is up-to-date (hash={}), skipping all migrations",
            &current_hash[..8]
        );
        return;
    }

    let migration_sql = create_table_sql(&SchemaMigration::class(), Dialect::Sqlite);
    execute_ddl_sqlite(connector, &migration_sql);

    let models: Vec<(&str, Class)> = collect_models();

    for (label, cls) in &models {
        if *label == "schema_migrations" {
            continue;
        }
        let version = format!("v0_{}_create", label);
        if is_migration_applied_sqlite(connector, &version) {
            info!("migration `{}` already applied, skipping", version);
            continue;
        }
        let sql = create_table_sql(cls, Dialect::Sqlite);
        info!("creating table `{}` …", cls.table_name);
        execute_ddl_sqlite(connector, &sql);
        record_migration_sqlite(
            connector,
            &version,
            &format!("create table {}", cls.table_name),
        );
    }

    info!("all sqlite table migrations completed");

    run_alter_migrations_sqlite(connector);

    record_schema_hash_sqlite(connector, &current_hash);
    info!("schema hash updated to {}", &current_hash[..8]);
}

#[cfg(not(feature = "sqlite"))]
fn run_migrations_sqlite(_connector: &Connector) {
    warn!("sqlite feature not enabled, skipping sqlite migrations");
}

// ---------------------------------------------------------------------------
// Schema hash fast-path (shared)
// ---------------------------------------------------------------------------

/// Check if the stored schema hash matches the current computed hash.
/// Returns true if all migrations are already applied and schema is current.
#[cfg(feature = "sqlite")]
fn is_schema_current_sqlite(connector: &Connector, current_hash: &str) -> bool {
    let conn_str = connector.get_conn_str();
    match Connection::open(&conn_str) {
        Ok(conn) => {
            match conn.query_row(
                "SELECT \"version\" FROM \"schema_migrations\" WHERE \"version\" LIKE 'schema_hash:%' LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            ) {
                Ok(stored) => {
                    let stored_hash = stored.strip_prefix("schema_hash:").unwrap_or("");
                    stored_hash == current_hash
                }
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

#[cfg(feature = "sqlite")]
fn record_schema_hash_sqlite(connector: &Connector, hash: &str) {
    let conn_str = connector.get_conn_str();
    if let Ok(conn) = Connection::open(&conn_str) {
        let _ = conn.execute(
            "DELETE FROM \"schema_migrations\" WHERE \"version\" LIKE 'schema_hash:%'",
            [],
        );
        let version = format!("schema_hash:{}", hash);
        let _ = conn.execute(
            "INSERT INTO \"schema_migrations\" (\"version\", \"description\", \"applied_at\") VALUES (?1, 'schema version hash', datetime('now'))",
            [&version],
        );
    }
}

#[cfg(feature = "mysql")]
fn is_schema_current_mysql(connector: &Connector, current_hash: &str) -> bool {
    let sql =
        "SELECT `version` FROM `schema_migrations` WHERE `version` LIKE 'schema_hash:%' LIMIT 1";
    match deck_mysql::Helper::query_rows(sql, vec![], connector) {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                let stored = row.get::<String, _>("version").unwrap_or_default();
                let stored_hash = stored.strip_prefix("schema_hash:").unwrap_or("");
                return stored_hash == current_hash;
            }
            false
        }
        Err(_) => false,
    }
}

#[cfg(feature = "mysql")]
fn record_schema_hash_mysql(connector: &Connector, hash: &str) {
    let _ = deck_mysql::Helper::execute(
        "DELETE FROM `schema_migrations` WHERE `version` LIKE 'schema_hash:%'",
        vec![],
        connector,
    );
    let version = format!("schema_hash:{}", hash);
    let _ = deck_mysql::Helper::execute(
        "INSERT INTO `schema_migrations` (`version`, `description`, `applied_at`) VALUES (:version, 'schema version hash', NOW())",
        vec![("version".to_string(), tube::Value::from(version))],
        connector,
    );
}

// ---------------------------------------------------------------------------
// Model collection (shared)
// ---------------------------------------------------------------------------

fn collect_models() -> Vec<(&'static str, Class)> {
    vec![
        ("analysis_history", AnalysisHistory::class()),
        ("stock_daily", StockDaily::class()),
        ("decision_signals", DecisionSignal::class()),
        ("decision_signal_outcomes", DecisionSignalOutcome::class()),
        ("decision_signal_feedback", DecisionSignalFeedback::class()),
        ("news_intel", NewsIntel::class()),
        ("intelligence_sources", IntelligenceSource::class()),
        ("intelligence_items", IntelligenceItem::class()),
        ("fundamental_snapshot", FundamentalSnapshot::class()),
        ("backtest_results", BacktestResult::class()),
        ("backtest_summaries", BacktestSummary::class()),
        ("portfolio_accounts", PortfolioAccount::class()),
        ("portfolio_trades", PortfolioTrade::class()),
        ("portfolio_positions", PortfolioPosition::class()),
        ("portfolio_position_lots", PortfolioPositionLot::class()),
        ("portfolio_cash_ledger", PortfolioCashLedger::class()),
        (
            "portfolio_corporate_actions",
            PortfolioCorporateAction::class(),
        ),
        ("portfolio_daily_snapshots", PortfolioDailySnapshot::class()),
        ("portfolio_fx_rates", PortfolioFxRate::class()),
        ("alert_rules", AlertRule::class()),
        ("alert_triggers", AlertTrigger::class()),
        ("alert_notifications", AlertNotification::class()),
        ("alert_cooldowns", AlertCooldown::class()),
        ("watchlist_stocks", WatchlistStock::class()),
        ("stock_pool", StockPool::class()),
        ("stock_quote", StockQuote::class()),
        ("conversation_messages", ConversationMessage::class()),
        ("conversation_summaries", ConversationSummary::class()),
        ("agent_provider_turns", AgentProviderTurn::class()),
        ("llm_usage", LlmUsage::class()),
        ("schema_migrations", SchemaMigration::class()),
    ]
}

// ---------------------------------------------------------------------------
// SQL generation (dialect-aware)
// ---------------------------------------------------------------------------

/// Generate a `CREATE TABLE IF NOT EXISTS` statement from a `Class` metadata.
pub fn create_table_sql(cls: &Class, dialect: Dialect) -> String {
    let table_name = &cls.table_name;
    let mut cols: Vec<String> = Vec::new();
    let mut primary_keys: Vec<String> = Vec::new();
    let mut autoincrement_col: Option<String> = None;

    for attr in &cls.attributes {
        if attr.increment > 0
            && attr.primary
            && cls.primary_type == "identity"
            && dialect == Dialect::Sqlite
        {
            let name = column_name(attr);
            autoincrement_col = Some(name.clone());
            primary_keys.push(name.clone());
            cols.push(format!(
                "\"{}\" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT",
                name
            ));
            continue;
        }
        let col = build_column_def(attr, &cls.primary_type, dialect);
        cols.push(col);
        if attr.primary {
            primary_keys.push(column_name(attr));
        }
    }

    if !primary_keys.is_empty() {
        if autoincrement_col.is_none() {
            cols.push(format!("PRIMARY KEY ({})", primary_keys.join(", ")));
        }
    }

    match dialect {
        Dialect::Mysql => {
            format!(
                "CREATE TABLE IF NOT EXISTS `{}` (\n  {}\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
                table_name,
                cols.join(",\n  "),
            )
        }
        Dialect::Sqlite => {
            format!(
                "CREATE TABLE IF NOT EXISTS \"{}\" (\n  {}\n)",
                table_name,
                cols.join(",\n  "),
            )
        }
    }
}

fn build_column_def(attr: &Attribute, primary_type: &str, dialect: Dialect) -> String {
    let name = column_name(attr);
    let sql_type = attribute_sql_type(attr, primary_type, dialect);

    let q = |s: &str| match dialect {
        Dialect::Mysql => format!("`{}`", s),
        Dialect::Sqlite => format!("\"{}\"", s),
    };

    let mut parts = vec![format!("{} {}", q(&name), sql_type)];

    if attr.required || attr.primary {
        parts.push("NOT NULL".to_owned());
    }

    if attr.increment > 0 && attr.primary && primary_type == "identity" {
        match dialect {
            Dialect::Mysql => parts.push("AUTO_INCREMENT".to_owned()),
            Dialect::Sqlite => parts.push("AUTOINCREMENT".to_owned()),
        }
    }

    if !attr.default_value.is_empty() && !attr.primary {
        let dv = translate_default_value(&attr.default_value, dialect);
        parts.push(format!("DEFAULT {}", dv));
    }

    if attr.unique && !attr.primary {
        parts.push("UNIQUE".to_owned());
    }

    if !attr.comment.is_empty() && dialect == Dialect::Mysql {
        parts.push(format!("COMMENT '{}'", attr.comment.replace('\'', "\\'")));
    }

    parts.join(" ")
}

fn translate_default_value(val: &str, dialect: Dialect) -> String {
    match dialect {
        Dialect::Mysql => val.to_owned(),
        Dialect::Sqlite => {
            let lower = val.to_lowercase();
            if lower.starts_with("current_timestamp") {
                "CURRENT_TIMESTAMP".to_owned()
            } else {
                val.to_owned()
            }
        }
    }
}

fn column_name(attr: &Attribute) -> String {
    if !attr.alias.is_empty() {
        attr.alias.clone()
    } else {
        attr.name.clone()
    }
}

fn attribute_sql_type(attr: &Attribute, primary_type: &str, dialect: Dialect) -> String {
    if attr.primary {
        if primary_type == "uuid" {
            return match dialect {
                Dialect::Mysql => "VARCHAR(36)".to_owned(),
                Dialect::Sqlite => "TEXT".to_owned(),
            };
        }
        if primary_type == "identity" {
            return match dialect {
                Dialect::Mysql => "BIGINT".to_owned(),
                Dialect::Sqlite => "INTEGER".to_owned(),
            };
        }
    }

    if attr.data_length > 0 && attr.data_type.contains("String") {
        return match dialect {
            Dialect::Mysql => format!("VARCHAR({})", attr.data_length),
            Dialect::Sqlite => "TEXT".to_owned(),
        };
    }

    rust_type_to_sql(&attr.data_type, dialect)
}

/// Map a Rust type string to a SQL column type.
pub fn rust_type_to_sql(data_type: &str, dialect: Dialect) -> String {
    let base = if data_type.starts_with("Option<") && data_type.ends_with('>') {
        &data_type[7..data_type.len() - 1]
    } else {
        data_type
    };

    match dialect {
        Dialect::Mysql => rust_type_to_sql_mysql(base),
        Dialect::Sqlite => rust_type_to_sql_sqlite(base),
    }
}

fn rust_type_to_sql_mysql(base: &str) -> String {
    match base {
        "i64" => "BIGINT".to_owned(),
        "i32" => "INT".to_owned(),
        "i16" => "SMALLINT".to_owned(),
        "i8" => "TINYINT".to_owned(),
        "u64" => "BIGINT UNSIGNED".to_owned(),
        "u32" => "INT UNSIGNED".to_owned(),
        "u16" => "SMALLINT UNSIGNED".to_owned(),
        "u8" => "TINYINT UNSIGNED".to_owned(),
        "f64" => "DOUBLE".to_owned(),
        "f32" => "FLOAT".to_owned(),
        "bool" => "TINYINT(1)".to_owned(),
        "String" => "VARCHAR(255)".to_owned(),
        "NaiveDateTime" => "DATETIME".to_owned(),
        "Vec<u8>" => "BLOB".to_owned(),
        _ => {
            if base.contains("String") {
                "VARCHAR(255)".to_owned()
            } else if base.contains("NaiveDateTime") || base.contains("DateTime") {
                "DATETIME".to_owned()
            } else if base.contains("Vec<u8>") || base.contains("Bytes") {
                "BLOB".to_owned()
            } else {
                "TEXT".to_owned()
            }
        }
    }
}

fn rust_type_to_sql_sqlite(base: &str) -> String {
    match base {
        "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" => "INTEGER".to_owned(),
        "f64" | "f32" => "REAL".to_owned(),
        "bool" => "INTEGER".to_owned(),
        "String" => "TEXT".to_owned(),
        "NaiveDateTime" => "TEXT".to_owned(),
        "Vec<u8>" => "BLOB".to_owned(),
        _ => {
            if base.contains("String")
                || base.contains("NaiveDateTime")
                || base.contains("DateTime")
            {
                "TEXT".to_owned()
            } else if base.contains("Vec<u8>") || base.contains("Bytes") {
                "BLOB".to_owned()
            } else {
                "TEXT".to_owned()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// MySQL migration tracking
// ---------------------------------------------------------------------------

#[cfg(feature = "mysql")]
fn is_migration_applied_mysql(connector: &Connector, version: &str) -> bool {
    let sql = "SELECT COUNT(*) AS cnt FROM `schema_migrations` WHERE `version` = :version";
    match deck_mysql::Helper::query_rows(
        sql,
        vec![(
            "version".to_string(),
            tube::Value::from(version.to_string()),
        )],
        connector,
    ) {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                if let Some(cnt) = row.get::<i64, _>(0) {
                    return cnt > 0;
                }
                if let Some(cnt) = row.get::<u64, _>(0) {
                    return cnt > 0;
                }
            }
            false
        }
        Err(e) => {
            warn!(
                "migration check query failed (table may not exist yet): {}",
                e
            );
            false
        }
    }
}

#[cfg(feature = "mysql")]
fn record_migration_mysql(connector: &Connector, version: &str, description: &str) {
    let sql = "INSERT INTO `schema_migrations` (`version`, `description`, `applied_at`) VALUES (:version, :description, NOW())";
    match deck_mysql::Helper::execute(
        sql,
        vec![
            (
                "version".to_string(),
                tube::Value::from(version.to_string()),
            ),
            (
                "description".to_string(),
                tube::Value::from(description.to_string()),
            ),
        ],
        connector,
    ) {
        Ok(_) => {}
        Err(e) => {
            warn!("migration record failed: {}", e);
        }
    }
}

#[cfg(feature = "mysql")]
fn execute_ddl_mysql(connector: &Connector, sql: &str) {
    match deck_mysql::Helper::execute(sql, vec![], connector) {
        Ok(_) => {}
        Err(e) => {
            warn!("DDL execution failed: {} — sql: {}", e, sql);
        }
    }
}

#[cfg(feature = "mysql")]
fn run_column_migrations_mysql(connector: &Connector) {
    let migrations: Vec<(&str, &str)> = vec![
        (
            "v1_watchlist_stocks_camel_case",
            "ALTER TABLE `watchlist_stocks` \
             CHANGE COLUMN `stock_code` `stockCode` VARCHAR(16) NOT NULL COMMENT '股票代码', \
             CHANGE COLUMN `stock_name` `stockName` VARCHAR(64) DEFAULT '' COMMENT '股票名称', \
             CHANGE COLUMN `group_name` `groupName` VARCHAR(32) DEFAULT 'default' COMMENT '分组', \
             CHANGE COLUMN `sort_order` `sortOrder` INT DEFAULT 1 COMMENT '排序权重', \
             CHANGE COLUMN `create_time` `createTime` DATETIME DEFAULT CURRENT_TIMESTAMP, \
             CHANGE COLUMN `modify_time` `modifyTime` DATETIME DEFAULT CURRENT_TIMESTAMP",
        ),
        (
            "v2_watchlist_stocks_snake_case",
            "ALTER TABLE `watchlist_stocks` \
             CHANGE COLUMN `stockCode` `stock_code` VARCHAR(16) NOT NULL COMMENT '股票代码', \
             CHANGE COLUMN `stockName` `stock_name` VARCHAR(64) DEFAULT '' COMMENT '股票名称', \
             CHANGE COLUMN `groupName` `group_name` VARCHAR(32) DEFAULT 'default' COMMENT '分组', \
             CHANGE COLUMN `sortOrder` `sort_order` INT DEFAULT 1 COMMENT '排序权重', \
             CHANGE COLUMN `createTime` `create_time` DATETIME DEFAULT CURRENT_TIMESTAMP, \
             CHANGE COLUMN `modifyTime` `modify_time` DATETIME DEFAULT CURRENT_TIMESTAMP",
        ),
    ];

    for (version, sql) in &migrations {
        if is_migration_applied_mysql(connector, version) {
            info!("migration `{}` already applied, skipping", version);
            continue;
        }
        info!("applying migration `{}` …", version);
        execute_ddl_mysql(connector, sql);
        record_migration_mysql(connector, version, &format!("rename columns: {}", version));
    }
}

fn collect_alter_migrations_mysql() -> Vec<(&'static str, &'static str)> {
    vec![
        ("v0_analysis_history_report_json_mediumtext", "ALTER TABLE analysis_history MODIFY COLUMN report_json MEDIUMTEXT"),
        ("v0_news_intel_content_mediumtext", "ALTER TABLE news_intel MODIFY COLUMN content MEDIUMTEXT"),
        ("v0_analysis_history_context_snapshot_mediumtext", "ALTER TABLE analysis_history MODIFY COLUMN context_snapshot MEDIUMTEXT"),
        ("v0_analysis_history_news_content_mediumtext", "ALTER TABLE analysis_history MODIFY COLUMN news_content MEDIUMTEXT"),
        ("v1_stock_daily_add_macd_columns", "ALTER TABLE stock_daily ADD COLUMN IF NOT EXISTS `ma60` DOUBLE NOT NULL DEFAULT 0 COMMENT '60日均线', ADD COLUMN IF NOT EXISTS `dif` DOUBLE NOT NULL DEFAULT 0 COMMENT 'MACD DIF值', ADD COLUMN IF NOT EXISTS `dea` DOUBLE NOT NULL DEFAULT 0 COMMENT 'MACD DEA值', ADD COLUMN IF NOT EXISTS `macd_hist` DOUBLE NOT NULL DEFAULT 0 COMMENT 'MACD柱状值'"),
        ("v3_stock_daily_unique_index", "ALTER TABLE stock_daily ADD UNIQUE INDEX `idx_stock_daily_code_date` (`stock_code`, `trade_date`)"),
        ("v3_stock_daily_date_index", "ALTER TABLE stock_daily ADD INDEX `idx_stock_daily_date` (`trade_date`)"),
        ("v3_stock_daily_status_index", "ALTER TABLE stock_daily ADD INDEX `idx_stock_daily_code_status` (`stock_code`, `status`)"),
        ("v6_analysis_history_data_as_of", "ALTER TABLE analysis_history ADD COLUMN IF NOT EXISTS `data_as_of` VARCHAR(255) DEFAULT '' COMMENT '数据基准时间'"),
    ]
}

#[cfg(feature = "mysql")]
fn run_alter_migrations_mysql(connector: &Connector) {
    let alters = collect_alter_migrations_mysql();

    for (version, sql) in &alters {
        if is_migration_applied_mysql(connector, version) {
            info!("migration `{}` already applied, skipping", version);
            continue;
        }
        info!("running alter migration `{}` …", version);
        execute_ddl_mysql(connector, sql);
        record_migration_mysql(connector, version, &format!("alter: {}", version));
    }
}

// ---------------------------------------------------------------------------
// SQLite migration tracking
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
fn is_migration_applied_sqlite(connector: &Connector, version: &str) -> bool {
    let sql = format!(
        "SELECT COUNT(*) AS cnt FROM \"schema_migrations\" WHERE \"version\" = '{}'",
        version.replace('\'', "''")
    );
    match sqlite_query_count(connector, &sql) {
        Ok(cnt) => cnt > 0,
        Err(e) => {
            warn!(
                "sqlite migration check failed (table may not exist yet): {}",
                e
            );
            false
        }
    }
}

#[cfg(feature = "sqlite")]
fn record_migration_sqlite(connector: &Connector, version: &str, description: &str) {
    let sql = format!(
        "INSERT INTO \"schema_migrations\" (\"version\", \"description\", \"applied_at\") VALUES ('{}', '{}', datetime('now'))",
        version.replace('\'', "''"),
        description.replace('\'', "''"),
    );
    execute_ddl_sqlite(connector, &sql);
}

#[cfg(feature = "sqlite")]
fn execute_ddl_sqlite(connector: &Connector, sql: &str) {
    let conn_str = connector.get_conn_str();
    match Connection::open(&conn_str) {
        Ok(conn) => {
            if let Err(e) = conn.execute_batch(sql) {
                warn!(
                    "sqlite DDL execution FAILED: {} — sql: {}",
                    e,
                    &sql[..sql.len().min(200)]
                );
            }
        }
        Err(e) => {
            warn!("sqlite open FAILED: {}", e);
        }
    }
}

#[cfg(feature = "sqlite")]
fn sqlite_query_count(connector: &Connector, sql: &str) -> Result<i64, String> {
    let conn_str = connector.get_conn_str();
    match Connection::open(&conn_str) {
        Ok(conn) => match conn.query_row(sql, [], |row| row.get::<_, i64>(0)) {
            Ok(cnt) => Ok(cnt),
            Err(e) => Err(format!("{}", e)),
        },
        Err(e) => Err(format!("{}", e)),
    }
}

fn collect_alter_migrations_sqlite() -> Vec<(&'static str, &'static str)> {
    vec![
        ("v1_stock_daily_add_macd_columns", "ALTER TABLE stock_daily ADD COLUMN \"ma60\" REAL NOT NULL DEFAULT 0"),
        ("v1_stock_daily_add_dif", "ALTER TABLE stock_daily ADD COLUMN \"dif\" REAL NOT NULL DEFAULT 0"),
        ("v1_stock_daily_add_dea", "ALTER TABLE stock_daily ADD COLUMN \"dea\" REAL NOT NULL DEFAULT 0"),
        ("v1_stock_daily_add_macd_hist", "ALTER TABLE stock_daily ADD COLUMN \"macd_hist\" REAL NOT NULL DEFAULT 0"),
        ("v2_watchlist_stocks_stock_code", "ALTER TABLE \"watchlist_stocks\" RENAME COLUMN \"stockCode\" TO \"stock_code\""),
        ("v2_watchlist_stocks_stock_name", "ALTER TABLE \"watchlist_stocks\" RENAME COLUMN \"stockName\" TO \"stock_name\""),
        ("v2_watchlist_stocks_group_name", "ALTER TABLE \"watchlist_stocks\" RENAME COLUMN \"groupName\" TO \"group_name\""),
        ("v2_watchlist_stocks_sort_order", "ALTER TABLE \"watchlist_stocks\" RENAME COLUMN \"sortOrder\" TO \"sort_order\""),
        ("v2_watchlist_stocks_create_time", "ALTER TABLE \"watchlist_stocks\" RENAME COLUMN \"createTime\" TO \"create_time\""),
        ("v2_watchlist_stocks_modify_time", "ALTER TABLE \"watchlist_stocks\" RENAME COLUMN \"modifyTime\" TO \"modify_time\""),
        ("v3_stock_daily_unique_index", "CREATE UNIQUE INDEX IF NOT EXISTS \"idx_stock_daily_code_date\" ON \"stock_daily\" (\"stock_code\", \"trade_date\")"),
        ("v3_stock_daily_date_index", "CREATE INDEX IF NOT EXISTS \"idx_stock_daily_date\" ON \"stock_daily\" (\"trade_date\")"),
        ("v3_stock_daily_status_index", "CREATE INDEX IF NOT EXISTS \"idx_stock_daily_code_status\" ON \"stock_daily\" (\"stock_code\", \"status\")"),
        ("v4_stock_pool_unique_code", "CREATE UNIQUE INDEX IF NOT EXISTS \"idx_stock_pool_code\" ON \"stock_pool\" (\"stock_code\")"),
        // v5: stock_pool 扩展字段
        ("v5_stock_pool_symbol", "ALTER TABLE \"stock_pool\" ADD COLUMN \"symbol\" TEXT DEFAULT ''"),
        ("v5_stock_pool_en_name", "ALTER TABLE \"stock_pool\" ADD COLUMN \"en_name\" TEXT DEFAULT ''"),
        ("v5_stock_pool_company_name", "ALTER TABLE \"stock_pool\" ADD COLUMN \"company_name\" TEXT DEFAULT ''"),
        ("v5_stock_pool_address", "ALTER TABLE \"stock_pool\" ADD COLUMN \"address\" TEXT DEFAULT ''"),
        ("v5_stock_pool_a_code", "ALTER TABLE \"stock_pool\" ADD COLUMN \"a_code\" TEXT DEFAULT ''"),
        ("v5_stock_pool_a_name", "ALTER TABLE \"stock_pool\" ADD COLUMN \"a_name\" TEXT DEFAULT ''"),
        ("v5_stock_pool_a_cost", "ALTER TABLE \"stock_pool\" ADD COLUMN \"a_cost\" REAL"),
        ("v5_stock_pool_province", "ALTER TABLE \"stock_pool\" ADD COLUMN \"province\" TEXT DEFAULT ''"),
        ("v5_stock_pool_city", "ALTER TABLE \"stock_pool\" ADD COLUMN \"city\" TEXT DEFAULT ''"),
        ("v5_stock_pool_area", "ALTER TABLE \"stock_pool\" ADD COLUMN \"area\" TEXT DEFAULT ''"),
        ("v5_stock_pool_market_time", "ALTER TABLE \"stock_pool\" ADD COLUMN \"market_time\" TEXT"),
        ("v5_stock_pool_website", "ALTER TABLE \"stock_pool\" ADD COLUMN \"website\" TEXT DEFAULT ''"),
        ("v5_stock_pool_pe", "ALTER TABLE \"stock_pool\" ADD COLUMN \"pe\" REAL"),
        ("v5_stock_pool_outstanding", "ALTER TABLE \"stock_pool\" ADD COLUMN \"outstanding\" REAL"),
        ("v5_stock_pool_total", "ALTER TABLE \"stock_pool\" ADD COLUMN \"total\" REAL"),
        ("v5_stock_pool_total_assets", "ALTER TABLE \"stock_pool\" ADD COLUMN \"total_assets\" REAL"),
        ("v5_stock_pool_flow_assets", "ALTER TABLE \"stock_pool\" ADD COLUMN \"flow_assets\" REAL"),
        ("v5_stock_pool_fixed_assets", "ALTER TABLE \"stock_pool\" ADD COLUMN \"fixed_assets\" REAL"),
        ("v5_stock_pool_esp", "ALTER TABLE \"stock_pool\" ADD COLUMN \"esp\" REAL"),
        ("v5_stock_pool_per_assets", "ALTER TABLE \"stock_pool\" ADD COLUMN \"per_assets\" REAL"),
        ("v5_stock_pool_pb", "ALTER TABLE \"stock_pool\" ADD COLUMN \"pb\" REAL"),
        ("v5_stock_pool_unassigned_profit", "ALTER TABLE \"stock_pool\" ADD COLUMN \"unassigned_profit\" REAL DEFAULT 0"),
        ("v5_stock_pool_per_unassigned", "ALTER TABLE \"stock_pool\" ADD COLUMN \"per_unassigned\" REAL"),
        ("v5_stock_pool_rev", "ALTER TABLE \"stock_pool\" ADD COLUMN \"rev\" REAL"),
        ("v5_stock_pool_profit", "ALTER TABLE \"stock_pool\" ADD COLUMN \"profit\" REAL"),
        ("v5_stock_pool_gpr", "ALTER TABLE \"stock_pool\" ADD COLUMN \"gpr\" REAL"),
        ("v5_stock_pool_npr", "ALTER TABLE \"stock_pool\" ADD COLUMN \"npr\" TEXT DEFAULT ''"),
        ("v5_stock_pool_holders", "ALTER TABLE \"stock_pool\" ADD COLUMN \"holders\" INTEGER"),
        // v5: stock_quote 唯一索引
        ("v5_stock_quote_unique", "CREATE UNIQUE INDEX IF NOT EXISTS \"idx_stock_quote_code_date\" ON \"stock_quote\" (\"stock_code\", \"trade_date\")"),
        ("v6_analysis_history_data_as_of", "ALTER TABLE \"analysis_history\" ADD COLUMN \"data_as_of\" TEXT DEFAULT ''"),
    ]
}

#[cfg(feature = "sqlite")]
fn run_alter_migrations_sqlite(connector: &Connector) {
    let alters = collect_alter_migrations_sqlite();

    for (version, sql) in &alters {
        if is_migration_applied_sqlite(connector, version) {
            info!("migration `{}` already applied, skipping", version);
            continue;
        }
        info!("running sqlite alter migration `{}` …", version);
        execute_ddl_sqlite(connector, sql);
        record_migration_sqlite(connector, version, &format!("alter: {}", version));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_type_to_sql_mysql() {
        assert_eq!(rust_type_to_sql("i64", Dialect::Mysql), "BIGINT");
        assert_eq!(rust_type_to_sql("i32", Dialect::Mysql), "INT");
        assert_eq!(rust_type_to_sql("i16", Dialect::Mysql), "SMALLINT");
        assert_eq!(rust_type_to_sql("i8", Dialect::Mysql), "TINYINT");
        assert_eq!(rust_type_to_sql("u64", Dialect::Mysql), "BIGINT UNSIGNED");
        assert_eq!(rust_type_to_sql("u32", Dialect::Mysql), "INT UNSIGNED");
        assert_eq!(rust_type_to_sql("f64", Dialect::Mysql), "DOUBLE");
        assert_eq!(rust_type_to_sql("f32", Dialect::Mysql), "FLOAT");
        assert_eq!(rust_type_to_sql("String", Dialect::Mysql), "VARCHAR(255)");
        assert_eq!(rust_type_to_sql("bool", Dialect::Mysql), "TINYINT(1)");
        assert_eq!(
            rust_type_to_sql("NaiveDateTime", Dialect::Mysql),
            "DATETIME"
        );
        assert_eq!(rust_type_to_sql("Vec<u8>", Dialect::Mysql), "BLOB");
        assert_eq!(
            rust_type_to_sql("Option<String>", Dialect::Mysql),
            "VARCHAR(255)"
        );
        assert_eq!(
            rust_type_to_sql("Option<NaiveDateTime>", Dialect::Mysql),
            "DATETIME"
        );
    }

    #[test]
    fn test_rust_type_to_sql_sqlite() {
        assert_eq!(rust_type_to_sql("i64", Dialect::Sqlite), "INTEGER");
        assert_eq!(rust_type_to_sql("i32", Dialect::Sqlite), "INTEGER");
        assert_eq!(rust_type_to_sql("u64", Dialect::Sqlite), "INTEGER");
        assert_eq!(rust_type_to_sql("f64", Dialect::Sqlite), "REAL");
        assert_eq!(rust_type_to_sql("f32", Dialect::Sqlite), "REAL");
        assert_eq!(rust_type_to_sql("String", Dialect::Sqlite), "TEXT");
        assert_eq!(rust_type_to_sql("bool", Dialect::Sqlite), "INTEGER");
        assert_eq!(rust_type_to_sql("NaiveDateTime", Dialect::Sqlite), "TEXT");
        assert_eq!(rust_type_to_sql("Vec<u8>", Dialect::Sqlite), "BLOB");
        assert_eq!(rust_type_to_sql("Option<String>", Dialect::Sqlite), "TEXT");
    }

    #[test]
    fn test_create_table_sql_schema_migration() {
        let cls = SchemaMigration::class();
        let mysql_sql = create_table_sql(&cls, Dialect::Mysql);
        assert!(mysql_sql.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(mysql_sql.contains("schema_migrations"));
        assert!(mysql_sql.contains("PRIMARY KEY"));
        assert!(mysql_sql.contains("AUTO_INCREMENT"));
        assert!(mysql_sql.contains("ENGINE=InnoDB"));

        let sqlite_sql = create_table_sql(&cls, Dialect::Sqlite);
        assert!(sqlite_sql.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(sqlite_sql.contains("schema_migrations"));
        assert!(sqlite_sql.contains("PRIMARY KEY"));
        assert!(sqlite_sql.contains("AUTOINCREMENT"));
        assert!(!sqlite_sql.contains("ENGINE=InnoDB"));
    }
}
