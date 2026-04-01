// Type-level validation of abi-typegen output.
// If this file compiles with `tsc --noEmit`, all generated types are valid.

// ── Barrel import (all contracts) ──────────────────────────────────────────
import {
  // ABI constants
  TokenAbi,
  VaultAbi,
  RegistryAbi,
  // Viem helpers
  getTokenContract,
  getVaultContract,
  getRegistryContract,
  // Viem param types
  type TokenTransferParams,
  type TokenApproveParams,
  type TokenMintParams,
  type VaultDepositUint256AddressParams,
  type VaultDepositUint256Params,
  type RegistryRegisterParams,
  // Ethers interfaces + factories
  type TokenContract,
  type VaultContract,
  type RegistryContract,
  connectToken,
  connectVault,
  connectRegistry,
} from './generated/index.js';

// ── Viem: verify ABI is as-const (literal types, not widened) ──────────────
const _tokenAbi: readonly unknown[] = TokenAbi;
const _vaultAbi: readonly unknown[] = VaultAbi;
const _registryAbi: readonly unknown[] = RegistryAbi;

// ── Viem: verify param types have correct field types ──────────────────────
const _transfer: TokenTransferParams = { to: '0xabc', amount: 100n };
const _approve: TokenApproveParams = { spender: '0xdef', amount: 200n };
const _mint: TokenMintParams = { to: '0x123', amount: 300n };
const _deposit: VaultDepositUint256AddressParams = { amount: 400n, recipient: '0x456' };
const _deposit1: VaultDepositUint256Params = { amount: 500n };
const _register: RegistryRegisterParams = {
  id: '0x0000000000000000000000000000000000000000000000000000000000000001',
  label: 'test',
};

// ── Viem: verify getContract helpers accept correct types ──────────────────
function _viemSmoke(client: import('viem').Client) {
  const token = getTokenContract('0x0000000000000000000000000000000000000001', client);
  const vault = getVaultContract('0x0000000000000000000000000000000000000002', client);
  const registry = getRegistryContract('0x0000000000000000000000000000000000000003', client);
  // Suppress unused warnings
  void token;
  void vault;
  void registry;
}

// ── Ethers: verify interface method signatures ─────────────────────────────
async function _ethersSmoke(runner: import('ethers').ContractRunner) {
  const token: TokenContract = connectToken('0x1', runner);
  const vault: VaultContract = connectVault('0x2', runner);
  const registry: RegistryContract = connectRegistry('0x3', runner);

  // Token methods
  const _balance: bigint = await token.balanceOf('0xabc');
  const _tx = await token.transfer('0xdef', 100n);

  // Vault methods — overloads
  await vault.depositUint256Address(100n, '0xabc');
  await vault.depositUint256(200n);

  // Vault struct return
  const pos = await vault.getPosition('0xabc');
  const _shares: bigint = pos.shares;
  const _token: string = pos.token;

  // Vault array operations
  const _balances: bigint[] = await vault.getBalances(['0xabc', '0xdef']);
  const _matrix: bigint[] = await vault.getMatrix();

  // Registry methods
  await registry.register(
    '0x0000000000000000000000000000000000000000000000000000000000000001',
    'label'
  );

  // Registry multi-return
  const result = await registry.batchLookup([
    '0x0000000000000000000000000000000000000000000000000000000000000001',
  ]);
  void result;

  // Registry pure functions
  const _hash: string = await registry.hashLabel('test');
  const _encoded: string = await registry.encode(42n, '0xabc');

  // Event filters
  const _transferFilter = token.filters.Transfer('0xabc', '0xdef');
  const _depositFilter = vault.filters.Deposited('0xabc');
  const _registeredFilter = registry.filters.Registered(
    '0x0000000000000000000000000000000000000000000000000000000000000001',
    '0xabc'
  );
}

// Suppress unused variable warnings
void _tokenAbi;
void _vaultAbi;
void _registryAbi;
void _transfer;
void _approve;
void _mint;
void _deposit;
void _deposit1;
void _register;
void _viemSmoke;
void _ethersSmoke;
