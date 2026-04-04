// tests/integration_test.rs
use abi_typegen_codegen::abi_writer::render_abi_file;
use abi_typegen_codegen::barrel::render_barrel;
use abi_typegen_codegen::ethers::render_ethers_file;
use abi_typegen_codegen::generate_contract_files;
use abi_typegen_codegen::solidity::render_solidity_file;
use abi_typegen_codegen::viem::render_viem_file;
use abi_typegen_codegen::yaml::render_yaml_file;
use abi_typegen_codegen::zod::render_zod_file;
use abi_typegen_config::Config;
use abi_typegen_core::parser::parse_artifact;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture: {}", path.display()))
}

fn default_config() -> Config {
    Config::from_toml_str("").unwrap()
}

fn viem_config() -> Config {
    Config::from_toml_str("[abi-typegen]\ntarget = \"viem\"\n").unwrap()
}

fn ethers_config() -> Config {
    Config::from_toml_str("[abi-typegen]\ntarget = \"ethers\"\n").unwrap()
}

fn zod_config() -> Config {
    Config::from_toml_str("[abi-typegen]\ntarget = \"zod\"\n").unwrap()
}

fn python_config() -> Config {
    Config::from_toml_str("[abi-typegen]\ntarget = \"python\"\n").unwrap()
}

fn solidity_config() -> Config {
    Config::from_toml_str("[abi-typegen]\ntarget = \"solidity\"\n").unwrap()
}

fn yaml_config() -> Config {
    Config::from_toml_str("[abi-typegen]\ntarget = \"yaml\"\n").unwrap()
}

// ── ERC20 pipeline ──────────────────────────────────────────────────────────

#[test]
fn erc20_viem_target_produces_two_files() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &viem_config());
    assert!(files.contains_key("ERC20.abi.ts"));
    assert!(files.contains_key("ERC20.viem.ts"));
    assert!(!files.contains_key("ERC20.ethers.ts"));
    assert_eq!(files.len(), 2);
}

#[test]
fn erc20_ethers_target_produces_two_files() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &ethers_config());
    assert!(files.contains_key("ERC20.abi.ts"));
    assert!(files.contains_key("ERC20.ethers.ts"));
    assert_eq!(files.len(), 2);
}

#[test]
fn erc20_zod_target_produces_abi_and_schema_files() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &zod_config());
    assert!(files.contains_key("ERC20.abi.ts"));
    assert!(files.contains_key("ERC20.zod.ts"));
    assert_eq!(files.len(), 2);
}

#[test]
fn erc20_solidity_target_produces_interface_file() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &solidity_config());
    assert!(files.contains_key("IERC20.sol"));
    assert!(!files.contains_key("ERC20.abi.ts"));
    assert_eq!(files.len(), 1);
}

#[test]
fn erc20_no_wrappers_produces_only_abi() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let mut cfg = default_config();
    cfg.wrappers = false;
    let files = generate_contract_files(&ir, &cfg);
    assert_eq!(files.len(), 1);
    assert!(files.contains_key("ERC20.abi.ts"));
}

#[test]
fn non_ts_target_ignores_wrappers_flag() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let mut cfg = python_config();
    cfg.wrappers = false;
    let files = generate_contract_files(&ir, &cfg);
    assert_eq!(files.len(), 1);
    assert!(files.contains_key("ERC20.py"));
    assert!(!files.contains_key("ERC20.abi.ts"));
}

#[test]
fn zod_target_ignores_wrappers_flag_for_primary_schema_output() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let mut cfg = zod_config();
    cfg.wrappers = false;
    let files = generate_contract_files(&ir, &cfg);
    assert_eq!(files.len(), 2);
    assert!(files.contains_key("ERC20.abi.ts"));
    assert!(files.contains_key("ERC20.zod.ts"));
}

#[test]
fn solidity_target_ignores_wrappers_flag_for_primary_interface_output() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let mut cfg = solidity_config();
    cfg.wrappers = false;
    let files = generate_contract_files(&ir, &cfg);
    assert_eq!(files.len(), 1);
    assert!(files.contains_key("IERC20.sol"));
}

#[test]
fn erc20_abi_file_is_valid_ts_with_as_const() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &default_config());
    let abi = &files["ERC20.abi.ts"];
    assert!(abi.contains("export const ERC20Abi = ["));
    assert!(abi.contains("] as const;"));
    assert!(!abi.contains("SolType"));
}

#[test]
fn erc20_viem_file_correct_structure() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &viem_config());
    let viem = &files["ERC20.viem.ts"];
    assert!(viem.contains("import { ERC20Abi } from './ERC20.abi.js'"));
    assert!(viem.contains("export function getERC20Contract("));
    assert!(viem.contains("export type ERC20TransferParams = {"));
    assert!(viem.contains("to: `0x${string}`"));
}

#[test]
fn erc20_ethers_file_correct_structure() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &ethers_config());
    let eth = &files["ERC20.ethers.ts"];
    assert!(eth.contains("export interface ERC20Contract {"));
    assert!(eth.contains("export function connectERC20("));
    assert!(eth.contains("balanceOf(account: string): Promise<bigint>"));
    assert!(
        eth.contains("transfer(to: string, amount: bigint): Promise<ContractTransactionResponse>")
    );
    assert!(eth.contains("Transfer(from?: string | null, to?: string | null): EventFilter"));
}

#[test]
fn erc20_zod_file_correct_structure() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &zod_config());
    let zod = &files["ERC20.zod.ts"];
    assert!(zod.contains("import * as z from 'zod';"));
    assert!(zod.contains("export const ERC20TransferParamsSchema = z.object({"));
    assert!(zod.contains("to: z.string().regex(/^0x[a-fA-F0-9]{40}$/)"));
    assert!(zod.contains("export const ERC20BalanceOfResultSchema = z.bigint().refine("));
}

#[test]
fn erc20_solidity_file_correct_structure() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &solidity_config());
    let solidity = &files["IERC20.sol"];
    assert!(solidity.contains("interface IERC20 {"));
    assert!(solidity.contains("pragma solidity >=0.8.0;"));
    assert!(
        solidity
            .contains("event Transfer(address indexed from, address indexed to, uint256 value);")
    );
    assert!(solidity.contains(
        "error ERC20InsufficientBalance(address sender, uint256 balance, uint256 needed);"
    ));
    assert!(solidity.contains("function totalSupply() external view returns (uint256);"));
    assert!(
        solidity.contains("function transfer(address to, uint256 amount) external returns (bool);")
    );
}

// ── Vault pipeline ─────────────────────────────────────────────────────────

#[test]
fn vault_overloads_in_viem() {
    let ir = parse_artifact("Vault", &fixture("vault.json")).unwrap();
    let files = generate_contract_files(&ir, &viem_config());
    let viem = &files["Vault.viem.ts"];
    assert!(viem.contains("export type VaultDepositUint256Params = {"));
    assert!(viem.contains("export type VaultDepositUint256AddressParams = {"));
}

#[test]
fn vault_overloads_in_ethers() {
    let ir = parse_artifact("Vault", &fixture("vault.json")).unwrap();
    let files = generate_contract_files(&ir, &ethers_config());
    let eth = &files["Vault.ethers.ts"];
    assert!(eth.contains("depositUint256("));
    assert!(eth.contains("depositUint256Address("));
}

#[test]
fn vault_tuple_output_in_ethers() {
    let ir = parse_artifact("Vault", &fixture("vault.json")).unwrap();
    let files = generate_contract_files(&ir, &ethers_config());
    let eth = &files["Vault.ethers.ts"];
    assert!(eth.contains("getPosition(user: string): Promise<{"));
    assert!(eth.contains("shares: bigint"));
}

#[test]
fn vault_array_input_in_ethers() {
    let ir = parse_artifact("Vault", &fixture("vault.json")).unwrap();
    let files = generate_contract_files(&ir, &ethers_config());
    let eth = &files["Vault.ethers.ts"];
    assert!(eth.contains("getBalances(users: string[]): Promise<bigint[]>"));
}

#[test]
fn vault_fixed_array_in_viem_abi() {
    let ir = parse_artifact("Vault", &fixture("vault.json")).unwrap();
    let files = generate_contract_files(&ir, &viem_config());
    let abi = &files["Vault.abi.ts"];
    assert!(abi.contains("uint256[3]"));
}

#[test]
fn vault_solidity_reconstructs_named_structs() {
    let ir = parse_artifact("Vault", &fixture("vault.json")).unwrap();
    let out = render_solidity_file(&ir);
    assert!(out.contains("struct Position {"));
    assert!(out.contains("uint256 shares;"));
    assert!(out.contains("event Deposited(address indexed user, uint256 amount, uint256 shares);"));
    assert!(out.contains("error InsufficientShares(uint256 requested, uint256 available);"));
    assert!(out.contains(
        "function getPosition(address user) external view returns (Position memory position);"
    ));
}

// ── Minimal pipeline ────────────────────────────────────────────────────────

#[test]
fn minimal_produces_abi_and_viem_wrapper() {
    let ir = parse_artifact("Minimal", &fixture("minimal.json")).unwrap();
    let files = generate_contract_files(&ir, &viem_config());
    let viem = &files["Minimal.viem.ts"];
    assert!(viem.contains("export function getMinimalContract("));
    // No write functions → no params types exported
    assert!(!viem.contains("export type"));
}

// ── Unnamed params ────────────────────────────────────────────────────────

fn unnamed_params_json() -> &'static str {
    r#"{
        "abi": [
            {
                "type": "function",
                "name": "swap",
                "inputs": [
                    {"name": "", "type": "address", "internalType": "address", "components": []},
                    {"name": "", "type": "uint256", "internalType": "uint256", "components": []}
                ],
                "outputs": [
                    {"name": "", "type": "uint256", "internalType": "uint256", "components": []}
                ],
                "stateMutability": "nonpayable"
            },
            {
                "type": "event",
                "name": "Swapped",
                "inputs": [
                    {"name": "", "type": "address", "internalType": "address", "indexed": true, "components": []}
                ],
                "anonymous": false
            }
        ]
    }"#
}

#[test]
fn unnamed_params_viem_uses_positional_names() {
    let ir = parse_artifact("Unnamed", unnamed_params_json()).unwrap();
    let out = render_viem_file(&ir);
    assert!(out.contains("arg0: `0x${string}`"));
    assert!(out.contains("arg1: bigint"));
    // Must not contain empty param name (line starting with `:`)
    for line in out.lines() {
        let trimmed = line.trim();
        assert!(
            !trimmed.starts_with(':'),
            "found empty param name in line: {}",
            line
        );
    }
}

#[test]
fn unnamed_params_ethers_uses_positional_names() {
    let ir = parse_artifact("Unnamed", unnamed_params_json()).unwrap();
    let out = render_ethers_file(&ir);
    assert!(out.contains("swap(arg0: string, arg1: bigint)"));
    // Event filter with unnamed indexed param
    assert!(out.contains("Swapped(arg0?: string | null)"));
}

// ── Reserved words ────────────────────────────────────────────────────────

fn reserved_word_json() -> &'static str {
    r#"{
        "abi": [{
            "type": "function",
            "name": "setConfig",
            "inputs": [
                {"name": "class", "type": "uint256", "internalType": "uint256", "components": []},
                {"name": "delete", "type": "address", "internalType": "address", "components": []}
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        }]
    }"#
}

#[test]
fn reserved_word_params_viem_escaped() {
    let ir = parse_artifact("Reserved", reserved_word_json()).unwrap();
    let out = render_viem_file(&ir);
    assert!(out.contains("_class: bigint"));
    assert!(out.contains("_delete: `0x${string}`"));
}

#[test]
fn reserved_word_params_ethers_escaped() {
    let ir = parse_artifact("Reserved", reserved_word_json()).unwrap();
    let out = render_ethers_file(&ir);
    assert!(out.contains("setConfig(_class: bigint, _delete: string)"));
}

// ── Empty ABI contract ─────────────────────────────────────────────────────

fn empty_abi_json() -> &'static str {
    r#"{ "abi": [] }"#
}

#[test]
fn empty_abi_produces_valid_abi_ts() {
    let ir = parse_artifact("Empty", empty_abi_json()).unwrap();
    let abi = render_abi_file(&ir);
    assert!(abi.contains("export const EmptyAbi = [] as const;"));
}

#[test]
fn empty_abi_viem_has_get_contract_but_no_params() {
    let ir = parse_artifact("Empty", empty_abi_json()).unwrap();
    let viem = render_viem_file(&ir);
    assert!(viem.contains("export function getEmptyContract("));
    assert!(
        !viem.contains("export type"),
        "empty ABI should produce no param types"
    );
}

#[test]
fn empty_abi_zod_emits_valid_import_only_file() {
    let ir = parse_artifact("Empty", empty_abi_json()).unwrap();
    let out = render_zod_file(&ir);
    assert!(out.contains("import * as z from 'zod';"));
    assert!(!out.contains("export const Empty"));
}

// ── Error-only contract ────────────────────────────────────────────────────

fn error_only_json() -> &'static str {
    r#"{
        "abi": [
            {
                "type": "error",
                "name": "Unauthorized",
                "inputs": [
                    {"name": "caller", "type": "address", "internalType": "address", "components": []}
                ]
            }
        ]
    }"#
}

#[test]
fn error_only_abi_contains_error() {
    let ir = parse_artifact("AuthGate", error_only_json()).unwrap();
    let abi = render_abi_file(&ir);
    assert!(abi.contains("\"Unauthorized\""));
    assert!(abi.contains("\"error\""));
}

#[test]
fn error_only_ethers_has_empty_interface() {
    let ir = parse_artifact("AuthGate", error_only_json()).unwrap();
    let eth = render_ethers_file(&ir);
    assert!(eth.contains("export interface AuthGateContract {"));
    // Interface should close immediately with no function signatures inside
    assert!(!eth.contains("): Promise<"));
    // No event filters either
    assert!(!eth.contains("filters:"));
}

// ── Three-way function overloads ───────────────────────────────────────────

fn three_way_overload_json() -> &'static str {
    r#"{
        "abi": [
            {
                "type": "function",
                "name": "process",
                "inputs": [
                    {"name": "amount", "type": "uint256", "internalType": "uint256", "components": []}
                ],
                "outputs": [],
                "stateMutability": "nonpayable"
            },
            {
                "type": "function",
                "name": "process",
                "inputs": [
                    {"name": "to", "type": "address", "internalType": "address", "components": []},
                    {"name": "amount", "type": "uint256", "internalType": "uint256", "components": []}
                ],
                "outputs": [],
                "stateMutability": "nonpayable"
            },
            {
                "type": "function",
                "name": "process",
                "inputs": [
                    {"name": "to", "type": "address", "internalType": "address", "components": []},
                    {"name": "amount", "type": "uint256", "internalType": "uint256", "components": []},
                    {"name": "data", "type": "bytes", "internalType": "bytes", "components": []}
                ],
                "outputs": [],
                "stateMutability": "nonpayable"
            }
        ]
    }"#
}

#[test]
fn three_way_overloads_viem_params() {
    let ir = parse_artifact("Overloaded", three_way_overload_json()).unwrap();
    let viem = render_viem_file(&ir);
    assert!(
        viem.contains("export type OverloadedProcessUint256Params = {"),
        "first overload should be OverloadedProcessUint256Params"
    );
    assert!(
        viem.contains("export type OverloadedProcessAddressUint256Params = {"),
        "second overload should be OverloadedProcessAddressUint256Params"
    );
    assert!(
        viem.contains("export type OverloadedProcessAddressUint256BytesParams = {"),
        "third overload should be OverloadedProcessAddressUint256BytesParams"
    );
}

#[test]
fn three_way_overloads_ethers_suffixed() {
    let ir = parse_artifact("Processor", three_way_overload_json()).unwrap();
    let eth = render_ethers_file(&ir);
    assert!(
        eth.contains("processUint256("),
        "first overload should be processUint"
    );
    assert!(
        eth.contains("processAddressUint256("),
        "second overload should be processAddressUint"
    );
    assert!(
        eth.contains("processAddressUint256Bytes("),
        "third overload should be process_2"
    );
}

// ── All state mutabilities in ethers ───────────────────────────────────────

fn all_mutabilities_json() -> &'static str {
    r#"{
        "abi": [
            {
                "type": "function",
                "name": "compute",
                "inputs": [{"name": "x", "type": "uint256", "internalType": "uint256", "components": []}],
                "outputs": [{"name": "", "type": "uint256", "internalType": "uint256", "components": []}],
                "stateMutability": "pure"
            },
            {
                "type": "function",
                "name": "getBalance",
                "inputs": [],
                "outputs": [{"name": "", "type": "uint256", "internalType": "uint256", "components": []}],
                "stateMutability": "view"
            },
            {
                "type": "function",
                "name": "donate",
                "inputs": [],
                "outputs": [],
                "stateMutability": "payable"
            },
            {
                "type": "function",
                "name": "reset",
                "inputs": [],
                "outputs": [],
                "stateMutability": "nonpayable"
            }
        ]
    }"#
}

#[test]
fn pure_fn_returns_promise_value() {
    let ir = parse_artifact("Mutability", all_mutabilities_json()).unwrap();
    let eth = render_ethers_file(&ir);
    assert!(
        eth.contains("compute(x: bigint): Promise<bigint>"),
        "pure function should return Promise<T>"
    );
}

#[test]
fn view_fn_returns_promise_value() {
    let ir = parse_artifact("Mutability", all_mutabilities_json()).unwrap();
    let eth = render_ethers_file(&ir);
    assert!(
        eth.contains("getBalance(): Promise<bigint>"),
        "view function should return Promise<T>"
    );
}

#[test]
fn payable_fn_returns_transaction_response() {
    let ir = parse_artifact("Mutability", all_mutabilities_json()).unwrap();
    let eth = render_ethers_file(&ir);
    assert!(
        eth.contains("donate(): Promise<ContractTransactionResponse>"),
        "payable function should return Promise<ContractTransactionResponse>"
    );
}

#[test]
fn nonpayable_fn_returns_transaction_response() {
    let ir = parse_artifact("Mutability", all_mutabilities_json()).unwrap();
    let eth = render_ethers_file(&ir);
    assert!(
        eth.contains("reset(): Promise<ContractTransactionResponse>"),
        "nonpayable function should return Promise<ContractTransactionResponse>"
    );
}

// ── Barrel with multiple contracts ─────────────────────────────────────────

#[test]
fn barrel_three_contracts_alphabetical_ordering() {
    let names = vec!["Alpha".to_string(), "Beta".to_string(), "Gamma".to_string()];
    let out = render_barrel(&names, &default_config());

    // All three must be present
    assert!(out.contains("export * from './Alpha.abi.js'"));
    assert!(out.contains("export * from './Beta.abi.js'"));
    assert!(out.contains("export * from './Gamma.abi.js'"));

    // Verify alphabetical ordering is preserved: Alpha before Beta before Gamma
    let alpha_pos = out.find("Alpha").expect("Alpha export missing");
    let beta_pos = out.find("Beta").expect("Beta export missing");
    let gamma_pos = out.find("Gamma").expect("Gamma export missing");
    assert!(
        alpha_pos < beta_pos && beta_pos < gamma_pos,
        "barrel exports should preserve alphabetical ordering"
    );
}

// ── ABI writer preserves raw JSON ──────────────────────────────────────────

fn custom_abi_json() -> &'static str {
    r#"{
        "abi": [
            {
                "type": "function",
                "name": "setThreshold",
                "inputs": [
                    {"name": "newThreshold", "type": "uint128", "internalType": "uint128", "components": []}
                ],
                "outputs": [
                    {"name": "ok", "type": "bool", "internalType": "bool", "components": []}
                ],
                "stateMutability": "nonpayable"
            }
        ]
    }"#
}

#[test]
fn abi_writer_preserves_field_values() {
    let ir = parse_artifact("Threshold", custom_abi_json()).unwrap();
    let abi = render_abi_file(&ir);

    // Exact field values from the original JSON must be present
    assert!(
        abi.contains("\"setThreshold\""),
        "function name must be preserved"
    );
    assert!(
        abi.contains("\"newThreshold\""),
        "param name must be preserved"
    );
    assert!(
        abi.contains("\"uint128\""),
        "specific type string uint128 must be preserved"
    );
    assert!(
        abi.contains("\"bool\""),
        "output type bool must be preserved"
    );
    assert!(
        abi.contains("\"nonpayable\""),
        "state mutability must be preserved"
    );
    assert!(abi.contains("export const ThresholdAbi ="));
    assert!(abi.contains("] as const;"));
}

// ── YAML target ───────────────────────────────────────────────────────────

#[test]
fn erc20_yaml_target_produces_one_file() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let files = generate_contract_files(&ir, &yaml_config());
    assert!(files.contains_key("ERC20.yaml"));
    assert!(!files.contains_key("ERC20.abi.ts"));
    assert_eq!(files.len(), 1);
}

#[test]
fn erc20_yaml_target_ignores_wrappers_flag() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let mut cfg = yaml_config();
    cfg.wrappers = false;
    let files = generate_contract_files(&ir, &cfg);
    assert_eq!(files.len(), 1);
    assert!(files.contains_key("ERC20.yaml"));
}

#[test]
fn erc20_yaml_correct_structure() {
    let ir = parse_artifact("ERC20", &fixture("erc20.json")).unwrap();
    let out = render_yaml_file(&ir);
    assert!(out.contains("name: \"ERC20\""));
    assert!(out.contains("functions:"));
    assert!(out.contains("events:"));
    assert!(out.contains("errors:"));
    assert!(out.contains("name: \"totalSupply\""));
    assert!(out.contains("name: \"transfer\""));
    assert!(out.contains("name: \"Transfer\""));
    assert!(out.contains("stateMutability: \"view\""));
}

#[test]
fn vault_yaml_has_overloaded_functions() {
    let ir = parse_artifact("Vault", &fixture("vault.json")).unwrap();
    let out = render_yaml_file(&ir);
    // Both deposit overloads should be present
    let deposit_count = out.matches("name: \"deposit\"").count();
    assert!(
        deposit_count >= 2,
        "Expected at least 2 deposit functions, found {}",
        deposit_count
    );
}

#[test]
fn vault_yaml_has_events_and_errors() {
    let ir = parse_artifact("Vault", &fixture("vault.json")).unwrap();
    let out = render_yaml_file(&ir);
    assert!(
        out.contains("name: \"Deposited\""),
        "Expected Deposited event, got:\n{out}"
    );
    assert!(
        out.contains("name: \"InsufficientShares\""),
        "Expected InsufficientShares error, got:\n{out}"
    );
}

#[test]
fn empty_abi_yaml_has_empty_sections() {
    let ir = parse_artifact("Empty", empty_abi_json()).unwrap();
    let out = render_yaml_file(&ir);
    assert!(out.contains("name: \"Empty\""));
    assert!(out.contains("functions: []"));
    assert!(out.contains("events: []"));
    assert!(out.contains("errors: []"));
}

#[test]
fn unnamed_params_yaml_omits_name_key() {
    let ir = parse_artifact("Unnamed", unnamed_params_json()).unwrap();
    let out = render_yaml_file(&ir);
    // The unnamed address param should have type but no name
    assert!(
        out.contains("type: \"address\""),
        "Expected address type, got:\n{out}"
    );
    assert!(
        out.contains("type: \"uint256\""),
        "Expected uint256 type, got:\n{out}"
    );
}

#[test]
fn yaml_config_roundtrip() {
    let toml = "[abi-typegen]\ntarget = \"yaml\"\n";
    let cfg = Config::from_toml_str(toml).unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Yaml);
}

#[test]
fn yaml_config_yml_alias() {
    let toml = "[abi-typegen]\ntarget = \"yml\"\n";
    let cfg = Config::from_toml_str(toml).unwrap();
    assert_eq!(*cfg.target(), abi_typegen_config::Target::Yaml);
}
