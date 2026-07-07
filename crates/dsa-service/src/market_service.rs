//! 大盘/市场服务

use dsa_core::{DsaError, DsaResult};
use qta_crawler::{Basic, Complex, EastMoney, Real};
use tube::Value;

/// 大盘/市场服务
pub struct MarketService {}

impl MarketService {
    /// 创建市场服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: overview, review, hot_sectors, hot_stocks, index, calendar
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "overview" => self.overview().await,
            "review" => self.review().await,
            "hot_sectors" => self.hot_sectors().await,
            "hot_stocks" => self.hot_stocks().await,
            "index" => self.index(params).await,
            "calendar" => self.calendar(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "market不支持方法: {}",
                method
            ))),
        }
    }

    async fn overview(&self) -> DsaResult<Value> {
        let real = Real::new();
        let sh = real.get_price("sh000001").await.ok();
        let sz = real.get_price("sz399001").await.ok();
        let cy = real.get_price("sz399006").await.ok();

        Ok(value!({
            "status": "ok",
            "data": {
                "shanghai": sh.unwrap_or(Value::Null),
                "shenzhen": sz.unwrap_or(Value::Null),
                "chinext": cy.unwrap_or(Value::Null),
            }
        }))
    }

    async fn review(&self) -> DsaResult<Value> {
        let gen = dsa_pipeline::market_review::MarketReviewGenerator::new();
        gen.generate(&value!({})).await
    }

    async fn hot_sectors(&self) -> DsaResult<Value> {
        let complex = Complex::get_hot_stock()
            .await
            .map_err(|e| DsaError::StockData(format!("获取热门板块失败: {}", e)))?;
        Ok(value!({"status": "ok", "data": complex}))
    }

    async fn hot_stocks(&self) -> DsaResult<Value> {
        let em = EastMoney::new();
        let spot = em
            .stock_zh_a_spot()
            .await
            .map_err(|e| DsaError::StockData(format!("获取热门股票失败: {}", e)))?;

        let hot: Vec<Value> = spot.into_iter().take(20).collect();
        Ok(value!({"status": "ok", "data": hot}))
    }

    async fn index(&self, params: &Value) -> DsaResult<Value> {
        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "sh000001".to_string());
        let real = Real::new();
        let data = real
            .get_price(&code)
            .await
            .map_err(|e| DsaError::StockData(format!("获取指数失败: {}", e)))?;
        Ok(value!({"status": "ok", "data": data}))
    }

    async fn calendar(&self, params: &Value) -> DsaResult<Value> {
        let market = params
            .get("market")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "SSE".to_string());
        let basic = Basic::new();
        let data = basic
            .get_trade_calendar(&market)
            .await
            .map_err(|e| DsaError::StockData(format!("获取交易日历失败: {}", e)))?;
        Ok(value!({"status": "ok", "data": data}))
    }
}
