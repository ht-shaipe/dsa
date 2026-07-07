//! 回测工具 - 对接回测引擎

use dsa_core::utils;
use deck_mysql::{DataRow, Helper};
use tube::Value;

pub struct BacktestTools;

impl BacktestTools {
    pub fn new() -> Self { Self }

    /// 获取回测摘要 - 从数据库聚合真实统计数据
    pub fn get_backtest_summary(code: &str) -> Value {
        let connector = match utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => {
                return value!({
                    "code": code,
                    "totalTrades": 0,
                    "winRate": 0.0,
                    "avgReturn": 0.0,
                    "maxDrawdown": 0.0,
                    "sharpeRatio": 0.0,
                    "message": "数据库连接失败",
                });
            }
        };

        let sql = if code.is_empty() {
            "SELECT COUNT(*) as total_trades, \
             SUM(CASE WHEN returnPct > 0 THEN 1 ELSE 0 END) as wins, \
             AVG(returnPct) as avg_return, \
             MAX(maxDrawdown) as max_drawdown, \
             SUM(CASE WHEN directionCorrect = 1 THEN 1 ELSE 0 END) as dir_correct \
             FROM backtest_results WHERE status = 1".to_string()
        } else {
            format!("SELECT COUNT(*) as total_trades, \
             SUM(CASE WHEN returnPct > 0 THEN 1 ELSE 0 END) as wins, \
             AVG(returnPct) as avg_return, \
             MAX(maxDrawdown) as max_drawdown, \
             SUM(CASE WHEN directionCorrect = 1 THEN 1 ELSE 0 END) as dir_correct \
             FROM backtest_results WHERE status = 1 AND stock_code = '{}'", code)
        };

        match Helper::query_rows(&sql, vec![], &connector) {
            Ok(rows) => {
                if let Some(row) = rows.first() {
                    let total: f64 = row.get_value(0).as_f64().unwrap_or(0.0);
                    let wins: f64 = row.get_value(1).as_f64().unwrap_or(0.0);
                    let avg_return: f64 = row.get_value(2).as_f64().unwrap_or(0.0);
                    let max_dd: f64 = row.get_value(3).as_f64().unwrap_or(0.0);
                    let _dir_correct: f64 = row.get_value(4).as_f64().unwrap_or(0.0);

                    let win_rate = if total > 0.0 { wins / total } else { 0.0 };
                    let sharpe = if total > 2.0 {
                        let avg_r = avg_return / 100.0;
                        let std_dev_sql = if code.is_empty() {
                            "SELECT STDDEV(return_pct) as std_dev FROM backtest_results WHERE status = 1".to_string()
                        } else {
                            format!("SELECT STDDEV(return_pct) as std_dev FROM backtest_results WHERE status = 1 AND stock_code = '{}'", code)
                        };
                        let std_dev = Helper::query_rows(&std_dev_sql, vec![], &connector)
                            .ok()
                            .and_then(|r| r.first().map(|row| row.get_value(0).as_f64().unwrap_or(0.0)))
                            .unwrap_or(0.0)
                            / 100.0;
                        if std_dev > 0.0 { (avg_r / std_dev) * (252.0_f64).sqrt() } else { 0.0 }
                    } else {
                        0.0
                    };

                    value!({
                        "code": code,
                        "totalTrades": total as i64,
                        "winRate": win_rate,
                        "avgReturn": avg_return,
                        "maxDrawdown": max_dd,
                        "sharpeRatio": sharpe,
                    })
                } else {
                    default_summary(code)
                }
            }
            Err(_) => default_summary(code),
        }
    }

    /// 获取最近回测列表
    pub fn get_recent_backtests(code: &str, limit: i64) -> Value {
        let connector = match utils::get_db_connector() {
            Ok(c) => c,
            Err(_) => return value!({"data": []}),
        };

        let sql = if code.is_empty() {
            format!("SELECT id, analysis_id, stock_code, signal_date, decision_action, \
                 simulatedEntry, simulatedExit, returnPct, maxDrawdown, directionCorrect, createTime \
                 FROM backtest_results WHERE status = 1 ORDER BY create_time DESC LIMIT {}", limit)
        } else {
            format!("SELECT id, analysis_id, stock_code, signal_date, decision_action, \
                 simulatedEntry, simulatedExit, returnPct, maxDrawdown, directionCorrect, createTime \
                 FROM backtest_results WHERE status = 1 AND stock_code = '{}' ORDER BY create_time DESC LIMIT {}", code, limit)
        };

        match Helper::query_rows(&sql, vec![], &connector) {
            Ok(rows) => {
                let results: Vec<Value> = rows.iter().map(|r| r.to_value2()).collect();
                value!({"data": results})
            }
            Err(_) => value!({"data": []}),
        }
    }
}

fn default_summary(code: &str) -> Value {
    value!({
        "code": code,
        "totalTrades": 0,
        "winRate": 0.0,
        "avgReturn": 0.0,
        "max_drawdown": 0.0,
        "sharpeRatio": 0.0,
    })
}
