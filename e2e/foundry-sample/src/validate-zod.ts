// Validates generated Zod output against the real Zod 4 API.
// Compile with: pnpm exec tsc --noEmit -p tsconfig.zod.json

import * as z from 'zod';

import {
  TokenAbi,
  VaultAbi,
  RegistryAbi,
  TokenTransferParamsSchema,
  type TokenTransferParams,
  TokenBalanceOfResultSchema,
  type TokenBalanceOfResult,
  VaultDepositUint256AddressParamsSchema,
  type VaultDepositUint256AddressParams,
  VaultDepositUint256ParamsSchema,
  type VaultDepositUint256Params,
  VaultGetPositionResultSchema,
  type VaultGetPositionResult,
  RegistryRegisterParamsSchema,
  type RegistryRegisterParams,
  RegistryGetEntryResultSchema,
  type RegistryGetEntryResult,
} from './generated-zod/index.js';

const _tokenAbi: readonly unknown[] = TokenAbi;
const _vaultAbi: readonly unknown[] = VaultAbi;
const _registryAbi: readonly unknown[] = RegistryAbi;

const _transferSchema: z.ZodType<TokenTransferParams> = TokenTransferParamsSchema;
const _balanceSchema: z.ZodType<TokenBalanceOfResult> = TokenBalanceOfResultSchema;
const _depositSchema: z.ZodType<VaultDepositUint256AddressParams> =
  VaultDepositUint256AddressParamsSchema;
const _depositOverloadSchema: z.ZodType<VaultDepositUint256Params> =
  VaultDepositUint256ParamsSchema;
const _positionSchema: z.ZodType<VaultGetPositionResult> = VaultGetPositionResultSchema;
const _registerSchema: z.ZodType<RegistryRegisterParams> = RegistryRegisterParamsSchema;
const _entrySchema: z.ZodType<RegistryGetEntryResult> = RegistryGetEntryResultSchema;

const transfer: TokenTransferParams = TokenTransferParamsSchema.parse({
  to: '0x0000000000000000000000000000000000000001',
  amount: 100n,
});
const balance: TokenBalanceOfResult = TokenBalanceOfResultSchema.parse(123n);

const deposit: VaultDepositUint256AddressParams = VaultDepositUint256AddressParamsSchema.parse({
  amount: 100n,
  recipient: '0x0000000000000000000000000000000000000002',
});
const depositOverload: VaultDepositUint256Params = VaultDepositUint256ParamsSchema.parse({
  amount: 200n,
});

const position: VaultGetPositionResult = VaultGetPositionResultSchema.parse({
  shares: 1n,
  depositedAt: 2n,
  token: '0x0000000000000000000000000000000000000003',
});

const register: RegistryRegisterParams = RegistryRegisterParamsSchema.parse({
  id: '0x0000000000000000000000000000000000000000000000000000000000000001',
  label: 'label',
});

const entry: RegistryGetEntryResult = RegistryGetEntryResultSchema.parse({
  id: '0x0000000000000000000000000000000000000000000000000000000000000001',
  label: 'registry',
  meta: {
    createdAt: 123n,
    owner: '0x0000000000000000000000000000000000000004',
    active: true,
  },
});

const safeTransfer = TokenTransferParamsSchema.safeParse({
  to: '0x0000000000000000000000000000000000000005',
  amount: 999n,
});

if (safeTransfer.success) {
  const parsed: TokenTransferParams = safeTransfer.data;
  void parsed;
}

const _shares: bigint = position.shares;
const _token: string = position.token;
const _entryOwner: string = entry.meta.owner;
const _entryActive: boolean = entry.meta.active;

void _tokenAbi;
void _vaultAbi;
void _registryAbi;
void _transferSchema;
void _balanceSchema;
void _depositSchema;
void _depositOverloadSchema;
void _positionSchema;
void _registerSchema;
void _entrySchema;
void transfer;
void balance;
void deposit;
void depositOverload;
void register;
void _shares;
void _token;
void _entryOwner;
void _entryActive;