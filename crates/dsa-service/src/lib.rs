#[macro_use]
extern crate tube;
#[allow(unused_imports)]
use tube::{Error, Result, Value};

#[macro_use]
extern crate deck;
#[allow(unused_imports)]
pub(crate) use deck::{ColumnExpr, Condition, Joint, Operator, OrderExpr};

#[macro_use]
extern crate lazy_static;

lazy_static! {
    pub static ref DATASOURCE_KEY: String = "default".to_owned();
}

pub mod alert;
pub mod alert_worker;
pub mod analysis;
pub mod backtest;
pub mod bot;
pub mod config;
pub mod decision;
pub mod decision_extractor;
pub mod indicator;
pub mod intelligence;
pub mod market;
pub mod market_context;
pub mod name_resolver;
pub mod notification;
pub mod portfolio;
pub mod report;
pub mod scheduler;
pub mod screening;
pub mod search;
pub mod social_sentiment;
pub mod stock;
pub mod system;
pub mod usage;

pub use alert::Alert;
pub use alert_worker::AlertWorker;
pub use analysis::Analysis;
pub use backtest::Backtest;
pub use bot::dispatcher::BotDispatcher;
pub use decision::Decision;
pub use decision_extractor::DecisionExtractor;
pub use indicator::Indicator;
pub use intelligence::Intelligence;
pub use market::Market;
pub use market_context::MarketContext;
pub use name_resolver::NameResolver;
pub use notification::Notification;
pub use portfolio::Portfolio;
pub use report::Report;
pub use scheduler::Scheduler;
pub use screening::Screening;
pub use search::Search;
pub use social_sentiment::SocialSentiment;
pub use stock::Stock;
pub use system::System;
pub use usage::Usage;
