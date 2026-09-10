import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const BINARY = (() => {
  try {
    const binPkg = path.dirname(
      fileURLToPath(import.meta.resolve("@0xdoublesharp/abi-typegen/package.json")),
    );
    const packagedBinary = path.join(binPkg, "bin", "abi-typegen");
    if (existsSync(packagedBinary)) {
      return packagedBinary;
    }
  } catch {
    // Continue to local-development and PATH fallbacks below.
  }

  const sourceCheckoutBinary = path.join(__dirname, "..", "..", "target", "debug", "abi-typegen");
  if (existsSync(sourceCheckoutBinary)) {
    return sourceCheckoutBinary;
  }

  return "abi-typegen";
})();

function targetArg(target) {
  return Array.isArray(target) ? target.join(",") : target;
}

function runTypegen(config, artifactsDir, cwd) {
  const cliArgs = [
    "generate",
    "--hardhat",
    "--artifacts",
    artifactsDir,
    "--out",
    config.out,
    "--target",
    targetArg(config.target),
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
    execFileSync(BINARY, cliArgs, { cwd, stdio: "inherit" });
  } catch (err) {
    console.error("abi-typegen: generation failed");
    throw err;
  }
}

export default async () => ({
  processArtifactsAfterSuccessfulBuild: async (context, artifactPaths) => {
    if (artifactPaths.length === 0) {
      return;
    }

    runTypegen(context.config.typegen, context.config.paths.artifacts, context.config.paths.root);
  },
});
