//! 市场工具 - 大盘和板块数据

use dsa_core::{DsaError, DsaResult};
use qta_crawler::{Complex, Real};
use tube::Value;

pub struct MarketTools;

impl MarketTools {
    pub fn new() -> Self { Self }

    /// 获取市场概览 - 主要指数实时数据
    pub async fn get_market_overview() -> DsaResult<Value> {
        let real = Real::new();

        // 并行获取三大指数
        let sh_result = real.get_price("sh000001").await;
        let sz_result = real.get_price("sz399001").await;
        let cy_result = real.get_price("sz399006").await;

        let sh_val = sh_result.unwrap_or_else(|_| value!({"name": "上证指数", "error": "获取失败"}));
        let sz_val = sz_result.unwrap_or_else(|_| value!({"name": "深证成指", "error": "获取失败"}));
        let cy_val = cy_result.unwrap_or_else(|_| value!({"name": "创业板指", "error": "获取失败"}));

        Ok(value!({
            "shanghai": sh_val,
            "shenzhen": sz_val,
            "chinext": cy_val,
        }))
    }

    /// 获取热门板块
    pub async fn get_hot_sectors() -> DsaResult<Value> {
        Complex::get_hot_stock()
            .await
            .map_err(|e| DsaError::StockData(format!("获取热门板块失败: {}", e)))
    }
}
