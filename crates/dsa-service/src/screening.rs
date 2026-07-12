use dsa_core::utils;
use dsa_pipeline::technical::TechnicalAnalyzer;
use qta_crawler::EastMoney;
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct Screening {
    request: RequestParameter,
}

impl Screening {
    pub fn new(param: &RequestParameter) -> Self {
        Screening {
            request: param.clone(),
        }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "status" => self.status().await,
            "strategies" => self.strategies().await,
            "hotspots" => self.hotspots().await,
            "hotspot_detail" => self.hotspot_detail().await,
            "screen" => self.screen().await,
            _ => Err(error!("screening不支持方法: {}", method)),
        }
    }

    async fn status(&self) -> Result<Value> {
        Ok(value!({"enabled": true, "installed": true, "version": "0.2.0"}))
    }

    async fn strategies(&self) -> Result<Value> {
        Ok(value!([
            {"id": "dual_low", "name": "双低策略", "description": "低价+低市盈率筛选"},
            {"id": "breakout", "name": "突破策略", "description": "价格突破均线压力位"},
            {"id": "value", "name": "价值策略", "description": "低PB+高ROE筛选"},
            {"id": "momentum", "name": "动量策略", "description": "近期涨幅领先股票"},
            {"id": "macd_golden_cross", "name": "MACD零上金叉", "description": "股价60日线上+DIF/DEA零上+绿柱缩短金叉"},
        ]))
    }

    async fn hotspots(&self) -> Result<Value> {
        let em = EastMoney::new();
        let industry = em.sector_rank("industry").await
            .map_err(|e| error!("获取行业热点失败: {}", e))?;
        let concept = em.sector_rank("concept").await
            .map_err(|e| error!("获取概念热点失败: {}", e))?;

        let mut all: Vec<Value> = Vec::new();
        all.extend(industry.into_iter().take(10));
        all.extend(concept.into_iter().take(10));

        let mut ranked: Vec<Value> = all;
        ranked.sort_by(|a, b| {
            let ca = a.get("changePercent").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let cb = b.get("changePercent").and_then(|v| v.as_f64()).unwrap_or(0.0);
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked.truncate(12);

        Ok(Value::Array(ranked))
    }

    async fn hotspot_detail(&self) -> Result<Value> {
        let params = self.value();
        let topic_val = utils::param_string(&params, "topic");
        if topic_val.is_empty() {
            return Err(error!("请提供热点主题"));
        }

        let em = EastMoney::new();
        let mut matched: Vec<Value> = Vec::new();

        for sector_type in &["industry", "concept"] {
            if let Ok(sectors) = em.sector_rank(sector_type).await {
                for item in &sectors {
                    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                    if name.contains(topic_val.as_str()) || topic_val.contains(name.as_str()) {
                        matched.push(item.clone());
                    }
                }
            }
        }

        Ok(value!({
            "topic": topic_val.clone(),
            "description": format!("{}相关热门板块", topic_val),
            "sectors": matched,
        }))
    }

    async fn screen(&self) -> Result<Value> {
        let params = self.value();
        let strategy = params
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "dual_low".to_string());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(20.0) as usize;

        if strategy == "macd_golden_cross" {
            return self.filter_macd_golden_cross(limit).await;
        }

        let em = EastMoney::new();
        let spot = em
            .stock_zh_a_spot()
            .await
            .map_err(|e| error!("获取行情失败: {}", e))?;

        let results: Vec<Value> = match strategy.as_str() {
            "dual_low" => Self::filter_dual_low(&spot, limit),
            "breakout" => Self::filter_breakout(&spot, limit),
            "value" => Self::filter_value(&spot, limit),
            "momentum" => Self::filter_momentum(&spot, limit),
            _ => spot.into_iter().take(limit).collect(),
        };
        let count = results.len() as i64;
        Ok(value!({"strategy": strategy, "count": count, "results": results}))
    }

    async fn filter_macd_golden_cross(&self, limit: usize) -> Result<Value> {
        let connector = utils::get_db_connector()
            .map_err(|e| error!("DB连接失败: {}", e))?;

        let sql = "SELECT sd.stock_code, sd.stock_name, sd.close, sd.ma60, sd.dif, sd.dea, sd.macd_hist, \
             sd.trade_date, sd.pct_chg, sd.volume, sd.amount, sd.turnover_rate, sd.volume_ratio \
             FROM stock_daily sd \
             INNER JOIN ( \
                 SELECT stock_code, MAX(trade_date) AS max_date \
                 FROM stock_daily WHERE status >= 1 AND ma60 > 0 \
                 GROUP BY stock_code \
             ) latest ON sd.stock_code = latest.stock_code AND sd.trade_date = latest.max_date \
             WHERE sd.status >= 1 AND sd.ma60 > 0 AND sd.close > sd.ma60 AND sd.dif > 0 AND sd.dea > 0 \
             ORDER BY sd.macd_hist DESC";

        let rows = deck::Helper::query_rows(sql, vec![], &connector)
            .map_err(|e| error!("查询MACD零上金叉候选失败: {}", e))?;

        let analyzer = TechnicalAnalyzer::new();
        let mut results: Vec<Value> = Vec::new();
        let mut checked = 0u32;

        for row in &rows {
            if results.len() >= limit {
                break;
            }
            let code = deck::DataRow::get_string(row, 0);
            let code_val: &str = &code;

            let hist_sql = "SELECT macd_hist FROM stock_daily \
                 WHERE stock_code = :code AND status >= 1 AND macd_hist != 0 \
                 ORDER BY trade_date DESC LIMIT 10";
            let hist_rows = deck::Helper::query_rows(
                hist_sql,
                vec![("code".to_string(), Value::from(code_val.to_string()))],
                &connector,
            )
            .map_err(|e| error!("查询MACD历史失败: {}", e))?;

            if hist_rows.len() < 2 {
                checked += 1;
                continue;
            }

            let hist_series: Vec<f64> = hist_rows.iter()
                .map(|r| deck::DataRow::get_value(r, 0).as_f64().unwrap_or(0.0))
                .collect();
            let mut hist_asc = hist_series;
            hist_asc.reverse();

            if !analyzer.is_macd_golden_cross(&hist_asc, 5) {
                checked += 1;
                continue;
            }

            let name = deck::DataRow::get_string(row, 1);
            let close = deck::DataRow::get_value(row, 2).as_f64().unwrap_or(0.0);
            let ma60 = deck::DataRow::get_value(row, 3).as_f64().unwrap_or(0.0);
            let dif = deck::DataRow::get_value(row, 4).as_f64().unwrap_or(0.0);
            let dea = deck::DataRow::get_value(row, 5).as_f64().unwrap_or(0.0);
            let macd_hist = deck::DataRow::get_value(row, 6).as_f64().unwrap_or(0.0);
            let pct_chg = deck::DataRow::get_value(row, 8).as_f64().unwrap_or(0.0);
            let turnover_rate = deck::DataRow::get_value(row, 11).as_f64().unwrap_or(0.0);
            let volume_ratio = deck::DataRow::get_value(row, 12).as_f64().unwrap_or(0.0);

            let above_ma60_pct = if ma60 > 0.0 { (close - ma60) / ma60 * 100.0 } else { 0.0 };

            results.push(value!({
                "code": code_val,
                "name": name,
                "close": close,
                "ma60": ma60,
                "dif": dif,
                "dea": dea,
                "macd_hist": macd_hist,
                "pct_chg": pct_chg,
                "turnover_rate": turnover_rate,
                "volume_ratio": volume_ratio,
                "above_ma60_pct": (above_ma60_pct * 100.0).round() / 100.0,
                "strategy": "macd_golden_cross",
            }));

            checked += 1;
        }

        Ok(value!({
            "strategy": "macd_golden_cross",
            "count": results.len() as i64,
            "checked": checked as i64,
            "results": results,
        }))
    }

    fn filter_dual_low(stocks: &[Value], limit: usize) -> Vec<Value> {
        stocks
            .iter()
            .filter(|s| {
                let price = s.get("最新价")
                    .and_then(|v| v.as_f64()).unwrap_or(999.0);
                let pe = s.get("市盈率动")
                    .and_then(|v| v.as_f64()).unwrap_or(999.0);
                let code: String = s.get("代码")
                    .and_then(|v| v.as_str()).unwrap_or_default();
                price > 0.0 && price < 20.0 && pe > 0.0 && pe < 30.0
                    && !code.starts_with('8') && !code.starts_with('4')
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn filter_breakout(stocks: &[Value], limit: usize) -> Vec<Value> {
        stocks
            .iter()
            .filter(|s| {
                let change_pct = s.get("涨跌幅")
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                let turnover = s.get("换手率")
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                let volume_ratio = s.get("量比")
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                change_pct > 3.0 && change_pct < 9.8 && turnover > 3.0 && volume_ratio > 2.0
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn filter_value(stocks: &[Value], limit: usize) -> Vec<Value> {
        stocks
            .iter()
            .filter(|s| {
                let pb = s.get("市净率")
                    .and_then(|v| v.as_f64()).unwrap_or(999.0);
                let roe = s.get("加权净资产收益率")
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                let pe = s.get("市盈率动")
                    .and_then(|v| v.as_f64()).unwrap_or(999.0);
                pb > 0.0 && pb < 2.0 && roe > 10.0 && pe > 0.0 && pe < 20.0
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn filter_momentum(stocks: &[Value], limit: usize) -> Vec<Value> {
        let mut ranked: Vec<&Value> = stocks
            .iter()
            .filter(|s| {
                let change_pct = s.get("涨跌幅")
                    .and_then(|v| v.as_f64()).unwrap_or(0.0);
                let code: String = s.get("代码")
                    .and_then(|v| v.as_str()).unwrap_or_default();
                change_pct > 0.0 && !code.starts_with('8') && !code.starts_with('4')
            })
            .collect();
        ranked.sort_by(|a, b| {
            let ca = a.get("涨跌幅")
                .and_then(|v| v.as_f64()).unwrap_or(0.0);
            let cb = b.get("涨跌幅")
                .and_then(|v| v.as_f64()).unwrap_or(0.0);
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked.into_iter().take(limit).cloned().collect()
    }

    fn value(&self) -> Value {
        self.request.value.clone()
    }
}
