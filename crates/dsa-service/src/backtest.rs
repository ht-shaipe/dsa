use dsa_core::db::{execute, query_rows, row_get_f64, row_get_i64, row_get_string};
use dsa_core::utils;
use tube::{Result, Value};
use tube_web::RequestParameter;

pub struct Backtest {
    request: RequestParameter,
}

impl Backtest {
    pub fn new(param: &RequestParameter) -> Self {
        Backtest { request: param.clone() }
    }

    pub async fn dispatch(&self, method: &str) -> Result<Value> {
        match method {
            "evaluate" => self.evaluate().await,
            "evaluate_batch" => self.evaluate_batch().await,
            "summary" => self.summary().await,
            "detail" => self.detail().await,
            "list" => self.list().await,
            _ => Err(tube::Error::msg(format!(
                "backtest不支持方法: {}",
                method
            ))),
        }
    }

    fn params(&self) -> &Value {
        &self.request.value
    }

    fn connector(&self) -> Result<deck_connector::Connector> {
        dsa_core::db::get_db_connector().map_err(|e| tube::Error::msg(e.to_string()))
    }

    fn is_sqlite(&self) -> bool {
        dsa_core::get_global_config().database.is_sqlite()
    }

    fn now_expr(&self) -> &'static str {
        if self.is_sqlite() { "datetime('now')" } else { "NOW()" }
    }

    fn date_sub_days_expr(&self, days: i64) -> String {
        if self.is_sqlite() {
            let now = chrono::Local::now();
            let target = now - chrono::Duration::days(days);
            format!("'{}'", target.format("%Y-%m-%d"))
        } else {
            format!("DATE_SUB(CURDATE(), INTERVAL {} DAY)", days)
        }
    }

    async fn evaluate(&self) -> Result<Value> {
        let params = self.params();
        let signal_id = utils::param_i64(params, "signalId");
        if signal_id == 0 {
            return Err(tube::Error::msg("请提供signalId".to_string()));
        }

        let connector = self.connector()?;
        let conf = dsa_core::get_global_config();
        let eval_window = conf.backtest.eval_window_days as i64;
        let neutral_band = conf.backtest.neutral_band_pct;

        // 检查是否已有评估结果 (避免重复)
        let check_sql = "SELECT id FROM backtest_results \
             WHERE analysis_id = (SELECT analysis_id FROM decision_signals WHERE id = :sid) \
             AND stock_code = (SELECT stock_code FROM decision_signals WHERE id = :sid) \
             AND signal_date = (SELECT signal_date FROM decision_signals WHERE id = :sid) \
             AND eval_window_days = :ew AND status >= 1 LIMIT 1";
        let existing = query_rows(
            check_sql,
            vec![
                ("sid".to_string(), Value::from(signal_id)),
                ("ew".to_string(), Value::from(eval_window)),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("检查已有评估失败: {}", e)))?;

        if !existing.is_empty() {
            let exist_id: i64 = row_get_i64(&existing[0], "id");
            return Ok(value!({
                "id": exist_id,
                "signalId": signal_id,
                "duplicate": true,
                "message": "评估结果已存在",
            }));
        }

        let sql = "SELECT id, stock_code, stock_name, signal_date, action, \
             entry_price, stop_loss, target_price, confidence_level, \
             sentiment_score, reasoning, scope_type, analysis_id, status \
             FROM decision_signals WHERE id = :id AND status >= 1";
        let rows = query_rows(
            sql,
            vec![("id".to_string(), Value::from(signal_id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询决策信号失败: {}", e)))?;

        if rows.is_empty() {
            return Err(tube::Error::msg(format!(
                "决策信号不存在: {}",
                signal_id
            )));
        }

        let row = &rows[0];
        let stock_code = row_get_string(row, "stockCode");
        let stock_name = row_get_string(row, "stockName");
        let signal_date_str = row_get_string(row, "signalDate");
        let action = row_get_string(row, "action");
        let entry_price: f64 = row_get_f64(row, "entryPrice");
        let stop_loss: f64 = row_get_f64(row, "stopLoss");
        let target_price: f64 = row_get_f64(row, "targetPrice");
        let analysis_id: i64 = row_get_f64(row, "analysisId") as i64;

        // signal_date 是 datetime 格式, 只取日期部分用于K线查询
        let signal_date_only = signal_date_str.split(' ').next().unwrap_or(&signal_date_str);

        let hist_sql = "SELECT trade_date, open, high, low, close, volume \
             FROM stock_daily \
             WHERE stock_code = :code AND DATE(trade_date) >= :sdate AND status = 1 \
             ORDER BY trade_date ASC LIMIT :limit";
        let hist_rows = query_rows(
            hist_sql,
            vec![
                ("code".to_string(), Value::from(stock_code.as_str())),
                ("sdate".to_string(), Value::from(signal_date_only)),
                ("limit".to_string(), Value::from(eval_window + 5)),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询K线数据失败: {}", e)))?;

        let eval_rows: Vec<_> = if hist_rows.len() >= eval_window as usize {
            hist_rows
        } else {
            let fallback_sql = "SELECT trade_date, open, high, low, close, volume \
                 FROM stock_daily \
                 WHERE stock_code = :code AND DATE(trade_date) < :sdate AND status = 1 \
                 ORDER BY trade_date DESC LIMIT :limit";
            let fallback_rows = query_rows(
                fallback_sql,
                vec![
                    ("code".to_string(), Value::from(stock_code.as_str())),
                    ("sdate".to_string(), Value::from(signal_date_only)),
                    ("limit".to_string(), Value::from(eval_window + 5)),
                ],
                &connector,
            )
            .map_err(|e| tube::Error::msg(format!("查询回退K线数据失败: {}", e)))?;
            if fallback_rows.len() < eval_window as usize {
                return Err(tube::Error::msg(format!(
                    "无足够K线数据: code={}, date={}, 需{}天, 仅有{}天",
                    stock_code, signal_date_only, eval_window, hist_rows.len() + fallback_rows.len()
                )));
            }
            let mut reversed = fallback_rows;
            reversed.reverse();
            reversed
        };

        let start_price: f64 = row_get_f64(&eval_rows[0], "open");
        let eval_closes: Vec<f64> = eval_rows
            .iter()
            .take(eval_window as usize)
            .map(|r| row_get_f64(r, "close"))
            .collect();
        let eval_highs: Vec<f64> = eval_rows
            .iter()
            .take(eval_window as usize)
            .map(|r| row_get_f64(r, "high"))
            .collect();
        let eval_lows: Vec<f64> = eval_rows
            .iter()
            .take(eval_window as usize)
            .map(|r| row_get_f64(r, "low"))
            .collect();

        let end_close = eval_closes.last().copied().unwrap_or(start_price);
        let max_high = eval_highs.iter().cloned().fold(0.0_f64, f64::max);
        let min_low = eval_lows.iter().cloned().fold(f64::MAX, f64::min);
        let stock_return_pct = if start_price > 0.0 {
            (end_close - start_price) / start_price * 100.0
        } else {
            0.0
        };

        let direction_expected = match action.as_str() {
            "buy" | "add" | "hold" | "watch" => "up",
            "sell" | "reduce" | "avoid" => "down",
            _ => "neutral",
        };

        let outcome = if direction_expected == "up" {
            if stock_return_pct > neutral_band {
                "win"
            } else if stock_return_pct < -neutral_band {
                "loss"
            } else {
                "neutral"
            }
        } else if direction_expected == "down" {
            if stock_return_pct < -neutral_band {
                "win"
            } else if stock_return_pct > neutral_band {
                "loss"
            } else {
                "neutral"
            }
        } else {
            "neutral"
        };

        let hit_stop_loss = stop_loss > 0.0 && eval_lows.iter().any(|&l| l <= stop_loss);
        let hit_take_profit = target_price > 0.0 && eval_highs.iter().any(|&h| h >= target_price);

        let mut max_drawdown = 0.0_f64;
        let mut peak = start_price;
        for &c in &eval_closes {
            if c > peak {
                peak = c;
            }
            let dd = (peak - c) / peak * 100.0;
            if dd > max_drawdown {
                max_drawdown = dd;
            }
        }

        let direction_correct = (direction_expected == "up" && stock_return_pct > 0.0)
            || (direction_expected == "down" && stock_return_pct < 0.0)
            || direction_expected == "neutral";

        let now = self.now_expr();
        let insert_sql = format!("INSERT INTO backtest_results \
             (analysis_id, stock_code, signal_date, decision_action, simulated_entry, simulated_exit, \
              return_pct, max_drawdown, direction_correct, scope_type, status, create_time, \
              eval_window_days, eval_status, evaluated_at, start_price, end_close, max_high, min_low, \
              stock_return_pct, direction_expected, outcome, stop_loss_price, take_profit_price, \
              hit_stop_loss, hit_take_profit, operation_advice) \
             VALUES (:aid, :code, :sdate, :action, :entry, :exit, \
              :ret, :dd, :dir_correct, 'watchlist', 1, {now}, \
              :ew, 'completed', {now}, :sp, :ec, :mh, :ml, \
              :sr, :de, :outcome, :sl, :tp, \
              :hit_sl, :hit_tp, :oa)");

        let result = execute(
            &insert_sql,
            vec![
                ("aid".to_string(), Value::from(analysis_id)),
                ("code".to_string(), Value::from(stock_code.as_str())),
                ("sdate".to_string(), Value::from(signal_date_only)),
                ("action".to_string(), Value::from(action.as_str())),
                (
                    "entry".to_string(),
                    Value::from(if entry_price > 0.0 { entry_price } else { start_price }),
                ),
                ("exit".to_string(), Value::from(end_close)),
                ("ret".to_string(), Value::from(stock_return_pct)),
                ("dd".to_string(), Value::from(max_drawdown)),
                ("dir_correct".to_string(), Value::from(direction_correct)),
                ("ew".to_string(), Value::from(eval_window as i64)),
                ("sp".to_string(), Value::from(start_price)),
                ("ec".to_string(), Value::from(end_close)),
                ("mh".to_string(), Value::from(max_high)),
                ("ml".to_string(), Value::from(min_low)),
                ("sr".to_string(), Value::from(stock_return_pct)),
                ("de".to_string(), Value::from(direction_expected)),
                ("outcome".to_string(), Value::from(outcome)),
                ("sl".to_string(), Value::from(stop_loss)),
                ("tp".to_string(), Value::from(target_price)),
                ("hit_sl".to_string(), Value::from(hit_stop_loss as i8)),
                ("hit_tp".to_string(), Value::from(hit_take_profit as i8)),
                ("oa".to_string(), Value::from(action.as_str())),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("保存回测结果失败: {}", e)))?;

        let now = self.now_expr();
        let _ = execute(
            &format!("UPDATE decision_signals SET status = 4, modify_time = {} WHERE id = :id", now),
            vec![("id".to_string(), Value::from(signal_id))],
            &connector,
        );

        Ok(value!({
            "id": result as i64,
            "signalId": signal_id,
            "stockCode": stock_code,
            "stockName": stock_name,
            "directionExpected": direction_expected,
            "outcome": outcome,
            "startPrice": start_price,
            "endClose": end_close,
            "maxHigh": max_high,
            "minLow": min_low,
            "stockReturnPct": stock_return_pct,
            "maxDrawdown": max_drawdown,
            "directionCorrect": direction_correct,
            "hitStopLoss": hit_stop_loss,
            "hitTakeProfit": hit_take_profit,
            "evalWindowDays": eval_window,
        }))
    }

    async fn evaluate_batch(&self) -> Result<Value> {
        let params = self.params();
        let limit = utils::param_i64(params, "limit").max(1).min(100) as i64;
        let connector = self.connector()?;
        let conf = dsa_core::get_global_config();
        let eval_window = conf.backtest.eval_window_days;

        let date_sub = self.date_sub_days_expr(eval_window as i64);
        let sql = format!("SELECT id FROM decision_signals \
             WHERE status = 1 AND signal_date < {} \
             ORDER BY signal_date ASC LIMIT :limit", date_sub);
        let rows = query_rows(
            &sql,
            vec![
                ("limit".to_string(), Value::from(limit)),
            ],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询待评估信号失败: {}", e)))?;

        let mut evaluated_count: i64 = 0;
        let mut errors: Vec<Value> = Vec::new();

        for row in &rows {
            let sid: i64 = row_get_i64(row, "id");
            let eval_params = value!({"signalId": sid});
            let eval_req = {
                let mut r = self.request.clone();
                r.value = eval_params;
                r
            };
            let eval_svc = Backtest::new(&eval_req);
            match eval_svc.evaluate().await {
                Ok(_) => {
                    evaluated_count += 1;
                }
                Err(e) => {
                    errors.push(value!({"signalId": sid, "error": e.to_string()}));
                }
            }
        }

        Ok(value!({
            "evaluatedCount": evaluated_count,
            "totalPending": rows.len() as i64,
            "errors": errors,
        }))
    }

    async fn summary(&self) -> Result<Value> {
        let connector = self.connector()?;
        let params = self.params();
        let code = utils::param_string(params, "code");
        let horizon = utils::param_string(params, "horizon");

        let mut conditions = vec!["status = 1".to_string()];
        let mut p: Vec<(String, Value)> = Vec::new();

        if !code.is_empty() {
            conditions.push("stock_code = :code".to_string());
            p.push(("code".to_string(), Value::from(code.as_str())));
        }
        if !horizon.is_empty() {
            conditions.push("eval_window_days = :ew".to_string());
            let ew: i64 = match horizon.as_str() {
                "short" => 5,
                "medium" => 10,
                "long" => 20,
                _ => horizon.parse().unwrap_or(10),
            };
            p.push(("ew".to_string(), Value::from(ew)));
        }

        let where_clause = conditions.join(" AND ");

        let sql = format!(
            "SELECT COUNT(*) as total, \
             SUM(CASE WHEN outcome = 'win' THEN 1 ELSE 0 END) as wins, \
             SUM(CASE WHEN outcome = 'loss' THEN 1 ELSE 0 END) as losses, \
             SUM(CASE WHEN outcome = 'neutral' THEN 1 ELSE 0 END) as neutrals, \
             AVG(stock_return_pct) as avg_return, \
             AVG(CASE WHEN outcome = 'win' THEN stock_return_pct ELSE NULL END) as avg_win, \
             AVG(CASE WHEN outcome = 'loss' THEN stock_return_pct ELSE NULL END) as avg_loss, \
             MAX(max_drawdown) as max_drawdown, \
             SUM(stock_return_pct * stock_return_pct) as sum_sq_return, \
             SUM(stock_return_pct) as sum_return \
             FROM backtest_results WHERE {}",
            where_clause
        );

        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| tube::Error::msg(format!("查询回测统计失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(value!({
                "totalTrades": 0,
                "winRate": 0.0,
                "avgReturn": 0.0,
                "sharpeRatio": 0.0,
                "maxDrawdown": 0.0,
                "wins": 0,
                "losses": 0,
                "neutrals": 0,
            }));
        }

        let r = &rows[0];
        let total: f64 = row_get_f64(r, "total");
        let wins: f64 = row_get_f64(r, "wins");
        let losses: f64 = row_get_f64(r, "losses");
        let neutrals: f64 = row_get_f64(r, "neutrals");
        let avg_return: f64 = row_get_f64(r, "avgReturn");
        let _avg_win: f64 = row_get_f64(r, "avgWin");
        let _avg_loss: f64 = row_get_f64(r, "avgLoss");
        let max_dd: f64 = row_get_f64(r, "maxDrawdown");
        let sum_sq: f64 = row_get_f64(r, "sumSqReturn");
        let sum_ret: f64 = row_get_f64(r, "sumReturn");
        let variance = if total > 1.0 {
            (sum_sq - sum_ret * sum_ret / total) / total
        } else {
            0.0
        };
        let std_return: f64 = if variance > 0.0 { variance.sqrt() } else { 0.0 };

        let win_rate = if total > 0.0 { wins / total * 100.0 } else { 0.0 };
        let sharpe_ratio = if std_return > 0.0 {
            avg_return / std_return
        } else {
            0.0
        };

        Ok(value!({
            "totalTrades": total as i64,
            "winRate": win_rate,
            "avgReturn": avg_return,
            "sharpeRatio": sharpe_ratio,
            "maxDrawdown": max_dd,
            "wins": wins as i64,
            "losses": losses as i64,
            "neutrals": neutrals as i64,
        }))
    }

    async fn detail(&self) -> Result<Value> {
        let params = self.params();
        let id = utils::param_i64(params, "id");
        if id == 0 {
            return Err(tube::Error::msg("请提供回测ID".to_string()));
        }

        let connector = self.connector()?;
        let sql = "SELECT id, analysis_id, stock_code, signal_date, decision_action, \
             simulated_entry, simulated_exit, exit_date, return_pct, max_drawdown, \
             direction_correct, scope_type, status, create_time, \
             eval_window_days, eval_status, evaluated_at, start_price, end_close, \
             max_high, min_low, stock_return_pct, direction_expected, outcome, \
             stop_loss_price, take_profit_price, hit_stop_loss, hit_take_profit \
             FROM backtest_results WHERE id = :id AND status >= 1";
        let rows = query_rows(
            sql,
            vec![("id".to_string(), Value::from(id))],
            &connector,
        )
        .map_err(|e| tube::Error::msg(format!("查询回测详情失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(Value::Null);
        }

        Ok(rows[0].clone())
    }

    async fn list(&self) -> Result<Value> {
        let connector = self.connector()?;
        let params = self.params();
        let code = utils::param_string(params, "code");
        let outcome = utils::param_string(params, "outcome");
        let limit = utils::param_i64(params, "limit").max(1).min(200) as i64;
        let offset = utils::param_i64(params, "offset").max(0) as i64;

        let mut conditions = vec!["status >= 1".to_string()];
        let mut p: Vec<(String, Value)> = Vec::new();

        if !code.is_empty() {
            conditions.push("stock_code = :code".to_string());
            p.push(("code".to_string(), Value::from(code.as_str())));
        }
        if !outcome.is_empty() {
            conditions.push("outcome = :outcome".to_string());
            p.push(("outcome".to_string(), Value::from(outcome.as_str())));
        }

        let where_clause = conditions.join(" AND ");

        let sql = format!(
            "SELECT id, analysis_id, stock_code, signal_date, decision_action, \
             simulated_entry, simulated_exit, return_pct, max_drawdown, direction_correct, \
             scope_type, status, create_time, eval_window_days, eval_status, \
             start_price, end_close, max_high, min_low, stock_return_pct, \
             direction_expected, outcome \
             FROM backtest_results WHERE {} ORDER BY create_time DESC LIMIT :limit OFFSET :offset",
            where_clause
        );
        p.push(("limit".to_string(), Value::from(limit)));
        p.push(("offset".to_string(), Value::from(offset)));

        let rows = query_rows(&sql, p, &connector)
            .map_err(|e| tube::Error::msg(format!("查询回测列表失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.clone()).collect();
        Ok(Value::Array(results))
    }
}
