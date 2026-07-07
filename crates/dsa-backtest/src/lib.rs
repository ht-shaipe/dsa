//! DSA Backtest - 回测引擎与决策信号追踪

#[macro_use]
extern crate tube;

pub mod engine;
pub mod signal;
pub mod report;

pub use engine::BacktestEngine;
pub use report::BacktestReport;
pub use signal::SignalTracker;
