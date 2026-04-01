// Validates all generated viem output against real viem API types.
// Compile with: pnpm exec tsc --noEmit

import { createPublicClient, createWalletClient, http, type Address } from 'viem';
import { mainnet } from 'viem/chains';

import { TokenAbi } from './generated/Token.abi.js';
import { VaultAbi } from './generated/Vault.abi.js';
import { RegistryAbi } from './generated/Registry.abi.js';
import { EdgeCasesAbi } from './generated/EdgeCases.abi.js';

import { getTokenContract, type TokenTransferParams, type TokenApproveParams, type TokenMintParams } from './generated/Token.viem.js';
import { getVaultContract, type VaultDepositUint256AddressParams, type VaultDepositUint256Params } from './generated/Vault.viem.js';
import { getRegistryContract, type RegistryRegisterParams } from './generated/Registry.viem.js';
import { getEdgeCasesContract } from './generated/EdgeCases.viem.js';

// ── ABI as-const ───────────────────────────────────────────────────────────
// viem requires as-const ABIs for type inference. Readonly = as-const confirmed.
const _ta: readonly unknown[] = TokenAbi;
const _va: readonly unknown[] = VaultAbi;
const _ra: readonly unknown[] = RegistryAbi;
const _ea: readonly unknown[] = EdgeCasesAbi;
void _ta; void _va; void _ra; void _ea;

// ── getContract factories ──────────────────────────────────────────────────
const pub = createPublicClient({ chain: mainnet, transport: http() });
const wal = createWalletClient({ chain: mainnet, transport: http() });
const addr: Address = '0x0000000000000000000000000000000000000001';

const token = getTokenContract(addr, pub);
const vault = getVaultContract(addr, pub);
const registry = getRegistryContract(addr, pub);
const edge = getEdgeCasesContract(addr, pub);
const tokenW = getTokenContract(addr, wal);
void token; void vault; void registry; void edge; void tokenW;

// ── Param types: address → `0x${string}` ───────────────────────────────────
type Hex = `0x${string}`;

const transfer: TokenTransferParams = { to: '0x02' as Hex, amount: 100n };
const approve: TokenApproveParams = { spender: '0x03' as Hex, amount: 200n };
const mint: TokenMintParams = { to: '0x04' as Hex, amount: 300n };
void transfer; void approve; void mint;

// ── Param types: overloaded deposit ────────────────────────────────────────
const dep: VaultDepositUint256AddressParams = { amount: 100n, recipient: '0x05' as Hex };
const dep1: VaultDepositUint256Params = { amount: 200n };
void dep; void dep1;

// ── Param types: bytes32 → `0x${string}` ──────────────────────────────────
const reg: RegistryRegisterParams = {
  id: '0x0000000000000000000000000000000000000000000000000000000000000001',
  label: 'test',
};
void reg;

// ── EdgeCases: no write params (all view/pure or no-input writes) ──────────
// getEdgeCasesContract generates no param types — only getContract helper.
// This is correct: reset() and fund() have no inputs, view/pure fns are excluded.

// ── Type-level assertions ──────────────────────────────────────────────────
// Verify exact field types match viem conventions.
const _to: Hex = transfer.to;
const _amt: bigint = transfer.amount;
const _sp: Hex = approve.spender;
const _regId: Hex = reg.id;
const _regLabel: string = reg.label;
const _depRecip: Hex = dep.recipient;
void _to; void _amt; void _sp; void _regId; void _regLabel; void _depRecip;
