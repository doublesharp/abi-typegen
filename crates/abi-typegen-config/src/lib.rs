//! Configuration parsing for abi-typegen (`foundry.toml`).

pub mod config;
pub use config::{parse_target, Config, ConfigError, Target};
