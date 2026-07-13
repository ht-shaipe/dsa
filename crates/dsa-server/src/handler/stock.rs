//! Stock handler - 分发到 stock service

use tube::{Result, Value};
use tube_web::RequestParameter;

pub async fn distribute(param: &RequestParameter) -> Result<Value> {
    let service = dsa_service::Stock::new(param);
    service.dispatch(&param.method).await.map_err(|e| { error!("{}", e); e })
}
