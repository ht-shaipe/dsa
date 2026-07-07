//! Backtest handler - 分发到 backtest engine

use tube::{Result, Value};
use tube_web::RequestParameter;

pub async fn distribute(param: &RequestParameter) -> Result<Value> {
    let engine = dsa_backtest::BacktestEngine::new();
    engine.dispatch(&param.method, &param.value).await.map_err(|e| error!("{}", e))
}
