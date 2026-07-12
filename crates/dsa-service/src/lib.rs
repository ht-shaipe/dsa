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

pub mod config;
pub mod auth;
pub mod usage;
pub mod stock;
pub mod analysis;
pub mod scheduler;
pub mod portfolio;
pub mod market;
pub mod decision;
pub mod intelligence;
pub mod alert;
pub mod system;
pub mod screening;
pub mod notification;
pub mod search;
pub mod social_sentiment;
pub mod report;
pub mod backtest;
pub mod alert_worker;
pub mod decision_extractor;
pub mod market_context;
pub mod name_resolver;
pub mod bot;
pub mod indicator;

pub use auth::Auth;
pub use usage::Usage;
pub use stock::Stock;
pub use analysis::Analysis;
pub use scheduler::Scheduler;
pub use portfolio::Portfolio;
pub use market::Market;
pub use decision::Decision;
pub use intelligence::Intelligence;
pub use alert::Alert;
pub use system::System;
pub use screening::Screening;
pub use notification::Notification;
pub use search::Search;
pub use social_sentiment::SocialSentiment;
pub use report::Report;
pub use backtest::Backtest;
pub use alert_worker::AlertWorker;
pub use decision_extractor::DecisionExtractor;
pub use market_context::MarketContext;
pub use name_resolver::NameResolver;
pub use bot::dispatcher::BotDispatcher;
pub use indicator::Indicator;
