//! Stock handler - 分发到 stock_service

use tube::{Result, Value};
use tube_web::RequestParameter;

pub async fn distribute(param: &RequestParameter) -> Result<Value> {
    let service = dsa_service::StockService::new();
    service.dispatch(&param.method, &param.value).await.map_err(|e| error!("{}", e))
}
