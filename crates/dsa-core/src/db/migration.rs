//! Database schema migration utility
//!
//! Uses DataModel trait metadata (Class, Attribute) to generate
//! `CREATE TABLE IF NOT EXISTS` SQL and tracks applied migrations
//! in a `schema_migrations` table.

use deck_connector::Connector;
use deck_model::{Attribute, Class, DataModel};
use tracing::{info, warn};

use crate::models::db::*;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run all pending migrations. Creates the `schema_migrations` tracking table
/// first, then iterates every model and ensures its table exists.
pub fn run_migrations(connector: &Connector) {
    // 1. Ensure the migration tracking table itself exists
    let migration_sql = create_table_sql(&SchemaMigration::class());
    execute_ddl(connector, &migration_sql);

    // 2. Collect all model tables in a stable order
    let models: Vec<(&str, Class)> = vec![
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
        ("portfolio_corporate_actions", PortfolioCorporateAction::class()),
        ("portfolio_daily_snapshots", PortfolioDailySnapshot::class()),
        ("portfolio_fx_rates", PortfolioFxRate::class()),
        ("alert_rules", AlertRule::class()),
        ("alert_triggers", AlertTrigger::class()),
        ("alert_notifications", AlertNotification::class()),
        ("alert_cooldowns", AlertCooldown::class()),
        ("watchlist_stocks", WatchlistStock::class()),
        ("conversation_messages", ConversationMessage::class()),
        ("conversation_summaries", ConversationSummary::class()),
        ("agent_provider_turns", AgentProviderTurn::class()),
        ("llm_usage", LlmUsage::class()),
        ("schema_migrations", SchemaMigration::class()),
    ];

    // 3. Create each table if it does not yet exist
    for (label, cls) in &models {
        // Skip schema_migrations — already handled above
        if *label == "schema_migrations" {
            continue;
        }
        let version = format!("v0_{}_create", label);
        if is_migration_applied(connector, &version) {
            info!("migration `{}` already applied, skipping", version);
            continue;
        }
        let sql = create_table_sql(cls);
        info!("creating table `{}` …", cls.table_name);
        execute_ddl(connector, &sql);
        record_migration(connector, &version, &format!("create table {}", cls.table_name));
    }

    info!("all migrations completed");

    run_column_migrations(connector);
    run_alter_migrations(connector);
}

// ---------------------------------------------------------------------------
// SQL generation
// ---------------------------------------------------------------------------

/// Generate a `CREATE TABLE IF NOT EXISTS` statement from a `Class` metadata.
pub fn create_table_sql(cls: &Class) -> String {
    let table_name = &cls.table_name;
    let mut cols: Vec<String> = Vec::new();
    let mut primary_keys: Vec<String> = Vec::new();

    for attr in &cls.attributes {
        let col = build_column_def(attr, &cls.primary_type);
        cols.push(col);
        if attr.primary {
            primary_keys.push(column_name(attr));
        }
    }

    // Composite / explicit primary key
    if !primary_keys.is_empty() {
        cols.push(format!(
            "PRIMARY KEY ({})",
            primary_keys.join(", ")
        ));
    }

    let table_opts = if cls.primary_type == "identity" {
        // MySQL InnoDB AUTO_INCREMENT on PK
        String::new()
    } else {
        String::new()
    };

    format!(
        "CREATE TABLE IF NOT EXISTS `{}` (\n  {}\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4{}",
        table_name,
        cols.join(",\n  "),
        table_opts,
    )
}

/// Build a single column definition line from an `Attribute`.
fn build_column_def(attr: &Attribute, primary_type: &str) -> String {
    let name = column_name(attr);
    let sql_type = attribute_sql_type(attr, primary_type);

    let mut parts = vec![format!("`{}` {}", name, sql_type)];

    // NOT NULL
    if attr.required || attr.primary {
        parts.push("NOT NULL".to_owned());
    }

    // AUTO_INCREMENT (only on identity PK)
    if attr.increment > 0 && attr.primary && primary_type == "identity" {
        parts.push("AUTO_INCREMENT".to_owned());
    }

    // DEFAULT
    if !attr.default_value.is_empty() && !attr.primary {
        parts.push(format!("DEFAULT {}", attr.default_value));
    }

    // UNIQUE
    if attr.unique && !attr.primary {
        parts.push("UNIQUE".to_owned());
    }

    // COMMENT
    if !attr.comment.is_empty() {
        parts.push(format!("COMMENT '{}'", attr.comment.replace('\'', "\\'")));
    }

    parts.join(" ")
}

/// Resolve the SQL column name: use `alias` if set, else `name`.
fn column_name(attr: &Attribute) -> String {
    if !attr.alias.is_empty() {
        attr.alias.clone()
    } else {
        // Convert snake_case field name to camelCase column name
        // to match the `rename` annotations used throughout the models
        attr.name.clone()
    }
}

/// Determine the MySQL column type for an attribute.
fn attribute_sql_type(attr: &Attribute, primary_type: &str) -> String {
    // Override PK type based on table's primary strategy
    if attr.primary {
        if primary_type == "uuid" {
            return "VARCHAR(36)".to_owned();
        }
        if primary_type == "identity" {
            return "BIGINT".to_owned();
        }
    }

    // If the data_length is explicitly set, honour it for VARCHAR
    if attr.data_length > 0 && attr.data_type.contains("String") {
        return format!("VARCHAR({})", attr.data_length);
    }

    rust_type_to_sql(&attr.data_type).to_owned()
}

/// Map a Rust type string (as produced by the derive macro) to a MySQL column type.
pub fn rust_type_to_sql(data_type: &str) -> &'static str {
    // The derive macro records Rust type names like "i64", "String",
    // "Option<NaiveDateTime>", "bool", etc.
    // We match from most specific to least specific.

    // Option types — strip the Option wrapper
    let base = if data_type.starts_with("Option<") && data_type.ends_with('>') {
        &data_type[7..data_type.len() - 1]
    } else {
        data_type
    };

    match base {
        // Integer types
        "i64" => "BIGINT",
        "i32" => "INT",
        "i16" => "SMALLINT",
        "i8" => "TINYINT",

        "u64" => "BIGINT UNSIGNED",
        "u32" => "INT UNSIGNED",
        "u16" => "SMALLINT UNSIGNED",
        "u8" => "TINYINT UNSIGNED",

        // Float types
        "f64" => "DOUBLE",
        "f32" => "FLOAT",

        // Boolean
        "bool" => "TINYINT(1)",

        // String
        "String" => "VARCHAR(255)",

        // DateTime
        "NaiveDateTime" => "DATETIME",

        // Binary
        "Vec<u8>" => "BLOB",

        // Fallback
        _ => {
            // Handle types we might not have exact matches for
            if base.contains("String") {
                "VARCHAR(255)"
            } else if base.contains("NaiveDateTime") || base.contains("DateTime") {
                "DATETIME"
            } else if base.contains("Vec<u8>") || base.contains("Bytes") {
                "BLOB"
            } else {
                "TEXT"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Migration tracking
// ---------------------------------------------------------------------------

/// Check whether a migration version has already been applied.
fn is_migration_applied(connector: &Connector, version: &str) -> bool {
    let sql = "SELECT COUNT(*) AS cnt FROM `schema_migrations` WHERE `version` = :version";
    match deck::Helper::query_rows(
        sql,
        vec![("version".to_string(), tube::Value::from(version.to_string()))],
        connector,
    ) {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                // mysql crate Row — read the first column as i64
                if let Some(cnt) = row.get::<i64, _>(0) {
                    return cnt > 0;
                }
                // Try u64
                if let Some(cnt) = row.get::<u64, _>(0) {
                    return cnt > 0;
                }
            }
            false
        }
        Err(e) => {
            warn!("migration check query failed (table may not exist yet): {}", e);
            false
        }
    }
}

/// Record a successful migration in the tracking table.
fn record_migration(connector: &Connector, version: &str, description: &str) {
    let sql = "INSERT INTO `schema_migrations` (`version`, `description`, `applied_at`) VALUES (:version, :description, NOW())";
    match deck::Helper::execute(
        sql,
        vec![
            ("version".to_string(), tube::Value::from(version.to_string())),
            ("description".to_string(), tube::Value::from(description.to_string())),
        ],
        connector,
    ) {
        Ok(_) => {}
        Err(e) => {
            warn!("migration record failed: {}", e);
        }
    }
}

/// Execute a DDL / DML statement that returns no meaningful rows.
fn execute_ddl(connector: &Connector, sql: &str) {
    match deck::Helper::execute(sql, vec![], connector) {
        Ok(_) => {}
        Err(e) => {
            warn!("DDL execution failed: {} — sql: {}", e, sql);
        }
    }
}

// ---------------------------------------------------------------------------
// Incremental column migrations (ALTER TABLE)
// ---------------------------------------------------------------------------

fn run_column_migrations(connector: &Connector) {
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
    ];

    for (version, sql) in &migrations {
        if is_migration_applied(connector, version) {
            info!("migration `{}` already applied, skipping", version);
            continue;
        }
        info!("applying migration `{}` …", version);
        execute_ddl(connector, sql);
        record_migration(connector, version, &format!("rename columns: {}", version));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_type_to_sql() {
        assert_eq!(rust_type_to_sql("i64"), "BIGINT");
        assert_eq!(rust_type_to_sql("i32"), "INT");
        assert_eq!(rust_type_to_sql("i16"), "SMALLINT");
        assert_eq!(rust_type_to_sql("i8"), "TINYINT");
        assert_eq!(rust_type_to_sql("u64"), "BIGINT UNSIGNED");
        assert_eq!(rust_type_to_sql("u32"), "INT UNSIGNED");
        assert_eq!(rust_type_to_sql("f64"), "DOUBLE");
        assert_eq!(rust_type_to_sql("f32"), "FLOAT");
        assert_eq!(rust_type_to_sql("String"), "VARCHAR(255)");
        assert_eq!(rust_type_to_sql("bool"), "TINYINT(1)");
        assert_eq!(rust_type_to_sql("NaiveDateTime"), "DATETIME");
        assert_eq!(rust_type_to_sql("Vec<u8>"), "BLOB");
        // Option variants
        assert_eq!(rust_type_to_sql("Option<String>"), "VARCHAR(255)");
        assert_eq!(rust_type_to_sql("Option<NaiveDateTime>"), "DATETIME");
    }

    #[test]
    fn test_create_table_sql_schema_migration() {
        let cls = SchemaMigration::class();
        let sql = create_table_sql(&cls);
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(sql.contains("schema_migrations"));
        assert!(sql.contains("PRIMARY KEY"));
    }
}

fn run_alter_migrations(connector: &Connector) {
    let alters: Vec<(&str, &str)> = vec![
        ("v0_analysis_history_report_json_mediumtext", "ALTER TABLE analysis_history MODIFY COLUMN report_json MEDIUMTEXT"),
        ("v0_news_intel_content_mediumtext", "ALTER TABLE news_intel MODIFY COLUMN content MEDIUMTEXT"),
        ("v0_analysis_history_context_snapshot_mediumtext", "ALTER TABLE analysis_history MODIFY COLUMN context_snapshot MEDIUMTEXT"),
        ("v0_analysis_history_news_content_mediumtext", "ALTER TABLE analysis_history MODIFY COLUMN news_content MEDIUMTEXT"),
        ("v1_stock_daily_add_macd_columns", "ALTER TABLE stock_daily ADD COLUMN IF NOT EXISTS `ma60` DOUBLE NOT NULL DEFAULT 0 COMMENT '60日均线', ADD COLUMN IF NOT EXISTS `dif` DOUBLE NOT NULL DEFAULT 0 COMMENT 'MACD DIF值', ADD COLUMN IF NOT EXISTS `dea` DOUBLE NOT NULL DEFAULT 0 COMMENT 'MACD DEA值', ADD COLUMN IF NOT EXISTS `macd_hist` DOUBLE NOT NULL DEFAULT 0 COMMENT 'MACD柱状值'"),
    ];

    for (version, sql) in &alters {
        if is_migration_applied(connector, version) {
            info!("migration `{}` already applied, skipping", version);
            continue;
        }
        info!("running alter migration `{}` …", version);
        execute_ddl(connector, sql);
        record_migration(connector, version, &format!("alter: {}", version));
    }
}
