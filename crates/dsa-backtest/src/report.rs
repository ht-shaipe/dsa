//! 回测报告 - 聚合统计计算

use tube::Value;

pub struct BacktestReport {
    pub total_trades: i64,
    pub win_rate: f64,
    pub avg_return: f64,
    pub direction_accuracy: f64,
    pub max_drawdown: f64,
}

impl BacktestReport {
    pub fn new() -> Self {
        Self {
            total_trades: 0,
            win_rate: 0.0,
            avg_return: 0.0,
            direction_accuracy: 0.0,
            max_drawdown: 0.0,
        }
    }

    /// 从回测结果数组计算聚合报告
    pub fn from_results(results: &[Value]) -> Self {
        if results.is_empty() {
            return Self::new();
        }

        let total = results.len() as i64;
        let mut wins = 0i64;
        let mut total_return = 0.0_f64;
        let mut dir_correct = 0i64;
        let mut max_dd = 0.0_f64;

        for r in results {
            let ret = r.get("returnPct").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if ret > 0.0 {
                wins += 1;
            }
            total_return += ret;

            if r.get("directionCorrect")
                .and_then(|v| v.as_f64())
                .map(|v| v > 0.0)
                .unwrap_or(false)
            {
                dir_correct += 1;
            }

            let dd = r.get("maxDrawdown").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if dd > max_dd {
                max_dd = dd;
            }
        }

        BacktestReport {
            total_trades: total,
            win_rate: if total > 0 {
                wins as f64 / total as f64 * 100.0
            } else {
                0.0
            },
            avg_return: if total > 0 {
                total_return / total as f64
            } else {
                0.0
            },
            direction_accuracy: if total > 0 {
                dir_correct as f64 / total as f64 * 100.0
            } else {
                0.0
            },
            max_drawdown: max_dd,
        }
    }

    /// 序列化为 Value
    pub fn to_value(&self) -> Value {
        value!({
            "totalTrades": self.total_trades,
            "winRate": self.win_rate,
            "avgReturn": self.avg_return,
            "directionAccuracy": self.direction_accuracy,
            "maxDrawdown": self.max_drawdown,
        })
    }
}
