//! 股票数据服务 - 整合 qta_crawler 多源数据

use dsa_core::{DsaError, DsaResult, utils};
use qta_crawler::{Basic, EastMoney, History, QQ, Real, Stock};
use tube::Value;

/// 股票数据服务
pub struct StockService {}

impl StockService {
    /// 创建股票服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 请求分发 - 可用方法: search, quote, quotes, kline, history, info, watchlist, spot, industries, concepts
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "search" => self.search(params).await,
            "quote" => self.get_quote(params).await,
            "quotes" => self.get_quotes(params).await,
            "kline" => self.get_kline(params).await,
            "history" => self.get_history(params).await,
            "info" => self.get_info(params).await,
            "watchlist" => self.get_watchlist(params).await,
            "spot" => self.get_spot(params).await,
            "industries" => self.get_industries().await,
            "concepts" => self.get_concepts().await,
            _ => Err(DsaError::ApiRouting(format!("stock不支持方法: {}", method))),
        }
    }

    async fn search(&self, params: &Value) -> DsaResult<Value> {
        let query = utils::param_string(params, "query");

        if query.is_empty() {
            return Ok(value!({"status": "ok", "data": []}));
        }

        let code = query.replace("sh", "").replace("sz", "");
        if code.len() == 6 && code.chars().all(|c| c.is_numeric()) {
            let prefix = utils::market_prefix(&code);
            let result = Real::new()
                .get_price(&format!("{}{}", prefix, code))
                .await
                .map_err(|e| DsaError::StockData(format!("搜索行情失败: {}", e)))?;
            Ok(value!({"status": "ok", "data": [result]}))
        } else {
            self.fuzzy_search(&query).await
        }
    }

    async fn fuzzy_search(&self, query: &str) -> DsaResult<Value> {
        let url = format!(
            "https://suggest3.sinajs.cn/suggest/type=11,12,15,21,22,23,24,25,26&key={}",
            urlencoding::encode(query)
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        let resp = client
            .get(&url)
            .header("Referer", "https://finance.sina.com.cn")
            .send()
            .await;

        match resp {
            Ok(response) => {
                let body = response.text().await.unwrap_or_default();
                let results = Self::parse_sina_suggest(&body);
                Ok(value!({"status": "ok", "data": results}))
            }
            Err(e) => {
                tracing::warn!("[股票搜索] 新浪搜索请求失败: {}", e);
                Ok(value!({"status": "ok", "data": []}))
            }
        }
    }

    fn parse_sina_suggest(body: &str) -> Vec<Value> {
        let json_str = if body.starts_with("var suggest_value=\"") && body.ends_with("\";") {
            &body[20..body.len() - 2]
        } else {
            body
        };

        if json_str.is_empty() {
            return vec![];
        }

        let mut results = Vec::new();
        for piece in json_str.split(';') {
            let parts: Vec<&str> = piece.split(',').collect();
            if parts.len() < 10 {
                continue;
            }

            let active = parts.get(8).map(|s| *s).unwrap_or("0");
            if active != "1" {
                continue;
            }

            let type_id = parts.get(1).map(|s| *s).unwrap_or("");
            let is_stock = matches!(type_id, "11" | "12" | "15");
            let is_fund = matches!(type_id, "21" | "22" | "23" | "24" | "25" | "26");

            let raw_code = parts.get(3).map(|s| *s).unwrap_or_default();
            let name = parts.get(4).map(|s| *s).unwrap_or_default();

            if raw_code.is_empty() || name.is_empty() {
                continue;
            }

            let (market, code) = if raw_code.starts_with("sh") || raw_code.starts_with("sz") {
                (&raw_code[..2], &raw_code[2..])
            } else if raw_code.starts_with("bj") {
                ("bj", &raw_code[2..])
            } else {
                ("", raw_code)
            };

            let stock_type = if is_stock { "stock" } else if is_fund { "fund" } else { "other" };

            results.push(value!({
                "code": code,
                "name": name,
                "market": market,
                "type": stock_type,
                "symbol": raw_code,
            }));

            if results.len() >= 15 {
                break;
            }
        }

        results
    }

    async fn get_quote(&self, params: &Value) -> DsaResult<Value> {
        let code = Self::param_code(params);
        let prefix = utils::market_prefix(&code);
        let symbol = format!("{}{}", prefix, code);

        let qq = QQ::new();
        let quote = qq
            .get_realtime_quote(&symbol)
            .await
            .map_err(|e| DsaError::StockData(format!("获取行情失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": quote}))
    }

    async fn get_quotes(&self, params: &Value) -> DsaResult<Value> {
        let codes_val = utils::param_string(params, "codes");
        if codes_val.is_empty() {
            return Ok(value!({"status": "ok", "data": []}));
        }

        let real = Real::new();
        let mut results = Vec::new();
        for code in codes_val.split(',') {
            let code = code.trim();
            if code.len() >= 6 {
                let prefix = utils::market_prefix(code);
                if let Ok(v) = real.get_price(&format!("{}{}", prefix, code)).await {
                    results.push(v);
                }
            }
        }
        Ok(value!({"status": "ok", "data": results}))
    }

    async fn get_kline(&self, params: &Value) -> DsaResult<Value> {
        let code = Self::param_code(params);
        let period = utils::param_string(params, "period");
        let period_str = if period.is_empty() { "daily" } else { &period };
        let start_date = utils::param_string(params, "startDate");
        let end_date = utils::param_string(params, "endDate");
        let adjust = utils::param_string(params, "adjust");
        let adjust_str = if adjust.is_empty() { "qfq" } else { &adjust };

        let em = EastMoney::new();
        let data = em
            .stock_zh_a_hist(
                &code,
                Some(period_str),
                if start_date.is_empty() {
                    None
                } else {
                    Some(start_date.as_str())
                },
                if end_date.is_empty() {
                    None
                } else {
                    Some(end_date.as_str())
                },
                Some(adjust_str),
            )
            .await
            .map_err(|e| DsaError::StockData(format!("获取K线失败: {}", e)))?;

        Ok(value!({"status": "ok", "data": data}))
    }

    async fn get_history(&self, params: &Value) -> DsaResult<Value> {
        let code = Self::param_code(params);
        let days = params.get("days").and_then(|v| v.as_f64()).unwrap_or(120.0) as i64;

        let result = History::get_163_hist(&code, days).await;
        Ok(value!({"status": "ok", "data": result}))
    }

    async fn get_info(&self, params: &Value) -> DsaResult<Value> {
        let code = Self::param_code(params);
        let detail = Stock::get_detail(&code)
            .await
            .map_err(|e| DsaError::StockData(format!("获取股票信息失败: {}", e)))?;
        Ok(value!({"status": "ok", "data": detail}))
    }

    async fn get_watchlist(&self, params: &Value) -> DsaResult<Value> {
        let codes_val = utils::param_string_default(params, "codes", "600519,300750,002594");
        let watchlist: Vec<String> = codes_val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let real = Real::new();
        let mut results = Vec::new();
        for code in &watchlist {
            let prefix = utils::market_prefix(code);
            if let Ok(v) = real.get_price(&format!("{}{}", prefix, code)).await {
                results.push(v);
            }
        }
        Ok(value!({"status": "ok", "data": results}))
    }

    async fn get_spot(&self, _params: &Value) -> DsaResult<Value> {
        let em = EastMoney::new();
        let data = em
            .stock_zh_a_spot()
            .await
            .map_err(|e| DsaError::StockData(format!("获取全市场行情失败: {}", e)))?;
        Ok(value!({"status": "ok", "data": data}))
    }

    async fn get_industries(&self) -> DsaResult<Value> {
        let basic = Basic::new();
        let data = basic
            .get_industry()
            .await
            .map_err(|e| DsaError::StockData(format!("获取行业列表失败: {}", e)))?;
        Ok(value!({"status": "ok", "data": data}))
    }

    async fn get_concepts(&self) -> DsaResult<Value> {
        let basic = Basic::new();
        let data = basic
            .get_concept()
            .await
            .map_err(|e| DsaError::StockData(format!("获取概念列表失败: {}", e)))?;
        Ok(value!({"status": "ok", "data": data}))
    }

    fn param_code(params: &Value) -> String {
        utils::param_string(params, "code")
    }
}
