//! 回测引擎 - 对齐原项目 backtest

use dsa_core::{DsaError, DsaResult};
use deck_connector::{get_connector, Connector};
use deck_mysql::{DataRow, Helper};
use tube::Value;

pub struct BacktestEngine {}

impl BacktestEngine {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn dispatch(&self, method: &str, params: &Value) -> DsaResult<Value> {
        match method {
            "run" => self.run(params).await,
            "list" => self.list(params).await,
            "detail" => self.detail(params).await,
            "outcomes" => self.outcomes(params).await,
            "performance" => self.performance(params).await,
            _ => Err(DsaError::ApiRouting(format!(
                "backtest不支持方法: {}",
                method
            ))),
        }
    }

    fn get_db_connector() -> Result<Connector, DsaError> {
        get_connector("default", "mysql")
            .ok_or_else(|| DsaError::Database("MySQL连接未初始化".to_string()))
    }

    /// 运行回测 - 评估分析预测 vs 实际走势
    async fn run(&self, params: &Value) -> DsaResult<Value> {
        let analysis_id = params
            .get("analysisId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;

        if analysis_id == 0 {
            return Err(DsaError::Validation("请提供分析ID".to_string()));
        }

        let conf = dsa_core::get_global_config();
        let eval_window = conf.backtest.eval_window_days as i64;
        let connector = Self::get_db_connector()?;

        // 查询分析历史记录
        let sql =
            "SELECT id, stock_code, stock_name, sentiment_score, trend_prediction, operation_advice, \
             decisionType, idealBuy, secondaryBuy, stopLoss, takeProfit, reportJson, \
             analysisSummary, riskWarning, marketContext, createTime \
             FROM analysis_history WHERE id = :id AND status = 1";
        let rows = Helper::query_rows(sql, vec![("id".to_string(), Value::from(analysis_id))], &connector)
            .map_err(|e| DsaError::Database(format!("查询分析历史失败: {}", e)))?;

        if rows.is_empty() {
            return Err(DsaError::Validation(format!(
                "分析记录不存在: {}",
                analysis_id
            )));
        }

        let row = &rows[0];
        let stock_code = row.get_string(1);
        let decision_type = row.get_string(5);
        let ideal_buy: f64 = row.get_value(6).as_f64().unwrap_or(0.0);
        let stop_loss: f64 = row.get_value(8).as_f64().unwrap_or(0.0);
        let take_profit: f64 = row.get_value(9).as_f64().unwrap_or(0.0);

        // 获取信号日期后的实际K线数据
        let signal_date_str = row.get_string(15);
        let hist_sql = "SELECT close FROM stock_daily \
             WHERE stock_code = :code AND status = 1 \
             ORDER BY trade_date DESC LIMIT :limit";
        let hist_rows = Helper::query_rows(
            hist_sql,
            vec![
                ("code".to_string(), Value::from(stock_code.as_str())),
                ("limit".to_string(), Value::from(eval_window + 5)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("查询K线数据失败: {}", e)))?;

        let entry_price = if ideal_buy > 0.0 {
            ideal_buy
        } else {
            hist_rows
                .first()
                .map(|r| r.get_value(0).as_f64().unwrap_or(0.0))
                .unwrap_or(0.0)
        };

        // 计算实际回报
        let eval_closes: Vec<f64> = hist_rows
            .iter()
            .take(eval_window as usize)
            .map(|r| r.get_value(0).as_f64().unwrap_or(0.0))
            .collect();

        let (actual_return, max_drawdown, simulated_exit, direction_correct, hit_stop_loss, hit_take_profit) =
            if entry_price > 0.0 && !eval_closes.is_empty() {
                let exit_price = eval_closes.last().copied().unwrap_or(entry_price);
                let return_pct = (exit_price - entry_price) / entry_price * 100.0;

                let mut max_dd = 0.0_f64;
                let mut peak = entry_price;
                for &c in &eval_closes {
                    if c > peak {
                        peak = c;
                    }
                    let dd = (peak - c) / peak * 100.0;
                    if dd > max_dd {
                        max_dd = dd;
                    }
                }

                let predicted_up = matches!(
                    decision_type.as_str(),
                    "buy" | "add" | "hold" | "watch"
                );
                let dir_correct = (predicted_up && return_pct > 0.0)
                    || (!predicted_up && return_pct < 0.0)
                    || decision_type == "hold";

                let hit_sl = stop_loss > 0.0
                    && eval_closes.iter().any(|&c| c <= stop_loss);
                let hit_tp = take_profit > 0.0
                    && eval_closes.iter().any(|&c| c >= take_profit);

                (return_pct, max_dd, exit_price, dir_correct, hit_sl, hit_tp)
            } else {
                (0.0, 0.0, 0.0, false, false, false)
            };

        // 存储回测结果
        let insert_sql = "INSERT INTO backtest_results \
             (analysisId, stockCode, signalDate, decisionAction, simulatedEntry, simulatedExit, \
              exitDate, returnPct, maxDrawdown, directionCorrect, scopeType, status, createTime) \
             VALUES (:aid, :code, :sdate, :action, :entry, :exit, NULL, :ret, :dd, :dir_correct, 'watchlist', 1, NOW())";
        let insert_result = Helper::execute(
            insert_sql,
            vec![
                ("aid".to_string(), Value::from(analysis_id)),
                ("code".to_string(), Value::from(stock_code.as_str())),
                ("sdate".to_string(), Value::from(signal_date_str.as_str())),
                (
                    "action".to_string(),
                    Value::from(decision_type.as_str()),
                ),
                ("entry".to_string(), Value::from(entry_price)),
                ("exit".to_string(), Value::from(simulated_exit)),
                ("ret".to_string(), Value::from(actual_return)),
                ("dd".to_string(), Value::from(max_drawdown)),
                ("dir_correct".to_string(), Value::from(direction_correct)),
            ],
            &connector,
        )
        .map_err(|e| DsaError::Database(format!("保存回测结果失败: {}", e)))?;

        Ok(value!({
            "status": "ok",
            "data": {
                "id": insert_result as i64,
                "analysisId": analysis_id,
                "stockCode": stock_code,
                "decisionAction": decision_type,
                "simulatedEntry": entry_price,
                "simulatedExit": simulated_exit,
                "returnPct": actual_return,
                "maxDrawdown": max_drawdown,
                "directionCorrect": direction_correct,
                "hitStopLoss": hit_stop_loss,
                "hitTakeProfit": hit_take_profit,
                "evalWindow": eval_window,
            }
        }))
    }

    /// 查询回测结果列表
    async fn list(&self, params: &Value) -> DsaResult<Value> {
        let connector = Self::get_db_connector()?;
        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let (sql, p) = if code.is_empty() {
            (
                "SELECT id, analysis_id, stock_code, signal_date, decision_action, \
                 simulatedEntry, simulatedExit, returnPct, maxDrawdown, directionCorrect, \
                 scopeType, status, createTime \
                 FROM backtest_results WHERE status = 1 ORDER BY create_time DESC LIMIT :limit"
                    .to_string(),
                vec![("limit".to_string(), Value::from(limit))],
            )
        } else {
            (
                "SELECT id, analysis_id, stock_code, signal_date, decision_action, \
                 simulatedEntry, simulatedExit, returnPct, maxDrawdown, directionCorrect, \
                 scopeType, status, createTime \
                 FROM backtest_results WHERE status = 1 AND stock_code = :code ORDER BY create_time DESC LIMIT :limit"
                    .to_string(),
                vec![
                    ("code".to_string(), Value::from(code.as_str())),
                    ("limit".to_string(), Value::from(limit)),
                ],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询回测列表失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(Value::Array(results))
    }

    /// 查询回测结果详情
    async fn detail(&self, params: &Value) -> DsaResult<Value> {
        let id = params
            .get("id")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        if id == 0 {
            return Err(DsaError::Validation("请提供回测ID".to_string()));
        }

        let connector = Self::get_db_connector()?;
        let sql = "SELECT id, analysis_id, stock_code, signal_date, decision_action, \
             simulatedEntry, simulatedExit, exitDate, returnPct, maxDrawdown, \
             directionCorrect, scopeType, status, createTime \
             FROM backtest_results WHERE id = :id AND status = 1";
        let rows = Helper::query_rows(sql, vec![("id".to_string(), Value::from(id))], &connector)
            .map_err(|e| DsaError::Database(format!("查询回测详情失败: {}", e)))?;

        if rows.is_empty() {
            return Err(DsaError::Validation(format!(
                "回测记录不存在: {}",
                id
            )));
        }

        Ok(rows[0].to_value2())
    }

    /// 查询信号结果评估
    async fn outcomes(&self, params: &Value) -> DsaResult<Value> {
        let connector = Self::get_db_connector()?;
        let signal_id = params
            .get("signalId")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i64;
        let limit = params
            .get("limit")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as i64;

        let (sql, p) = if signal_id > 0 {
            (
                "SELECT id, signal_id, stock_code, eval_horizon, eval_date, actual_return, \
                 maxDrawdown, directionCorrect, hitTarget, hitStopLoss, status, createTime \
                 FROM decision_signal_outcomes WHERE signal_id = :sid AND status = 1 \
                 ORDER BY eval_date DESC LIMIT :limit"
                    .to_string(),
                vec![
                    ("sid".to_string(), Value::from(signal_id)),
                    ("limit".to_string(), Value::from(limit)),
                ],
            )
        } else {
            (
                "SELECT id, signal_id, stock_code, eval_horizon, eval_date, actual_return, \
                 maxDrawdown, directionCorrect, hitTarget, hitStopLoss, status, createTime \
                 FROM decision_signal_outcomes WHERE status = 1 \
                 ORDER BY eval_date DESC LIMIT :limit"
                    .to_string(),
                vec![("limit".to_string(), Value::from(limit))],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询信号结果失败: {}", e)))?;

        let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
        Ok(Value::Array(results))
    }

    /// 聚合回测统计数据
    async fn performance(&self, params: &Value) -> DsaResult<Value> {
        let connector = Self::get_db_connector()?;
        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let (sql, p) = if code.is_empty() {
            (
                "SELECT COUNT(*) as total_trades, \
                 SUM(CASE WHEN returnPct > 0 THEN 1 ELSE 0 END) as wins, \
                 AVG(returnPct) as avg_return, \
                 SUM(CASE WHEN directionCorrect = 1 THEN 1 ELSE 0 END) as dir_correct, \
                 MAX(maxDrawdown) as max_drawdown \
                 FROM backtest_results WHERE status = 1"
                    .to_string(),
                vec![],
            )
        } else {
            (
                "SELECT COUNT(*) as total_trades, \
                 SUM(CASE WHEN returnPct > 0 THEN 1 ELSE 0 END) as wins, \
                 AVG(returnPct) as avg_return, \
                 SUM(CASE WHEN directionCorrect = 1 THEN 1 ELSE 0 END) as dir_correct, \
                 MAX(maxDrawdown) as max_drawdown \
                 FROM backtest_results WHERE status = 1 AND stock_code = :code"
                    .to_string(),
                vec![("code".to_string(), Value::from(code.as_str()))],
            )
        };

        let rows = Helper::query_rows(&sql, p, &connector)
            .map_err(|e| DsaError::Database(format!("查询回测统计失败: {}", e)))?;

        if rows.is_empty() {
            return Ok(value!({
                "status": "ok",
                "data": {
                    "totalTrades": 0,
                    "winRate": 0.0,
                    "avgReturn": 0.0,
                    "directionAccuracy": 0.0,
                    "maxDrawdown": 0.0,
                }
            }));
        }

        let row = &rows[0];
        let total: f64 = row.get_value(0).as_f64().unwrap_or(0.0);
        let wins: f64 = row.get_value(1).as_f64().unwrap_or(0.0);
        let avg_return: f64 = row.get_value(2).as_f64().unwrap_or(0.0);
        let dir_correct: f64 = row.get_value(3).as_f64().unwrap_or(0.0);
        let max_dd: f64 = row.get_value(4).as_f64().unwrap_or(0.0);

        let win_rate = if total > 0.0 {
            wins / total * 100.0
        } else {
            0.0
        };
        let dir_accuracy = if total > 0.0 {
            dir_correct / total * 100.0
        } else {
            0.0
        };

        Ok(value!({
            "status": "ok",
            "data": {
                "totalTrades": total as i64,
                "winRate": win_rate,
                "avgReturn": avg_return,
                "directionAccuracy": dir_accuracy,
                "maxDrawdown": max_dd,
            }
        }))
    }
}
