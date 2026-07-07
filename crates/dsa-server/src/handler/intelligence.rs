//! Intelligence handler - 分发到 intelligence_service

use tube::{Result, Value};
use tube_web::RequestParameter;

pub async fn distribute(param: &RequestParameter) -> Result<Value> {
    let service = dsa_service::IntelligenceService::new();
    service.dispatch(&param.method, &param.value).await.map_err(|e| error!("{}", e))
}
