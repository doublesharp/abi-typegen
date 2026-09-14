#!/usr/bin/env node

/** Install the current platform's binary using hashes pinned in this npm package. */
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { archiveName, platformTargets, validateChecksums } from "./checksums.mjs";
import { installBinary } from "./installer.mjs";

const packageDir = dirname(dirname(fileURLToPath(import.meta.url)));

try {
  const key = `${process.platform}-${process.arch}`;
  const target = Object.hasOwn(platformTargets, key) ? platformTargets[key] : undefined;
  if (!target) throw new Error(`Unsupported platform ${key}`);
  const pkg = JSON.parse(await readFile(join(packageDir, "package.json"), "utf8"));
  const manifest = validateChecksums(
    JSON.parse(await readFile(join(packageDir, "checksums.json"), "utf8")),
    pkg.version,
  );
  const archive = archiveName(target);
  const url = `https://github.com/doublesharp/abi-typegen/releases/download/v${pkg.version}/${archive}`;
  console.log(`abi-typegen: downloading and verifying ${target} binary...`);
  await installBinary({
    url,
    expectedHash: manifest.files[archive],
    binDir: join(packageDir, "bin"),
    binaryName: process.platform === "win32" ? "abi-typegen.exe" : "abi-typegen",
  });
  console.log("abi-typegen: installed successfully (SHA-256 verified)");
} catch (error) {
  console.error(`abi-typegen: ${error.message}`);
  console.error("Build from source: cargo install abi-typegen");
  process.exitCode = 1;
}
