//! Target-specific renderers for code generation.

pub mod csharp;
pub mod ethers5;
pub mod ethers6;
mod ethers_common;
pub mod go;
/// Godot 4 GDScript bindings backed by the shared C runtime.
pub mod godot;
pub mod kotlin;
pub mod python;
pub mod rust;
pub mod solidity;
pub mod swift;
/// Unreal Engine reflected adapters backed by the shared C runtime.
pub mod unreal;
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

/// PHP contract bindings.
pub mod php;

/// Experimental GnuCOBOL read bindings.
pub mod cobol;

/// Ruby contract bindings.
pub mod ruby;

/// Shell contract bindings backed by Foundry cast.
pub mod shell;

/// Elixir contract bindings backed by Ethers.
pub mod elixir;
