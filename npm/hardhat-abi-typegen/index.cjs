/**
 * Hardhat 2 plugin entry point for @0xdoublesharp/hardhat-abi-typegen.
 *
 * Usage in hardhat.config.ts:
 *   import "@0xdoublesharp/hardhat-abi-typegen";
 *
 * This hooks into the "compile" task to auto-generate TypeScript types
 * after every `npx hardhat compile`.
 *
 * Optional config in hardhat.config.ts:
 *   typegen: {
 *     out: "types",          // default: "src/generated"
 *     target: "viem",        // default: "viem"; comma-separated values allowed
 *     wrappers: true,        // default: true
 *     contracts: ["MyToken"], // default: [] (all)
 *   }
 */

/// <reference path="./type-extensions.d.ts" />

const { execFileSync } = require("child_process");
const path = require("path");

const { extendConfig, subtask } = require("hardhat/config");
const { TASK_COMPILE_SOLIDITY_COMPILE_JOBS } = require("hardhat/builtin-tasks/task-names");

// Resolve defaults for the typegen config so hre.config.typegen is always populated.
extendConfig((config, userConfig) => {
  const typegen = userConfig.typegen || {};
  config.typegen = {
    out: typegen.out || "src/generated",
    target: typegen.target || "viem",
    wrappers: typegen.wrappers !== false,
    contracts: typegen.contracts || [],
    exclude: typegen.exclude || [],
  };
});

// Resolve the binary from @0xdoublesharp/abi-typegen package
const BINARY = (() => {
  try {
    // Try to find the binary via the companion package
    const binPkg = path.dirname(require.resolve("@0xdoublesharp/abi-typegen/package.json"));
    return path.join(binPkg, "bin", "abi-typegen");
  } catch {
    // Fallback: assume it's on PATH
    return "abi-typegen";
  }
})();

subtask(TASK_COMPILE_SOLIDITY_COMPILE_JOBS).setAction(async (args, hre, runSuper) => {
  const result = await runSuper(args);

  const config = hre.config.typegen;
  const artifactsDir = path.join(hre.config.paths.artifacts, "contracts");

  const cliArgs = [
    "generate",
    "--hardhat",
    "--artifacts",
    artifactsDir,
    "--out",
    config.out,
    "--target",
    config.target,
  ];

  if (!config.wrappers) {
    cliArgs.push("--no-wrappers");
  }

  for (const name of config.contracts) {
    cliArgs.push("--contracts", name);
  }

  if (config.exclude.length > 0) {
    cliArgs.push("--exclude", config.exclude.join(","));
  }

  try {
    execFileSync(BINARY, cliArgs, { stdio: "inherit" });
  } catch (err) {
    console.error("abi-typegen: generation failed");
    throw err;
  }

  return result;
});
