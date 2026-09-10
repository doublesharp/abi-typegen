// Exercise generated bindings with real ethers runtimes and offline call data.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import ts from 'typescript';
import * as v6 from 'ethers';
import { ethers as v5 } from 'ethers5';

const binary = process.env.ABI_TYPEGEN_BIN ?? fileURLToPath(new URL('../../target/debug/abi-typegen', import.meta.url));
const address = '0x0000000000000000000000000000000000000011';
const other = '0x0000000000000000000000000000000000000022';
const abi = [
  { type: 'function', name: 'quote', stateMutability: 'view', inputs: [{ name: 'amount', type: 'uint256' }], outputs: [{ type: 'uint256' }] },
  { type: 'function', name: 'quote', stateMutability: 'view', inputs: [{ name: 'account', type: 'address' }], outputs: [{ type: 'uint256' }] },
  { type: 'function', name: 'quoteUint256', stateMutability: 'view', inputs: [], outputs: [{ type: 'uint256' }] },
  { type: 'function', name: 'nested', stateMutability: 'view', inputs: [{ name: 'items', type: 'tuple[][2]', components: [{ name: 'amount', type: 'uint256' }, { name: 'ok', type: 'bool' }] }], outputs: [{ type: 'uint256' }] },
  { type: 'function', name: 'nested', stateMutability: 'view', inputs: [{ name: 'value', type: 'bytes32' }], outputs: [{ type: 'uint256' }] },
  { type: 'event', name: 'Moved', inputs: [
    { name: 'amount', type: 'uint256', indexed: false },
    { name: 'from', type: 'address', indexed: true },
    { name: 'memo', type: 'bytes32', indexed: false },
    { name: 'to', type: 'address', indexed: true },
  ] },
];

const dir = mkdtempSync(join(tmpdir(), 'abi-ethers-runtime-'));
try {
  const input = join(dir, 'abi.json');
  writeFileSync(input, JSON.stringify(abi));
  writeFileSync(join(dir, 'package.json'), '{"type":"module"}');
  for (const [target, runtime] of [['ethers', v6], ['ethers5', v5]]) {
    const output = join(dir, target);
    const config = join(dir, `${target}.toml`);
    writeFileSync(config, `[abi-typegen]\ntarget = "${target}"\nout = ${JSON.stringify(output)}\n`);
    execFileSync(binary, ['--config', config, 'fetch', '--file', input, '--name', 'Runtime', '--artifacts', join(dir, target + '-artifacts')]);
    for (const name of readdirSync(output).filter(name => name.endsWith('.ts'))) {
      const source = readFileSync(join(output, name), 'utf8')
        .replaceAll("from 'ethers'", `from '${import.meta.resolve(target === 'ethers' ? 'ethers' : 'ethers5')}'`);
      const compiled = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
      writeFileSync(join(output, name.replace(/\.ts$/, '.js')), compiled);
    }
    const { connectRuntime } = await import(pathToFileURL(join(output, `Runtime.${target}.js`)));
    const iface = target === 'ethers' ? new runtime.Interface(abi) : new runtime.utils.Interface(abi);
    const calls = [];
    const call = async transaction => {
      calls.push(transaction.data);
      const fragment = iface.getFunction(transaction.data.slice(0, 10));
      return iface.encodeFunctionResult(fragment, [calls.length]);
    };
    let runner;
    if (target === 'ethers') {
      runner = { call };
    } else {
      class OfflineProvider extends runtime.providers.BaseProvider {
        constructor() { super({ chainId: 1, name: 'offline' }); }
        async detectNetwork() { return { chainId: 1, name: 'offline' }; }
        async perform(method, params) {
          assert.equal(method, 'call');
          return call(params.transaction);
        }
      }
      runner = new OfflineProvider();
    }
    const contract = connectRuntime(address, runner);
    assert.equal((await contract.quoteUint256_2(7)).toString(), '1');
    assert.equal((await contract.quoteAddress(other)).toString(), '2');
    assert.equal((await contract.quoteUint256()).toString(), '3');
    const nested = [[{ amount: 9, ok: true }], []];
    assert.equal((await contract.nestedTupleUint256BoolEndTupleArrayArray2(nested)).toString(), '4');
    assert.deepEqual(calls, [
      iface.encodeFunctionData('quote(uint256)', [7]),
      iface.encodeFunctionData('quote(address)', [other]),
      iface.encodeFunctionData('quoteUint256()', []),
      iface.encodeFunctionData('nested((uint256,bool)[][2])', [nested]),
    ]);
    const filter = contract.filters.Moved(null, address, null, other);
    const topics = target === 'ethers' ? await filter.getTopicFilter() : filter.topics;
    assert.deepEqual(topics, iface.encodeFilterTopics('Moved', [null, address, null, other]));
    console.log(`${target}: generated overload calls, alias collisions, nested tuples, and event topics passed`);
  }
} finally {
  rmSync(dir, { recursive: true, force: true });
}
