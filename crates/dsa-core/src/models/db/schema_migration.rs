//! 数据库Schema迁移表

use deck::Model;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Model, Default, Debug, Clone, Serialize, Deserialize)]
#[table(name = "schema_migrations", comment = "数据库迁移记录", primary = "identity")]
pub struct SchemaMigration {
    #[field(primary = true, increment = 1)]
    pub id: i64,

    #[field(required = true, unique = true, comment = "迁移版本")]
    pub version: String,

    #[field(comment = "描述")]
    pub description: String,

    #[field(rename = "applied_at", default_value = "current_timestamp()")]
    pub applied_at: Option<chrono::NaiveDateTime>,
}
