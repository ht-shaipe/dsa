use dsa_core::utils;
use deck_mysql::{DataRow, Helper};
use qta_crawler::Real;
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct AlertWorker {
    request: RequestParameter,
}

impl AlertWorker {
    pub fn new(param: &RequestParameter) -> Self {
        AlertWorker { request: param.clone() }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "run" => self.run().await,
            "run_single" => self.run_single().await,
            "indicators" => self.indicators().await,
            "market_light" => self.market_light().await,
            _ => Err(tube::Error::from(format!("alert_worker不支持方法: {}", method))),
        }
    }

    fn params(&self) -> &Value { &self.request.value }

    async fn run(&self) -> Result<Value> {
        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;

        let sql = "SELECT id, stock_code, stock_name, rule_type, condition_json,             enabled, last_triggered_at, trigger_count, alert_type, severity             FROM alert_rules WHERE enabled = 1 AND status >= 1";
        let rules = Helper::query_rows(sql, vec![], &connector)
            .map_err(|e| tube::Error::from(format!("查询告警规则失败: {}", e)))?;

        let mut triggered_count: i64 = 0;
        let real = Real::new();

        for rule_row in &rules {
            let rule_id: i64 = rule_row.get_value(0).as_f64().unwrap_or(0.0) as i64;
            let stock_code = rule_row.get_string(1);
            let condition_json = rule_row.get_string(4);
            let alert_type = rule_row.get_string(8);
            let _severity = rule_row.get_string(9);

            if stock_code.is_empty() {
                continue;
            }

            let prefix = utils::market_prefix(&stock_code);
            let quote = match real.get_price(&format!("{}{}", prefix, stock_code)).await {
                Ok(q) => q,
                Err(_) => continue,
            };

            let current_price = quote.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let change_pct = quote
                .get("changePercent")
                .or_else(|| quote.get("change_pct"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let volume = quote.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let condition: Value = serde_json::from_str(&condition_json).unwrap_or(value!({}));
            let triggered = self.evaluate_condition(&condition, current_price, change_pct, volume);

            if !triggered {
                continue;
            }

            let reason = self.describe_trigger(&condition, current_price, change_pct);

            let trigger_sql = "INSERT INTO alert_triggers                 (rule_id, stock_code, trigger_type, trigger_value, condition_snapshot, notified, status, create_time)                 VALUES (:rid, :code, :ttype, :val, :cond, 1, 1, NOW())";
            let trigger_result = Helper::execute(
                trigger_sql,
                vec![
                    ("rid".to_string(), Value::from(rule_id)),
                    ("code".to_string(), Value::from(stock_code.as_str())),
                    ("ttype".to_string(), Value::from(alert_type.as_str())),
                    ("val".to_string(), Value::from(current_price)),
                    ("cond".to_string(), Value::from(condition_json.as_str())),
                ],
                &connector,
            );

            if let Ok(trigger_id) = trigger_result {
                let _ = Helper::execute(
                    "UPDATE alert_rules SET last_triggered_at = NOW(),                     trigger_count = trigger_count + 1, modify_time = NOW() WHERE id = :id",
                    vec![("id".to_string(), Value::from(rule_id))],
                    &connector,
                );
                let _ = Helper::execute(
                    "INSERT INTO alert_notifications                     (trigger_id, channel, attempt, success, error_code, retryable,                      latency_ms, diagnostics, create_time)                     VALUES (:tid, 'system', 1, 1, '', 0, 0, :diag, NOW())",
                    vec![
                        ("tid".to_string(), Value::from(trigger_id as i64)),
                        ("diag".to_string(), Value::from(reason.as_str())),
                    ],
                    &connector,
                );
            }

            triggered_count += 1;
        }

        Ok(value!({
            "status": "ok",
            "data": {
                "totalRules": rules.len() as i64,
                "triggeredCount": triggered_count,
            }
        }))
    }

    async fn run_single(&self) -> Result<Value> {
        let params = self.params();
        let rule_id = utils::param_i64(params, "ruleId");
        if rule_id == 0 {
            return Err(tube::Error::from("请提供ruleId"));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let sql = "SELECT id, stock_code, stock_name, rule_type, condition_json,             enabled, last_triggered_at, trigger_count, alert_type, severity             FROM alert_rules WHERE id = :id AND status >= 1";
        let rows = Helper::query_rows(
            sql,
            vec![("id".to_string(), Value::from(rule_id))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询告警规则失败: {}", e)))?;

        if rows.is_empty() {
            return Err(tube::Error::from(format!("告警规则不存在: {}", rule_id)));
        }

        let rule_row = &rows[0];
        let stock_code = rule_row.get_string(1);
        let condition_json = rule_row.get_string(4);
        let alert_type = rule_row.get_string(8);

        let real = Real::new();
        let prefix = utils::market_prefix(&stock_code);
        let quote = real
            .get_price(&format!("{}{}", prefix, stock_code))
            .await
            .map_err(|e| tube::Error::from(format!("获取行情失败: {}", e)))?;

        let current_price = quote.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let change_pct = quote
            .get("changePercent")
            .or_else(|| quote.get("change_pct"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let volume = quote.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let condition: Value = serde_json::from_str(&condition_json).unwrap_or(value!({}));
        let triggered = self.evaluate_condition(&condition, current_price, change_pct, volume);

        if !triggered {
            return Ok(value!({
                "status": "ok",
                "data": {
                    "rule_id": rule_id,
                    "triggered": false,
                    "current_price": current_price,
                    "changePct": change_pct,
                }
            }));
        }

        let reason = self.describe_trigger(&condition, current_price, change_pct);

        let trigger_sql = "INSERT INTO alert_triggers             (rule_id, stock_code, trigger_type, trigger_value, condition_snapshot, notified, status, create_time)             VALUES (:rid, :code, :ttype, :val, :cond, 1, 1, NOW())";
        let trigger_result = Helper::execute(
            trigger_sql,
            vec![
                ("rid".to_string(), Value::from(rule_id)),
                ("code".to_string(), Value::from(stock_code.as_str())),
                ("ttype".to_string(), Value::from(alert_type.as_str())),
                ("val".to_string(), Value::from(current_price)),
                ("cond".to_string(), Value::from(condition_json.as_str())),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("创建触发记录失败: {}", e)))?;

        let _ = Helper::execute(
            "UPDATE alert_rules SET last_triggered_at = NOW(), trigger_count = trigger_count + 1,             modify_time = NOW() WHERE id = :id",
            vec![("id".to_string(), Value::from(rule_id))],
            &connector,
        );

        Ok(value!({
            "status": "ok",
            "data": {
                "rule_id": rule_id,
                "triggered": true,
                "triggerId": trigger_result as i64,
                "current_price": current_price,
                "changePct": change_pct,
                "reason": reason,
            }
        }))
    }

    async fn indicators(&self) -> Result<Value> {
        let params = self.params();
        let code = utils::param_string(params, "code");
        if code.is_empty() {
            return Err(tube::Error::from("请提供股票代码"));
        }

        let connector = utils::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))?;
        let real = Real::new();
        let prefix = utils::market_prefix(&code);

        let quote = real
            .get_price(&format!("{}{}", prefix, code))
            .await
            .map_err(|e| tube::Error::from(format!("获取行情失败: {}", e)))?;

        let current_price = quote.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let change_pct = quote
            .get("changePercent")
            .or_else(|| quote.get("change_pct"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let current_volume = quote.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let hist_sql = "SELECT close, high, low, volume             FROM stock_daily WHERE stock_code = :code AND status = 1             ORDER BY trade_date DESC LIMIT 60";
        let hist_rows = Helper::query_rows(
            hist_sql,
            vec![("code".to_string(), Value::from(code.as_str()))],
            &connector,
        )
        .map_err(|e| tube::Error::from(format!("查询K线数据失败: {}", e)))?;

        let closes: Vec<f64> = hist_rows.iter().map(|r| r.get_value(0).as_f64().unwrap_or(0.0)).collect();
        let volumes: Vec<f64> = hist_rows.iter().map(|r| r.get_value(3).as_f64().unwrap_or(0.0)).collect();

        let ma5 = if closes.len() >= 5 { closes[..5].iter().sum::<f64>() / 5.0 } else { 0.0 };
        let ma10 = if closes.len() >= 10 { closes[..10].iter().sum::<f64>() / 10.0 } else { 0.0 };
        let ma20 = if closes.len() >= 20 { closes[..20].iter().sum::<f64>() / 20.0 } else { 0.0 };
        let ma60 = if closes.len() >= 60 { closes[..60].iter().sum::<f64>() / 60.0 } else { 0.0 };

        let ma_position = if current_price > ma5 && current_price > ma20 {
            "above"
        } else if current_price < ma5 && current_price < ma20 {
            "below"
        } else {
            "crossing"
        };

        let (bb_upper, bb_lower, bb_status) = if closes.len() >= 20 {
            let mean = ma20;
            let variance: f64 = closes[..20].iter().map(|&c| (c - mean).powi(2)).sum::<f64>() / 20.0;
            let std_dev = variance.sqrt();
            let upper = mean + 2.0 * std_dev;
            let lower = mean - 2.0 * std_dev;
            let status = if current_price > upper {
                "above_upper"
            } else if current_price < lower {
                "below_lower"
            } else {
                "within"
            };
            (upper, lower, status)
        } else {
            (0.0, 0.0, "insufficient_data")
        };

        let rsi = if closes.len() >= 15 {
            let mut gains = 0.0_f64;
            let mut losses = 0.0_f64;
            for i in 0..14.min(closes.len() - 1) {
                let diff = closes[i] - closes[i + 1];
                if diff > 0.0 { gains += diff; } else { losses += diff.abs(); }
            }
            let avg_gain = gains / 14.0;
            let avg_loss = losses / 14.0;
            if avg_loss == 0.0 { 100.0 } else { 100.0 - (100.0 / (1.0 + avg_gain / avg_loss)) }
        } else {
            50.0
        };

        let (macd_line, signal_line, macd_signal) = if closes.len() >= 26 {
            let ema12 = Self::calc_ema(&closes, 12);
            let ema26 = Self::calc_ema(&closes, 26);
            let dif = ema12 - ema26;
            let dea = dif * 2.0 / 10.0;
            let histogram = (dif - dea) * 2.0;
            let signal = if histogram > 0.0 { "bullish" } else { "bearish" };
            (dif, dea, signal)
        } else {
            (0.0, 0.0, "insufficient_data")
        };

        let avg_volume_5: f64 = if volumes.len() >= 5 {
            volumes[..5].iter().sum::<f64>() / 5.0
        } else {
            1.0
        };
        let volume_ratio = if avg_volume_5 > 0.0 { current_volume / avg_volume_5 } else { 1.0 };

        Ok(value!({
            "status": "ok",
            "data": {
                "code": code,
                "current_price": current_price,
                "changePct": change_pct,
                "volume_ratio": volume_ratio,
                "maPosition": ma_position,
                "ma5": ma5,
                "ma10": ma10,
                "ma20": ma20,
                "ma60": ma60,
                "bollingerBand": {
                    "upper": bb_upper,
                    "lower": bb_lower,
                    "status": bb_status,
                },
                "rsi": rsi,
                "macd": {
                    "dif": macd_line,
                    "dea": signal_line,
                    "signal": macd_signal,
                },
            }
        }))
    }

    async fn market_light(&self) -> Result<Value> {
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

        let up_count = [sh_change, sz_change, cy_change].iter().filter(|&&c| c > 0.0).count();
        let down_count = [sh_change, sz_change, cy_change].iter().filter(|&&c| c < 0.0).count();

        let light = if up_count == 3 {
            "green"
        } else if down_count == 3 {
            "red"
        } else {
            "yellow"
        };

        let severity = if [sh_change, sz_change, cy_change].iter().all(|&c| c < -2.0) {
            "deep_red"
        } else if [sh_change, sz_change, cy_change].iter().all(|&c| c > 2.0) {
            "deep_green"
        } else {
            light
        };

        Ok(value!({
            "status": "ok",
            "data": {
                "light": severity,
                "indices": {
                    "上证指数": { "changePercent": sh_change },
                    "深证成指": { "changePercent": sz_change },
                    "创业板指": { "changePercent": cy_change },
                },
                "upCount": up_count as i64,
                "downCount": down_count as i64,
            }
        }))
    }

    fn evaluate_condition(
        &self,
        condition: &Value,
        current_price: f64,
        change_pct: f64,
        volume: f64,
    ) -> bool {
        let price_above = condition.get("priceAbove").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let price_below = condition.get("priceBelow").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let change_above = condition.get("changeAbove").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let change_below = condition.get("changeBelow").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let volume_above = condition.get("volumeAbove").and_then(|v| v.as_f64()).unwrap_or(0.0);

        if price_above > 0.0 && current_price >= price_above { return true; }
        if price_below > 0.0 && current_price <= price_below { return true; }
        if change_above > 0.0 && change_pct >= change_above { return true; }
        if change_below < 0.0 && change_pct <= change_below { return true; }
        if volume_above > 0.0 && volume >= volume_above { return true; }
        false
    }

    fn describe_trigger(&self, condition: &Value, current_price: f64, change_pct: f64) -> String {
        let mut reasons = Vec::new();
        let price_above = condition.get("priceAbove").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let price_below = condition.get("priceBelow").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let change_above = condition.get("changeAbove").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let change_below = condition.get("changeBelow").and_then(|v| v.as_f64()).unwrap_or(0.0);

        if price_above > 0.0 && current_price >= price_above {
            reasons.push(format!("price {:.2}>={:.2}", current_price, price_above));
        }
        if price_below > 0.0 && current_price <= price_below {
            reasons.push(format!("price {:.2}<={:.2}", current_price, price_below));
        }
        if change_above > 0.0 && change_pct >= change_above {
            reasons.push(format!("change {:.2}%>={:.2}%", change_pct, change_above));
        }
        if change_below < 0.0 && change_pct <= change_below {
            reasons.push(format!("change {:.2}%<={:.2}%", change_pct, change_below));
        }
        reasons.join("; ")
    }

    fn calc_ema(data: &[f64], period: usize) -> f64 {
        if data.len() < period { return 0.0; }
        let k = 2.0 / (period as f64 + 1.0);
        let mut ema = data[..period].iter().sum::<f64>() / period as f64;
        for &val in &data[period..] {
            ema = val * k + ema * (1.0 - k);
        }
        ema
    }
}
