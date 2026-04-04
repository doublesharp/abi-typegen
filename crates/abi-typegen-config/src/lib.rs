//! Configuration parsing for abi-typegen (`foundry.toml`).

pub mod config;
pub use config::{Config, ConfigError, Target, parse_target};
