use qta_crawler::{Basic, Complex, EastMoney, Real};
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct Market {
    request: RequestParameter,
}

impl Market {
    pub fn new(param: &RequestParameter) -> Self {
        Market {
            request: param.clone(),
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "overview" => self.overview().await,
            "review" => self.review().await,
            "hot_sectors" => self.hot_sectors().await,
            "hot_stocks" => self.hot_stocks().await,
            "index" => self.index().await,
            "calendar" => self.calendar().await,
            _ => Err(error!("market不支持方法: {}", method)),
        }
    }

    async fn overview(&self) -> Result<Value> {
        let real = Real::new();

        let sh = real.get_price("sh000001").await.ok();
        let sz = real.get_price("sz399001").await.ok();
        let cy = real.get_price("sz399006").await.ok();

        let build = |v: Option<Value>, default_name: &str| -> Value {
            match v {
                Some(data) => {
                    let close = data.get("close").and_then(|p| p.as_f64()).unwrap_or(0.0);
                    let change_pct = data.get("changePercent").and_then(|p| p.as_f64()).unwrap_or(0.0);
                    let name = data.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| default_name.to_string());
                    value!({
                        "name": name,
                        "price": close,
                        "close": close,
                        "changePercent": change_pct,
                        "open": data.get("open").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "preClose": data.get("preClose").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "high": data.get("high").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "low": data.get("low").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "volume": data.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        "amount": data.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    })
                }
                None => value!({
                    "name": default_name,
                    "price": 0.0,
                    "close": 0.0,
                    "changePercent": 0.0,
                }),
            }
        };

        Ok(value!({
            "sh": build(sh, "上证指数"),
            "sz": build(sz, "深证成指"),
            "cy": build(cy, "创业板指"),
        }))
    }

    async fn review(&self) -> Result<Value> {
        let gen = dsa_pipeline::market_review::MarketReviewGenerator::new();
        gen.generate(&value!({})).await
            .map_err(|e| error!("生成市场回顾失败: {}", e))
    }

    async fn hot_sectors(&self) -> Result<Value> {
        let complex = Complex::get_hot_stock()
            .await
            .map_err(|e| error!("获取热门板块失败: {}", e))?;
        Ok(complex)
    }

    async fn hot_stocks(&self) -> Result<Value> {
        let em = EastMoney::new();
        let spot = em
            .stock_zh_a_spot()
            .await
            .map_err(|e| error!("获取热门股票失败: {}", e))?;

        let hot: Vec<Value> = spot.into_iter().take(20).collect();
        Ok(Value::Array(hot))
    }

    async fn index(&self) -> Result<Value> {
        let code = self.value()
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "sh000001".to_string());
        let real = Real::new();
        let data = real
            .get_price(&code)
            .await
            .map_err(|e| error!("获取指数失败: {}", e))?;
        Ok(data)
    }

    async fn calendar(&self) -> Result<Value> {
        let market = self.value()
            .get("market")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "SSE".to_string());
        let basic = Basic::new();
        let dates = basic
            .get_trade_calendar(&market)
            .await
            .map_err(|e| error!("获取交易日历失败: {}", e))?;
        let data: Vec<Value> = dates.iter().map(|d| Value::from(d.as_str())).collect();
        Ok(Value::Array(data))
    }

    fn value(&self) -> Value {
        self.request.value.clone()
    }
}
