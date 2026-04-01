// Validates all generated ethers v6 output against real ethers API types.
// Compile with: pnpm exec tsc --noEmit

import { JsonRpcProvider, type ContractTransactionResponse, type EventFilter } from 'ethers';

import {
  type TokenContract,
  connectToken,
} from './generated/Token.ethers.js';
import {
  type VaultContract,
  connectVault,
} from './generated/Vault.ethers.js';
import {
  type RegistryContract,
  connectRegistry,
} from './generated/Registry.ethers.js';
import {
  type EdgeCasesContract,
  connectEdgeCases,
} from './generated/EdgeCases.ethers.js';

const provider = new JsonRpcProvider('http://localhost:8545');

// ── Connect factories ──────────────────────────────────────────────────────
const token: TokenContract = connectToken('0x1', provider);
const vault: VaultContract = connectVault('0x2', provider);
const registry: RegistryContract = connectRegistry('0x3', provider);
const edge: EdgeCasesContract = connectEdgeCases('0x4', provider);

// ── Token ──────────────────────────────────────────────────────────────────
async function validateToken(t: TokenContract) {
  // View → Promise<T>
  const name: string = await t.name();
  const symbol: string = await t.symbol();
  const decimals: number = await t.decimals();
  const totalSupply: bigint = await t.totalSupply();
  const balance: bigint = await t.balanceOf('0xabc');
  const allowance: bigint = await t.allowance('0xabc', '0xdef');

  // Write → Promise<ContractTransactionResponse>
  const tx1: ContractTransactionResponse = await t.transfer('0xdef', 100n);
  const tx2: ContractTransactionResponse = await t.approve('0xdef', 200n);
  const tx3: ContractTransactionResponse = await t.mint('0xdef', 300n);

  // Event filters
  const f1: EventFilter = t.filters.Transfer('0xabc', '0xdef');
  const f2: EventFilter = t.filters.Transfer(null, null);
  const f3: EventFilter = t.filters.Approval('0xabc', null);

  void name; void symbol; void decimals; void totalSupply; void balance;
  void allowance; void tx1; void tx2; void tx3; void f1; void f2; void f3;
}

// ── Vault ──────────────────────────────────────────────────────────────────
async function validateVault(v: VaultContract) {
  // Overloaded deposits
  const tx0: ContractTransactionResponse = await v.depositUint256Address(100n, '0xabc');
  const tx1: ContractTransactionResponse = await v.depositUint256(200n);

  // Struct return → inline object
  const pos = await v.getPosition('0xabc');
  const shares: bigint = pos.shares;
  const depositedAt: bigint = pos.depositedAt;
  const posToken: string = pos.token;

  // Public mapping auto-accessor → tuple
  const mapping = await v.positions('0xabc');

  // Array I/O
  const balances: bigint[] = await v.getBalances(['0xabc']);
  const matrix: bigint[] = await v.getMatrix();

  // Event filters
  const f1: EventFilter = v.filters.Deposited('0xabc');
  const f2: EventFilter = v.filters.Deposited(null);
  const f3: EventFilter = v.filters.BatchProcessed();

  void tx0; void tx1; void shares; void depositedAt; void posToken;
  void mapping; void balances; void matrix; void f1; void f2; void f3;
}

// ── Registry ───────────────────────────────────────────────────────────────
async function validateRegistry(r: RegistryContract) {
  const id = '0x0000000000000000000000000000000000000000000000000000000000000001';

  // Write
  const tx: ContractTransactionResponse = await r.register(id, 'label');

  // Nested struct return
  const entry = await r.getEntry(id);
  const entryId: string = entry.id;
  const label: string = entry.label;
  const createdAt: bigint = entry.meta.createdAt;
  const owner: string = entry.meta.owner;
  const active: boolean = entry.meta.active;

  // Multi-return → Promise<[T1, T2]>
  const batch: [string[], string[]] = await r.batchLookup([id]);

  // Pure functions
  const hash: string = await r.hashLabel('test');
  const encoded: string = await r.encode(42n, '0xabc');

  // Event filter with bytes32 indexed param
  const f: EventFilter = r.filters.Registered(id, '0xabc');

  void tx; void entryId; void label; void createdAt; void owner; void active;
  void batch; void hash; void encoded; void f;
}

// ── EdgeCases ──────────────────────────────────────────────────────────────
async function validateEdgeCases(e: EdgeCasesContract) {
  // ── Integer width boundary: uint8-uint48 → number, uint64+ → bigint ──
  const small = await e.smallInts();
  const a: number = small.a; // uint8
  const b: number = small.b; // uint16
  const c: number = small.c; // uint32
  const d: number = small.d; // uint48

  const large = await e.largeInts();
  const e1: bigint = large.a; // uint64
  const e2: bigint = large.b; // uint128
  const e3: bigint = large.c; // uint256

  // ── Signed ints: same boundary ───────────────────────────────────────
  const signed = await e.signedInts();
  const s1: number = signed.a; // int8
  const s2: number = signed.b; // int48
  const s3: bigint = signed.c; // int256

  // ── Bytes variants ───────────────────────────────────────────────────
  const fb = await e.fixedBytes();
  const b1: string = fb.a; // bytes1
  const b16: string = fb.b; // bytes16
  const b32: string = fb.c; // bytes32

  const db: string = await e.dynamicBytes(); // bytes

  // ── Nested arrays ────────────────────────────────────────────────────
  const nested: bigint[][] = await e.nestedArray(); // uint256[][]

  // ── Fixed array of addresses ─────────────────────────────────────────
  const addrs: string[] = await e.fixedArrayOfAddresses(); // address[3]

  // ── Multiple named returns ───────────────────────────────────────────
  const multi = await e.multiReturn();
  const count: bigint = multi.count;
  const total: bigint = multi.total;
  const flag: boolean = multi.flag;

  // ── Zero-param write ─────────────────────────────────────────────────
  const resetTx: ContractTransactionResponse = await e.reset();

  // ── Payable write ────────────────────────────────────────────────────
  const fundTx: ContractTransactionResponse = await e.fund();

  // ── Struct with all types ────────────────────────────────────────────
  const complex = await e.getComplex();
  const cId: bigint = complex.id;
  const cOwner: string = complex.owner;
  const cHash: string = complex.hash;
  const cActive: boolean = complex.active;
  const cName: string = complex.name;
  const cTimestamp: bigint = complex.timestamp;

  // ── Struct as input parameter ────────────────────────────────────────
  const processResult: string = await e.processComplex({
    id: 1n,
    owner: '0x0000000000000000000000000000000000000001',
    hash: '0x0000000000000000000000000000000000000000000000000000000000000000',
    active: true,
    name: 'test',
    timestamp: 12345n,
  });

  // ── Event filters ────────────────────────────────────────────────────
  // Anonymous event — no indexed params
  const debugFilter: EventFilter = e.filters.DebugLog();
  // Mixed indexed: only `id` is indexed
  const itemFilter: EventFilter = e.filters.ItemCreated(42n);
  // Both indexed
  const transferFilter: EventFilter = e.filters.Transfer('0xabc', '0xdef');

  void a; void b; void c; void d; void e1; void e2; void e3;
  void s1; void s2; void s3; void b1; void b16; void b32; void db;
  void nested; void addrs; void count; void total; void flag;
  void resetTx; void fundTx;
  void cId; void cOwner; void cHash; void cActive; void cName; void cTimestamp;
  void processResult;
  void debugFilter; void itemFilter; void transferFilter;
}

void token; void vault; void registry; void edge;
void validateToken; void validateVault; void validateRegistry; void validateEdgeCases;
