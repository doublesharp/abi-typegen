//! Target-specific renderers for code generation.

pub mod csharp;
pub mod ethers5;
pub mod ethers6;
mod ethers_common;
pub mod go;
pub mod kotlin;
pub mod python;
pub mod rust;
pub mod solidity;
pub mod swift;
pub mod viem;
pub mod wagmi;
pub mod web3js;
pub mod yaml;
pub mod zod;

/// C11 contract bindings.
pub mod c;
/// C++17 contract bindings.
pub mod cpp;

/// Java contract bindings.
pub mod java;

/// Dart contract bindings.
pub mod dart;
