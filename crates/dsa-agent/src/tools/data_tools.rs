use dsa_core::{utils, DsaError, DsaResult};
use qta_crawler::{Stock, QQ};
use tube::Value;

pub struct DataTools;

impl DataTools {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_realtime_quote(symbol: &str) -> DsaResult<Value> {
        QQ::new()
            .get_realtime_quote(symbol)
            .await
            .map_err(|e| DsaError::StockData(format!("获取实时行情失败: {}", e)))
    }

    pub async fn get_stock_info(code: &str) -> DsaResult<Value> {
        Stock::get_detail(code)
            .await
            .map_err(|e| DsaError::StockData(format!("获取股票信息失败: {}", e)))
    }

    pub async fn get_kline_data(code: &str, period: &str) -> DsaResult<Value> {
        let bars = utils::fetch_kline(code, period).await?;

        Ok(value!({
            "code": code,
            "period": period,
            "count": bars.len(),
            "data": serde_json::to_value(&bars).unwrap_or_default(),
        }))
    }

    pub async fn get_realtime_price(symbol: &str) -> DsaResult<Value> {
        utils::fetch_realtime_price(symbol).await
    }
}
