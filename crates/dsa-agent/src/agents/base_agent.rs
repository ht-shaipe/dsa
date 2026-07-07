//! 基础Agent trait

use async_trait::async_trait;
use dsa_core::{DsaResult};
use tube::Value;

#[async_trait(?Send)]
pub trait BaseAgent: Send + Sync {
    fn name(&self) -> &str;
    fn role(&self) -> &str;
    async fn process(&self, input: &Value) -> DsaResult<Value>;
}
