//! Core ABI parsing and IR types for abi-typegen.

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod parser;
pub mod types;

pub use parser::ParseError;
