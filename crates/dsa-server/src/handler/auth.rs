//! Auth handler - 分发到 auth_service

use tube::{Result, Value};
use tube_web::RequestParameter;

pub async fn distribute(param: &RequestParameter) -> Result<Value> {
    let service = dsa_service::AuthService::new();
    service.dispatch(&param.method, &param.value).await.map_err(|e| error!("{}", e))
}
