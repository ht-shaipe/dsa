use dsa_core::db::{query_rows, execute, row_get_string, row_get_f64};
use dsa_core::utils;
use dsa_pipeline::technical::TechnicalAnalyzer;
use deck_connector::Connector;
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct Indicator {
    request: RequestParameter,
}

impl Indicator {
    pub fn new(param: &RequestParameter) -> Self {
        Indicator { request: param.clone() }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "calc_all" => self.calc_all().await,
            "calc_stock" => self.calc_stock().await,
            "status" => self.status().await,
            _ => Err(error!("indicator不支持方法: {}", method)),
        }
    }

    fn params(&self) -> &Value {
        &self.request.value
    }

    async fn status(&self) -> Result<Value> {
        Ok(value!({"enabled": true, "version": "0.1.0"}))
    }

    async fn calc_all(&self) -> Result<Value> {
        let connector = utils::get_db_connector()
            .map_err(|e| error!("DB连接失败: {}", e))?;

        let sql = "SELECT DISTINCT stock_code, stock_name FROM stock_daily WHERE status = 1 ORDER BY stock_code";
        let rows = query_rows(sql, vec![], &connector)
            .map_err(|e| error!("查询股票列表失败: {}", e))?;

        let mut success = 0u32;
        let mut failed = 0u32;
        let total = rows.len();

        for row in &rows {
            let code = row_get_string(row, "stockCode");
            if code.is_empty() {
                continue;
            }
            match Self::calc_for_stock(&connector, &code).await {
                Ok(_) => success += 1,
                Err(e) => {
                    tracing::warn!("计算指标失败 {}: {}", code, e);
                    failed += 1;
                }
            }
        }

        Ok(value!({
            "total": total as i64,
            "success": success as i64,
            "failed": failed as i64,
        }))
    }

    async fn calc_stock(&self) -> Result<Value> {
        let connector = utils::get_db_connector()
            .map_err(|e| error!("DB连接失败: {}", e))?;
        let code = utils::param_string(self.params(), "code");
        if code.is_empty() {
            return Err(error!("请提供股票代码(code参数)"));
        }
        Self::calc_for_stock(&connector, &code).await
    }

    async fn calc_for_stock(connector: &Connector, code: &str) -> Result<Value> {
        let sql = "SELECT close, volume, amount, trade_date, stock_name, open, high, low, pct_chg, \
             volume_ratio, turnover_rate, status \
             FROM stock_daily WHERE stock_code = :code AND status >= 1 \
             ORDER BY trade_date ASC LIMIT 120";
        let rows = query_rows(
            sql,
            vec![("code".to_string(), Value::from(code.to_string()))],
            connector,
        )
        .map_err(|e| error!("查询K线数据失败 {}: {}", code, e))?;

        if rows.len() < 26 {
            return Ok(value!({"code": code, "skipped": true, "reason": "数据不足26天"}));
        }

        let closes: Vec<f64> = rows.iter().map(|r| row_get_f64(r, "close")).collect();
        let analyzer = TechnicalAnalyzer::new();

        let macd_pts = analyzer.macd_series(&closes, 12, 26, 9);
        let offset = closes.len() - macd_pts.len();
        let start_idx = 60usize.saturating_sub(offset);

        let mut updated = 0u32;
        for (i, pt) in macd_pts.iter().enumerate() {
            let bar_idx = offset + i;
            let ma60 = if bar_idx + 1 >= 60 {
                let s: f64 = closes[bar_idx + 1 - 60..=bar_idx].iter().sum();
                s / 60.0
            } else {
                0.0
            };
            let ma5 = if bar_idx + 1 >= 5 {
                let s: f64 = closes[bar_idx + 1 - 5..=bar_idx].iter().sum();
                s / 5.0
            } else {
                0.0
            };
            let ma10 = if bar_idx + 1 >= 10 {
                let s: f64 = closes[bar_idx + 1 - 10..=bar_idx].iter().sum();
                s / 10.0
            } else {
                0.0
            };
            let ma20 = if bar_idx + 1 >= 20 {
                let s: f64 = closes[bar_idx + 1 - 20..=bar_idx].iter().sum();
                s / 20.0
            } else {
                0.0
            };

            if i < start_idx {
                continue;
            }

            let row = &rows[bar_idx];
            let date_str = row_get_string(row, "tradeDate");
            if date_str.is_empty() {
                continue;
            }

            let update_sql = "UPDATE stock_daily SET ma5 = :ma5, ma10 = :ma10, ma20 = :ma20, ma60 = :ma60, \
                 dif = :dif, dea = :dea, macd_hist = :macd_hist \
                 WHERE stock_code = :code AND trade_date = :date AND status >= 1";
            let res = execute(
                update_sql,
                vec![
                    ("ma5".to_string(), Value::from(ma5)),
                    ("ma10".to_string(), Value::from(ma10)),
                    ("ma20".to_string(), Value::from(ma20)),
                    ("ma60".to_string(), Value::from(ma60)),
                    ("dif".to_string(), Value::from(pt.dif)),
                    ("dea".to_string(), Value::from(pt.dea)),
                    ("macd_hist".to_string(), Value::from(pt.hist)),
                    ("code".to_string(), Value::from(code.to_string())),
                    ("date".to_string(), Value::from(date_str)),
                ],
                connector,
            );
            if let Ok(affected) = res {
                if affected > 0 {
                    updated += 1;
                }
            }
        }

        Ok(value!({"code": code, "updated": updated as i64}))
    }
}
