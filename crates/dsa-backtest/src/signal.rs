//! 信号追踪 - 决策信号的追踪与评估

use dsa_core::{DsaError, DsaResult};
use deck_connector::{get_connector, Connector};
use deck_mysql::{DataRow, Helper};
use tube::Value;

pub struct SignalTracker {
    pub signals: Vec<Value>,
}

impl SignalTracker {
    pub fn new() -> Self {
        Self { signals: Vec::new() }
    }

    pub fn add_signal(&mut self, signal: Value) {
        self.signals.push(signal);
    }

    /// 评估信号预测 vs 实际走势
    pub async fn evaluate_outcomes(&self, eval_window: i64) -> DsaResult<Vec<Value>> {
        let connector = Self::get_db_connector()?;
        let conf = dsa_core::get_global_config();
        let window = if eval_window > 0 {
            eval_window
        } else {
            conf.backtest.eval_window_days as i64
        };

        // 查询待评估的活跃信号
        let sql = "SELECT id, stock_code, action, entry_price, stop_loss, target_price, \
             signal_date FROM decision_signals \
             WHERE status = 1 AND action IN ('buy', 'add', 'hold', 'reduce', 'sell') \
             ORDER BY signal_date DESC LIMIT 100";
        let rows = Helper::query_rows(sql, vec![], &connector)
            .map_err(|e| DsaError::Database(format!("查询决策信号失败: {}", e)))?;

        let mut outcomes = Vec::new();

        for row in &rows {
            let signal_id: i64 = row.get_value(0).as_f64().unwrap_or(0.0) as i64;
            let code = row.get_string(1);
            let action = row.get_string(2);
            let entry_price: f64 = row.get_value(3).as_f64().unwrap_or(0.0);
            let stop_loss: f64 = row.get_value(4).as_f64().unwrap_or(0.0);
            let target_price: f64 = row.get_value(5).as_f64().unwrap_or(0.0);
            let signal_date_raw = row.get_string(6);
            let signal_date_only = signal_date_raw.split(' ').next().unwrap_or(&signal_date_raw);

            // 检查是否已有评估结果
            let check_sql = "SELECT id FROM decision_signal_outcomes WHERE signal_id = :sid AND eval_horizon = :horizon LIMIT 1";
            let existing = Helper::query_rows(
                check_sql,
                vec![
                    ("sid".to_string(), Value::from(signal_id)),
                    ("horizon".to_string(), Value::from(window as i64)),
                ],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("检查已有评估失败: {}", e)))?;

            if !existing.is_empty() {
                continue;
            }

            // 获取信号日期之后的实际K线数据
            let hist_sql = "SELECT high, low, close FROM stock_daily \
                 WHERE stock_code = :code AND DATE(trade_date) >= :sdate AND status = 1 \
                 ORDER BY trade_date ASC LIMIT :limit";
            let hist_rows = Helper::query_rows(
                hist_sql,
                vec![
                    ("code".to_string(), Value::from(code.as_str())),
                    ("sdate".to_string(), Value::from(signal_date_only)),
                    ("limit".to_string(), Value::from(window + 5)),
                ],
                &connector,
            )
            .map_err(|e| DsaError::Database(format!("查询K线数据失败: {}", e)))?;

            if hist_rows.is_empty() || entry_price <= 0.0 {
                continue;
            }

            let eval_closes: Vec<f64> = hist_rows
                .iter()
                .take(window as usize)
                .map(|r| r.get_value(2).as_f64().unwrap_or(0.0))
                .collect();
            let eval_highs: Vec<f64> = hist_rows
                .iter()
                .take(window as usize)
                .map(|r| r.get_value(0).as_f64().unwrap_or(0.0))
                .collect();
            let eval_lows: Vec<f64> = hist_rows
                .iter()
                .take(window as usize)
                .map(|r| r.get_value(1).as_f64().unwrap_or(0.0))
                .collect();

            let exit_price = eval_closes.last().copied().unwrap_or(entry_price);
            let actual_return = (exit_price - entry_price) / entry_price * 100.0;

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

            let predicted_up = matches!(action.as_str(), "buy" | "add" | "hold" | "watch");
            let direction_correct = (predicted_up && actual_return > 0.0)
                || (!predicted_up && actual_return < 0.0);

            let hit_target = target_price > 0.0 && eval_highs.iter().any(|&h| h >= target_price);
            let hit_sl = stop_loss > 0.0 && eval_lows.iter().any(|&l| l <= stop_loss);

            // 保存评估结果
            let insert_sql = "INSERT INTO decision_signal_outcomes \
                 (signal_id, stock_code, eval_horizon, eval_date, actual_return, max_drawdown, \
                  direction_correct, hit_target, hit_stop_loss, status, create_time) \
                 VALUES (:sid, :code, :horizon, NOW(), :ret, :dd, :dir_correct, :hit_target, :hit_sl, 1, NOW())";
            if let Err(e) = Helper::execute(
                insert_sql,
                vec![
                    ("sid".to_string(), Value::from(signal_id)),
                    ("code".to_string(), Value::from(code.as_str())),
                    ("horizon".to_string(), Value::from(window as i64)),
                    ("ret".to_string(), Value::from(actual_return)),
                    ("dd".to_string(), Value::from(max_dd)),
                    ("dir_correct".to_string(), Value::from(direction_correct)),
                    ("hit_target".to_string(), Value::from(hit_target)),
                    ("hit_sl".to_string(), Value::from(hit_sl)),
                ],
                &connector,
            ) {
                tracing::warn!("插入信号评估结果失败: {}", e);
            }

            outcomes.push(value!({
                "signalId": signal_id,
                "stock_code": code,
                "action": action,
                "actualReturn": actual_return,
                "max_drawdown": max_dd,
                "direction_correct": direction_correct,
                "hitTarget": hit_target,
                "hit_stop_loss": hit_sl,
            }));
        }

        Ok(outcomes)
    }

    fn get_db_connector() -> Result<Connector, DsaError> {
        get_connector("default", "mysql")
            .ok_or_else(|| DsaError::Database("MySQL连接未初始化".to_string()))
    }
}
