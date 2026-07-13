use tube::{Result, Value};
use tube_web::RequestParameter;

pub async fn distribute(param: &RequestParameter) -> Result<Value> {
    let service = dsa_service::Search::new(param);
    service.dispatch(&param.method).await.map_err(|e| { error!("{}", e); e })
}
