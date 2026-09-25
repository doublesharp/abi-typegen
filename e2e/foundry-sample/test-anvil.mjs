// Generated APIs must submit real transactions and decode real chain results.
import assert from "node:assert/strict";
import { test } from "node:test";
import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  readFileSync,
  writeFileSync,
  readdirSync,
  rmSync,
} from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import ts from "typescript";
import * as viem from "viem";
import { privateKeyToAccount } from "viem/accounts";
import * as ethers from "ethers";
import { ethers as ethers5 } from "ethers5";
import { Web3 } from "web3";

const url = process.env.ATG_RPC_URL;
assert.ok(url, "Run through e2e/native/anvil.py");
const address = process.env.ATG_TOKEN_ADDRESS;
const privateKey = process.env.ATG_PRIVATE_KEY;
const account = privateKeyToAccount(privateKey);
const chain = viem.defineChain({
  id: Number(process.env.ATG_CHAIN_ID),
  name: "Local binding test",
  nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
  rpcUrls: { default: { http: [url] } },
});
const client = viem.createPublicClient({
  chain,
  transport: viem.http(url),
  pollingInterval: 50,
});
assert.equal(await client.getChainId(), chain.id);
const binary =
  process.env.ABI_TYPEGEN_BIN ??
  fileURLToPath(new URL("../../target/debug/abi-typegen", import.meta.url));
const dir = mkdtempSync(
  join(fileURLToPath(new URL(".", import.meta.url)), ".anvil-"),
);
const amount = 1n << 128n;
const zero = "0x0000000000000000000000000000000000000000";
// Some SDKs unref their polling timers while requests are pending.
const keepAlive = setInterval(() => {}, 1000);
const sameAddress = (a, b) => assert.equal(a.toLowerCase(), b.toLowerCase());
const receipt = async (hash) => {
  const mined = await client.waitForTransactionReceipt({
    hash,
    timeout: 20_000,
  });
  assert.equal(mined.status, "success");
  return mined;
};
await test("Generated Anvil consumers", { timeout: 120_000 }, async (t) => {
  try {
    execFileSync(binary, [
      "generate",
      "--artifacts",
      fileURLToPath(new URL("out", import.meta.url)),
      "--contracts",
      "Token",
      "--target",
      "viem,ethers,ethers5,web3js,wagmi",
      "--out",
      dir,
    ]);
    writeFileSync(join(dir, "package.json"), '{"type":"module"}');
    for (const target of ["viem", "ethers", "ethers5", "web3js", "wagmi"]) {
      const output = join(dir, target);
      for (const name of readdirSync(output).filter((name) =>
        name.endsWith(".ts"),
      )) {
        const typedSource = readFileSync(join(output, name), "utf8").replaceAll(
          "from 'ethers'",
          target === "ethers5" ? "from 'ethers5'" : "from 'ethers'",
        );
        writeFileSync(join(output, name), typedSource);
        const source = typedSource.replace(
          /from '([^./][^']*)'/g,
          (_, specifier) =>
            `from '${import.meta.resolve(specifier === "ethers" && target === "ethers5" ? "ethers5" : specifier)}'`,
        );
        writeFileSync(
          join(output, name.replace(/\.ts$/, ".js")),
          ts.transpileModule(source, {
            compilerOptions: {
              target: ts.ScriptTarget.ES2022,
              module: ts.ModuleKind.ES2022,
            },
          }).outputText,
        );
      }
    }
    const typecheck = ts.createProgram(
      ["viem", "ethers", "ethers5", "web3js", "wagmi"].flatMap((target) =>
        readdirSync(join(dir, target))
          .filter((name) => name.endsWith(".ts"))
          .map((name) => join(dir, target, name)),
      ),
      {
        noEmit: true,
        strict: true,
        skipLibCheck: true,
        target: ts.ScriptTarget.ES2022,
        module: ts.ModuleKind.ESNext,
        moduleResolution: ts.ModuleResolutionKind.Bundler,
      },
    );
    const diagnostics = ts.getPreEmitDiagnostics(typecheck);
    assert.equal(
      diagnostics.length,
      0,
      ts.formatDiagnosticsWithColorAndContext(diagnostics, {
        getCanonicalFileName: (path) => path,
        getCurrentDirectory: () => dir,
        getNewLine: () => "\n",
      }),
    );
    const { TokenAbi } = await import(
      pathToFileURL(join(dir, "viem/Token.abi.js"))
    );
    const checkTransfer = (mined) => {
      const log = viem.decodeEventLog({ abi: TokenAbi, ...mined.logs[0] });
      assert.equal(log.eventName, "Transfer");
      sameAddress(log.args.to, account.address);
      assert.equal(log.args.amount, amount);
    };
    await t.test(
      "viem generated read/write, receipt, event, and revert",
      { timeout: 30_000 },
      async () => {
        const { getTokenContract } = await import(
          pathToFileURL(join(dir, "viem/Token.viem.js"))
        );
        const wallet = viem
          .createWalletClient({ account, chain, transport: viem.http(url) })
          .extend(viem.publicActions);
        const token = getTokenContract(address, wallet);
        const before = await token.read.balanceOf([account.address]);
        checkTransfer(
          await receipt(await token.write.mint([account.address, amount])),
        );
        assert.equal(
          await token.read.balanceOf([account.address]),
          before + amount,
        );
        await assert.rejects(
          token.simulate.transfer([zero, 1n]),
          /InvalidRecipient/,
        );
        await assert.rejects(
          token.simulate.transfer([account.address, (1n << 256n) - 1n]),
          /InsufficientBalance/,
        );
        await receipt(await token.write.approve([account.address, 17n]));
        assert.equal(
          await token.read.allowance([account.address, account.address]),
          17n,
        );
      },
    );
    for (const [target, sdk] of [
      ["ethers", ethers],
      ["ethers5", ethers5],
    ]) {
      await t.test(
        `${target} generated read/write, receipt, event, and revert`,
        { timeout: 30_000 },
        async () => {
          const { connectToken } = await import(
            pathToFileURL(join(dir, `${target}/Token.${target}.js`))
          );
          const provider =
            target === "ethers"
              ? new sdk.JsonRpcProvider(url)
              : new sdk.providers.JsonRpcProvider(url);
          const wallet = new sdk.Wallet(privateKey, provider);
          const signer =
            target === "ethers" ? new sdk.NonceManager(wallet) : wallet;
          try {
            const token = connectToken(address, signer);
            const before = BigInt(
              (await token.balanceOf(account.address)).toString(),
            );
            const mint = await token.mint(account.address, amount.toString());
            const mined = await mint.wait();
            assert.equal(mined.status, 1);
            const transfer = token.interface.parseLog(mined.logs[0]);
            assert.equal(transfer.name, "Transfer");
            assert.equal(BigInt(transfer.args.amount.toString()), amount);
            sameAddress(transfer.args.to, account.address);
            assert.equal(
              BigInt((await token.balanceOf(account.address)).toString()),
              before + amount,
            );
            await assert.rejects(
              target === "ethers"
                ? token.transfer.staticCall(zero, 1)
                : token.callStatic.transfer(zero, 1),
            );
            const approval = await token.approve(account.address, 19);
            assert.equal((await approval.wait()).status, 1);
            assert.equal(
              (
                await token.allowance(account.address, account.address)
              ).toString(),
              "19",
            );
          } finally {
            if (provider.destroy) provider.destroy();
            else provider.removeAllListeners();
          }
        },
      );
    }
    await t.test(
      "web3.js generated read/write, receipt, event, and revert",
      { timeout: 30_000 },
      async () => {
        const { createToken } = await import(
          pathToFileURL(join(dir, "web3js/Token.web3.js"))
        );
        const web3 = new Web3(url);
        web3.eth.accounts.wallet.add(privateKey);
        const token = createToken(web3, address);
        const before = await token.methods.balanceOf(account.address).call();
        const mined = await token.methods
          .mint(account.address, amount)
          .send({ from: account.address, gas: "300000" });
        assert.equal(mined.status, 1n);
        assert.equal(mined.events.Transfer.returnValues.amount, amount);
        sameAddress(mined.events.Transfer.returnValues.to, account.address);
        assert.equal(
          await token.methods.balanceOf(account.address).call(),
          before + amount,
        );
        await assert.rejects(
          token.methods.transfer(zero, 1).call({ from: account.address }),
        );
        await token.methods
          .approve(account.address, 23)
          .send({ from: account.address, gas: "300000" });
        assert.equal(
          await token.methods
            .allowance(account.address, account.address)
            .call(),
          23n,
        );
      },
    );
    await t.test(
      "wagmi generated React hooks submit, read, watch events, and report reverts",
      { timeout: 40_000 },
      async () => {
        const { JSDOM } = await import("jsdom");
        const dom = new JSDOM("<!doctype html><html><body></body></html>", {
          url: "http://localhost/",
        });
        globalThis.window = dom.window;
        globalThis.document = dom.window.document;
        globalThis.Event = dom.window.Event;
        globalThis.CustomEvent = dom.window.CustomEvent;
        Object.defineProperty(globalThis, "navigator", {
          value: dom.window.navigator,
          configurable: true,
        });
        globalThis.IS_REACT_ACT_ENVIRONMENT = true;
        const React = await import("react");
        const { renderHook, act, waitFor, cleanup } =
          await import("@testing-library/react");
        const { WagmiProvider, createConfig, http } = await import("wagmi");
        const { mock } = await import("wagmi/connectors");
        const { connect, disconnect } = await import("wagmi/actions");
        const { QueryClient, QueryClientProvider } =
          await import("@tanstack/react-query");
        const hooks = await import(
          pathToFileURL(join(dir, "wagmi/Token.wagmi.js"))
        );
        // This connector supplies wallet identity; transaction and read RPCs go to Anvil.
        const config = createConfig({
          storage: null,
          chains: [chain],
          connectors: [mock({ accounts: [account.address] })],
          transports: { [chain.id]: http(url) },
          pollingInterval: 50,
        });
        const queryClient = new QueryClient({
          defaultOptions: { queries: { retry: false, gcTime: 0 } },
        });
        const wrapper = ({ children }) =>
          React.createElement(
            WagmiProvider,
            { config, reconnectOnMount: false },
            React.createElement(
              QueryClientProvider,
              { client: queryClient },
              children,
            ),
          );
        const logs = [];
        try {
          await connect(config, { connector: config.connectors[0] });
          const mint = renderHook(() => hooks.useTokenMint(address), {
            wrapper,
          });
          const read = renderHook(
            () => hooks.useTokenBalanceOf(address, { arg0: account.address }),
            { wrapper },
          );
          renderHook(
            () => hooks.useTokenTransferEvent(address, (log) => logs.push(log)),
            { wrapper },
          );
          await waitFor(
            () => assert.equal(read.result.current.isSuccess, true),
            { timeout: 10_000 },
          );
          const before = read.result.current.data;
          await act(async () =>
            mint.result.current.write({ to: account.address, amount }),
          );
          await waitFor(
            () => {
              if (mint.result.current.error) throw mint.result.current.error;
              assert.ok(mint.result.current.data);
            },
            { timeout: 10_000 },
          );
          checkTransfer(await receipt(mint.result.current.data));
          await act(async () => {
            await read.result.current.refetch();
          });
          await waitFor(
            () => assert.equal(read.result.current.data, before + amount),
            { timeout: 10_000 },
          );
          await waitFor(
            () => assert.ok(logs.some((log) => log.args.amount === amount)),
            { timeout: 10_000 },
          );
          const transfer = renderHook(() => hooks.useTokenTransfer(address), {
            wrapper,
          });
          await act(async () =>
            transfer.result.current.write({ to: zero, amount: 1n }),
          );
          await waitFor(
            () =>
              assert.ok(
                transfer.result.current.error || transfer.result.current.data,
              ),
            { timeout: 10_000 },
          );
          if (transfer.result.current.data) {
            // Wallet submission can succeed even when execution reverts.
            const failed = await client.waitForTransactionReceipt({
              hash: transfer.result.current.data,
              timeout: 20_000,
            });
            assert.equal(failed.status, "reverted");
          } else {
            assert.match(
              String(transfer.result.current.error),
              /revert|InvalidRecipient/i,
            );
          }
        } finally {
          cleanup();
          await disconnect(config);
          queryClient.clear();
          dom.window.close();
        }
      },
    );
  } finally {
    clearInterval(keepAlive);
    rmSync(dir, { recursive: true, force: true });
  }
});
