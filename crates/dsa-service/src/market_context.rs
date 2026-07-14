use dsa_core::db::{query_rows, row_get_f64};
use dsa_core::utils;
use qta_crawler::Real;
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct MarketContext {
    request: RequestParameter,
}

impl MarketContext {
    pub fn new(param: &RequestParameter) -> Self {
        MarketContext { request: param.clone() }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "build" => self.build().await,
            "phase" => self.phase().await,
            "guardrail" => self.guardrail().await,
            _ => Err(tube::Error::from(format!("market_context不支持方法: {}", method))),
        }
    }

    fn params(&self) -> &Value { &self.request.value }

    async fn build(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(tube::Error::from("请提供股票代码"));
        }

        let real = Real::new();

        let sh = real.get_price("sh000001").await.ok();
        let sz = real.get_price("sz399001").await.ok();
        let cy = real.get_price("sz399006").await.ok();

        let sh_change = sh.as_ref()
            .and_then(|v| v.get("changePercent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);
        let sz_change = sz.as_ref()
            .and_then(|v| v.get("changePercent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);
        let cy_change = cy.as_ref()
            .and_then(|v| v.get("changePercent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);

        let prefix = utils::market_prefix(&code);
        let quote = real
            .get_price(&format!("{}{}", prefix, code))
            .await
            .map_err(|e| tube::Error::from(format!("获取行情失败: {}", e)))?;

        let stock_price = quote.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let stock_change = quote
            .get("changePercent")
            .or_else(|| quote.get("change_pct"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let market_phase = self.determine_phase_from_changes(sh_change, sz_change, cy_change);

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sector_sql = "SELECT stock_code, stock_name, close, pct_chg             FROM stock_daily             WHERE trade_date = (SELECT MAX(trade_date) FROM stock_daily WHERE status = 1)             AND stock_code LIKE :sector_prefix AND status = 1             ORDER BY pct_chg DESC LIMIT 10";
        let sector_prefix = if code.starts_with('6') { "6%" } else if code.starts_with('0') || code.starts_with('3') { "0%" } else { "%" };
        let sector_rows = query_rows(
            sector_sql,
            vec![("sector_prefix".to_string(), Value::from(sector_prefix))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询行业数据失败: {}", e)))?;

        let sector_performance: Vec<Value> = sector_rows;

        Ok(value!({
            "code": code,
            "stockPrice": stock_price,
            "stockChangePct": stock_change,
            "marketPhase": market_phase,
            "indices": {
                "上证指数": {"changePercent": sh_change},
                "深证成指": {"changePercent": sz_change},
                "创业板指": {"changePercent": cy_change},
            },
            "sectorPerformance": sector_performance,
        }))
    }

    async fn phase(&self) -> Result<Value> {
        let params = self.params();
        let index_code_raw = utils::param_string(params, "indexCode");
        let index_code = if index_code_raw.is_empty() { "000001".to_string() } else { index_code_raw };

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT close FROM stock_daily             WHERE stock_code = :code AND status = 1             ORDER BY trade_date DESC LIMIT 60";
        let rows = query_rows(
            sql,
            vec![("code".to_string(), Value::from(index_code.as_str()))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询指数K线失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(value!({
                "phase": "unknown",
                "mas": {}
            }));
        }

        let closes: Vec<f64> = rows.iter().map(|r| row_get_f64(r, "close")).collect();

        let ma5 = if closes.len() >= 5 { closes[..5].iter().sum::<f64>() / 5.0 } else { 0.0 };
        let ma10 = if closes.len() >= 10 { closes[..10].iter().sum::<f64>() / 10.0 } else { 0.0 };
        let ma20 = if closes.len() >= 20 { closes[..20].iter().sum::<f64>() / 20.0 } else { 0.0 };
        let ma60 = if closes.len() >= 60 { closes[..60].iter().sum::<f64>() / 60.0 } else { 0.0 };

        let current = closes.first().copied().unwrap_or(0.0);

        let phase = if ma5 > ma10 && ma10 > ma20 && (ma20 > ma60 || ma60 == 0.0) {
            "bull"
        } else if ma5 < ma10 && ma10 < ma20 && (ma20 < ma60 || ma60 == 0.0) {
            "bear"
        } else if current > ma20 {
            "bull_leaning"
        } else if current < ma20 {
            "bear_leaning"
        } else {
            "sideways"
        };

        Ok(value!({
            "indexCode": index_code,
            "currentPrice": current,
            "phase": phase,
            "mas": {
                "ma5": ma5,
                "ma10": ma10,
                "ma20": ma20,
                "ma60": ma60,
            },
        }))
    }

    async fn guardrail(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        let action = utils::param_string(params, "action");
        if code.is_empty() || action.is_empty() {
            return Err(tube::Error::from("请提供code和action"));
        }

        let real = Real::new();

        let sh = real.get_price("sh000001").await.ok();
        let sz = real.get_price("sz399001").await.ok();
        let cy = real.get_price("sz399006").await.ok();

        let sh_change = sh.as_ref()
            .and_then(|v| v.get("changePercent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);
        let sz_change = sz.as_ref()
            .and_then(|v| v.get("changePercent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);
        let cy_change = cy.as_ref()
            .and_then(|v| v.get("changePercent").and_then(|p| p.as_f64()))
            .unwrap_or(0.0);

        let market_phase = self.determine_phase_from_changes(sh_change, sz_change, cy_change);

        let (allowed, reason, severity) = match action.as_str() {
            "buy" | "add" => {
                if market_phase == "extreme_bear" {
                    (false, "极端熊市，不建议买入".to_string(), "critical")
                } else if market_phase == "bear" {
                    (true, "熊市中买入需谨慎，建议减仓".to_string(), "warning")
                } else {
                    (true, "市场条件允许买入".to_string(), "info")
                }
            }
            "sell" | "reduce" => {
                if market_phase == "extreme_bull" {
                    (false, "极端牛市，不建议卖出".to_string(), "critical")
                } else if market_phase == "bull" {
                    (true, "牛市中卖出需谨慎，可能错过后续涨幅".to_string(), "warning")
                } else {
                    (true, "市场条件允许卖出".to_string(), "info")
                }
            }
            _ => (true, "无特殊限制".to_string(), "info"),
        };

        Ok(value!({
            "code": code,
            "action": action,
            "allowed": allowed,
            "reason": reason,
            "severity": severity,
            "marketPhase": market_phase,
            "indices": {
                "上证指数": {"changePercent": sh_change},
                "深证成指": {"changePercent": sz_change},
                "创业板指": {"changePercent": cy_change},
            },
        }))
    }

    fn determine_phase_from_changes(&self, sh: f64, sz: f64, cy: f64) -> &'static str {
        let all_up = sh > 0.0 && sz > 0.0 && cy > 0.0;
        let all_down = sh < 0.0 && sz < 0.0 && cy < 0.0;
        let all_extreme_up = sh > 2.0 && sz > 2.0 && cy > 2.0;
        let all_extreme_down = sh < -2.0 && sz < -2.0 && cy < -2.0;

        if all_extreme_down { "extreme_bear" }
        else if all_extreme_up { "extreme_bull" }
        else if all_down { "bear" }
        else if all_up { "bull" }
        else { "sideways" }
    }
}
