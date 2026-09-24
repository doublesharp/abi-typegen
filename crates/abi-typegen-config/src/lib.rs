//! Configuration parsing for abi-typegen (`foundry.toml`).

pub mod config;
pub use config::{Config, ConfigError, DEFAULT_PACKAGE, Target, parse_target, validate_package};
