//! Agent handler - 分发到 agent orchestrator

use tube::{Result, Value};
use tube_web::RequestParameter;

pub async fn distribute(param: &RequestParameter) -> Result<Value> {
    let orch = dsa_agent::Orchestrator::new();
    orch.dispatch("agent", &param.method, &param.value)
        .await
        .map_err(|e| {
            let msg = format!("{}", e);
            error!("{}", &msg);
            tube::Error::from(msg)
        })
}
