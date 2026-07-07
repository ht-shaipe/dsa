//! Database module — schema migration utilities

pub mod migration;

pub use migration::run_migrations;
