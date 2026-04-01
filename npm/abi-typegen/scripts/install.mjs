#!/usr/bin/env node

/**
 * Downloads the correct pre-built abi-typegen binary for the current platform
 * from GitHub Releases and places it in the package's bin/ directory.
 */

import { readFileSync } from "fs";
import { chmodSync, mkdirSync, existsSync } from "fs";
import { execSync } from "child_process";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

const pkg = JSON.parse(readFileSync(join(__dirname, "..", "package.json"), "utf8"));
const VERSION = pkg.version;
const REPO = "doublesharp/abi-typegen";

const PLATFORM_MAP = {
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "win32-x64": "x86_64-pc-windows-msvc",
};

const key = `${process.platform}-${process.arch}`;
const target = PLATFORM_MAP[key];

if (!target) {
  console.error(`abi-typegen: unsupported platform ${key}`);
  console.error("Build from source: cargo install abi-typegen");
  process.exit(1);
}

const isWindows = process.platform === "win32";
const ext = isWindows ? "zip" : "tar.gz";
const binaryName = isWindows ? "abi-typegen.exe" : "abi-typegen";
const url = `https://github.com/${REPO}/releases/download/v${VERSION}/abi-typegen-${target}.${ext}`;
const binDir = join(__dirname, "..", "bin");
const binPath = join(binDir, binaryName);

// Skip if already downloaded
if (existsSync(binPath)) {
  process.exit(0);
}

mkdirSync(binDir, { recursive: true });

console.log(`abi-typegen: downloading ${target} binary...`);

try {
  if (isWindows) {
    const zipPath = join(binDir, "abi-typegen.zip");
    execSync(`curl -fsSL "${url}" -o "${zipPath}"`, { stdio: "inherit" });
    execSync(`tar -xf "${zipPath}" -C "${binDir}"`, { stdio: "inherit" });
    execSync(`del "${zipPath}"`, { stdio: "inherit" });
  } else {
    execSync(`curl -fsSL "${url}" | tar -xz -C "${binDir}"`, { stdio: "inherit" });
    chmodSync(binPath, 0o755);
  }
  console.log("abi-typegen: installed successfully");
} catch {
  console.error(`abi-typegen: failed to download binary from ${url}`);
  console.error("Build from source: cargo install abi-typegen");
  process.exit(1);
}
