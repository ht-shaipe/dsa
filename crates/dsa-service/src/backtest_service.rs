//! 回测服务 - 信号回测评估/批量评估/统计摘要

use dsa_core::{DsaError, DsaResult, utils};
use deck_mysql::{DataRow, Helper};
use tube::Value;

/// 回测服务
pub struct BacktestService;

impl BacktestService {
    /// 创建回测服务实例
    pub fn new() -> Self {
        Self
    }

    /// 请求分发 - 可用方法: evaluate, evaluate_batch, summary, detail, list
    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "evaluate" => self.evaluate(params).await,
            "evaluate_batch" => self.evaluate_batch(params).await,
            "summary" => self.summary(params).await,
            "detail" => self.detail(params).await,
            "list" => self.list(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "backtest_service不支持方法: {}",
                method
            ))),
        }
    }

    /// 评估单个决策信号的回测结果
    async fn evaluate(&self, params: &Value) -> DsaResult<Value> {
        let signal_id = utils::param_i64(params, "signalId");
        if signal_id == 0 {
            return Err(DsaError::Validation("请提供signalId".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let conf = dsa_core::get_global_config();
        let eval_window = conf.backtest.eval_window_days as i64;
        let neutral_band = conf.backtest.neutral_band_pct;

        // 查询决策信号
        let sql = "SELECT id, stock_code, stock_name, signal_date, action, \
             entry_price, stop_loss, target_price, confidence_level, \
             sentiment_score, reasoning, scope_type, analysis_id, status \
             FROM decision_signals WHERE id = :id AND status >= 1";
        let rows = Helper::query_rows(
            sql,
            vec![("id".to_string(), Value::from(signal_id))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询决策信号失败: {}", e)))?;

        if rows.is_empty() {
            return Err(DsaError::Validation(format!(
                "决策信号不存在: {}",
                signal_id
            )));
        }

        let row = &rows[0];
        let stock_code = row.get_string(1);
        let stock_name = row.get_string(2);
        let signal_date = row.get_string(3);
        let action = row.get_string(4);
        let entry_price: f64 = row.get_value(5).as_f64().unwrap_or(0.0);
        let stop_loss: f64 = row.get_value(6).as_f64().unwrap_or(0.0);
        let target_price: f64 = row.get_value(7).as_f64().unwrap_or(0.0);
        let analysis_id: i64 = row.get_value(12).as_f64().unwrap_or(0.0) as i64;

        // 查询信号日期之后的stock_daily数据
        let hist_sql = "SELECT trade_date, open, high, low, close, volume \
             FROM stock_daily \
             WHERE stock_code = :code AND trade_date >= :sdate AND status = 1 \
             ORDER BY trade_date ASC LIMIT :limit";
        let hist_rows = Helper::query_rows(
            hist_sql,
            vec![
                ("code".to_string(), Value::from(stock_code.as_str())),
                ("sdate".to_string(), Value::from(signal_date.as_str())),
                ("limit".to_string(), Value::from(eval_window + 5)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询K线数据失败: {}", e)))?;

        if hist_rows.is_empty() {
            return Err(DsaError::Backtest(format!(
                "无K线数据: code={}, date={}",
                stock_code, signal_date
            )));
        }

        // 计算回测指标
        let start_price: f64 = hist_rows[0].get_value(1).as_f64().unwrap_or(0.0); // open of first day
        let eval_closes: Vec<f64> = hist_rows
            .iter()
            .take(eval_window as usize)
            .map(|r| r.get_value(4).as_f64().unwrap_or(0.0)) // close
            .collect();
        let eval_highs: Vec<f64> = hist_rows
            .iter()
            .take(eval_window as usize)
            .map(|r| r.get_value(2).as_f64().unwrap_or(0.0)) // high
            .collect();
        let eval_lows: Vec<f64> = hist_rows
            .iter()
            .take(eval_window as usize)
            .map(|r| r.get_value(3).as_f64().unwrap_or(0.0)) // low
            .collect();

        let end_close = eval_closes.last().copied().unwrap_or(start_price);
        let max_high = eval_highs.iter().cloned().fold(0.0_f64, f64::max);
        let min_low = eval_lows.iter().cloned().fold(f64::MAX, f64::min);
        let stock_return_pct = if start_price > 0.0 {
            (end_close - start_price) / start_price * 100.0
        } else {
            0.0
        };

        // 判断方向预期
        let direction_expected = match action.as_str() {
            "buy" | "add" | "hold" | "watch" => "up",
            "sell" | "reduce" | "avoid" => "down",
            _ => "neutral",
        };

        // 判断结果: win/loss/neutral
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

        // 检查止损止盈
        let hit_stop_loss = stop_loss > 0.0 && eval_lows.iter().any(|&l| l <= stop_loss);
        let hit_take_profit = target_price > 0.0 && eval_highs.iter().any(|&h| h >= target_price);

        // 计算最大回撤
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

        // 方向是否正确
        let direction_correct = (direction_expected == "up" && stock_return_pct > 0.0)
            || (direction_expected == "down" && stock_return_pct < 0.0)
            || direction_expected == "neutral";

        // 保存回测结果
        let insert_sql = "INSERT INTO backtest_results \
             (analysis_id, stock_code, signal_date, decision_action, simulated_entry, simulated_exit, \
              return_pct, max_drawdown, direction_correct, scope_type, status, create_time, \
              eval_window_days, eval_status, evaluated_at, start_price, end_close, max_high, min_low, \
              stock_return_pct, direction_expected, outcome, stop_loss_price, take_profit_price, \
              hit_stop_loss, hit_take_profit, operation_advice) \
             VALUES (:aid, :code, :sdate, :action, :entry, :exit, \
              :ret, :dd, :dir_correct, 'watchlist', 1, NOW(), \
              :ew, 'completed', NOW(), :sp, :ec, :mh, :ml, \
              :sr, :de, :outcome, :sl, :tp, \
              :hit_sl, :hit_tp, :oa) \
             ON DUPLICATE KEY UPDATE \
              eval_status='completed', evaluated_at=NOW(), start_price=VALUES(start_price), \
              end_close=VALUES(end_close), max_high=VALUES(max_high), min_low=VALUES(min_low), \
              stock_return_pct=VALUES(stock_return_pct), direction_expected=VALUES(direction_expected), \
              outcome=VALUES(outcome), max_drawdown=VALUES(max_drawdown), return_pct=VALUES(return_pct), \
              direction_correct=VALUES(direction_correct), hit_stop_loss=VALUES(hit_stop_loss), \
              hit_take_profit=VALUES(hit_take_profit)";

        let result = Helper::execute(
            insert_sql,
            vec![
                ("aid".to_string(), Value::from(analysis_id)),
                ("code".to_string(), Value::from(stock_code.as_str())),
                ("sdate".to_string(), Value::from(signal_date.as_str())),
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
        .map_err(|e| DsaError::Database(format!("保存回测结果失败: {}", e)))?;

        // 更新信号状态为已评估
        let _ = Helper::execute(
            "UPDATE decision_signals SET status = 4, modify_time = NOW() WHERE id = :id",
            vec![("id".to_string(), Value::from(signal_id))],
            &connector,
        );

        Ok(value!({
            "id": result as i64,
            "signalId": signal_id,
            "stock_code": stock_code,
            "stock_name": stock_name,
            "direction_expected": direction_expected,
            "outcome": outcome,
            "start_price": start_price,
            "end_close": end_close,
            "max_high": max_high,
            "min_low": min_low,
            "stock_return_pct": stock_return_pct,
            "max_drawdown": max_drawdown,
            "direction_correct": direction_correct,
            "hit_stop_loss": hit_stop_loss,
            "hit_take_profit": hit_take_profit,
            "evalWindow": eval_window,
        }))
    }

    /// 批量评估所有待评估信号
    async fn evaluate_batch(&self, params: &Value) -> DsaResult<Value> {
        let limit = utils::param_i64(params, "limit").max(1).min(100) as i64;
        let connector = utils::get_db_connector()?;
        let conf = dsa_core::get_global_config();
        let eval_window = conf.backtest.eval_window_days;

        // 查询所有活跃且已过评估窗口的信号
        let sql = "SELECT id FROM decision_signals \
             WHERE status = 1 AND signal_date < DATE_SUB(CURDATE(), INTERVAL :ew DAY) \
             ORDER BY signal_date ASC LIMIT :limit";
        let rows = Helper::query_rows(
            sql,
            vec![
                ("ew".to_string(), Value::from(eval_window as i64)),
                ("limit".to_string(), Value::from(limit)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询待评估信号失败: {}", e)))?;

        let mut evaluated_count: i64 = 0;
        let mut errors: Vec<Value> = Vec::new();

        for row in &rows {
            let sid: i64 = row.get_value(0).as_f64().unwrap_or(0.0) as i64;
            let eval_params = value!({"signalId": sid});
            match self.evaluate(&eval_params).await {
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

    /// 回测统计摘要
    async fn summary(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
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
             STDDEV(stock_return_pct) as std_return \
             FROM backtest_results WHERE {}",
            where_clause
        );

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询回测统计失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(value!({
                "totalTrades": 0,
                "winRate": 0.0,
                "avgReturn": 0.0,
                "sharpeRatio": 0.0,
                "max_drawdown": 0.0,
                "wins": 0,
                "losses": 0,
                "neutrals": 0,
            }));
        }

        let r = &rows[0];
        let total: f64 = r.get_value(0).as_f64().unwrap_or(0.0);
        let wins: f64 = r.get_value(1).as_f64().unwrap_or(0.0);
        let losses: f64 = r.get_value(2).as_f64().unwrap_or(0.0);
        let neutrals: f64 = r.get_value(3).as_f64().unwrap_or(0.0);
        let avg_return: f64 = r.get_value(4).as_f64().unwrap_or(0.0);
        let _avg_win: f64 = r.get_value(5).as_f64().unwrap_or(0.0);
        let _avg_loss: f64 = r.get_value(6).as_f64().unwrap_or(0.0);
        let max_dd: f64 = r.get_value(7).as_f64().unwrap_or(0.0);
        let std_return: f64 = r.get_value(8).as_f64().unwrap_or(0.0);

        let win_rate = if total > 0.0 { wins / total * 100.0 } else { 0.0 };
        // Sharpe ratio approximation: avg_return / std_return (annualized would multiply by sqrt(252/eval_window))
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
            "max_drawdown": max_dd,
            "wins": wins as i64,
            "losses": losses as i64,
            "neutrals": neutrals as i64,
        }))
    }

    /// 单个回测结果详情
    async fn detail(&self, params: &Value) -> DsaResult<Value> {
        let id = utils::param_i64(params, "id");
        if id == 0 {
            return Err(DsaError::Validation("请提供回测ID".to_string()));
        }

        let connector = utils::get_db_connector()?;
        let sql = "SELECT id, analysis_id, stock_code, signal_date, decision_action, \
             simulated_entry, simulated_exit, exit_date, return_pct, max_drawdown, \
             direction_correct, scope_type, status, create_time, \
             eval_window_days, eval_status, evaluated_at, start_price, end_close, \
             max_high, min_low, stock_return_pct, direction_expected, outcome, \
             stop_loss_price, take_profit_price, hit_stop_loss, hit_take_profit \
             FROM backtest_results WHERE id = :id AND status >= 1";
        let rows = Helper::query_rows(
            sql,
            vec![("id".to_string(), Value::from(id))],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询回测详情失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(Value::Null);
        }

        Ok(rows[0].to_value2())
    }

    /// 回测结果列表
    async fn list(&self, params: &Value) -> DsaResult<Value> {
        let connector = utils::get_db_connector()?;
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

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询回测列表失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(Value::Array(results))
    }
}
