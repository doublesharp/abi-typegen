import { execFile } from "node:child_process";
import { chmod, lstat, mkdir, mkdtemp, rename, rm } from "node:fs/promises";
import { join } from "node:path";
import { promisify } from "node:util";
import { sha256File } from "./checksums.mjs";

const exec = promisify(execFile);

/** Download over HTTPS with four attempts and bounded exponential backoff. */
export async function downloadFile(url, destination, { run = exec } = {}) {
  try {
    await run(
      "curl",
      [
        "--disable",
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--max-redirs",
        "5",
        "--retry",
        "3",
        "--retry-connrefused",
        "--retry-max-time",
        "120",
        "--connect-timeout",
        "15",
        "--max-time",
        "60",
        "--output",
        destination,
        url,
      ],
      { timeout: 180_000, killSignal: "SIGKILL", windowsHide: true },
    );
  } catch (error) {
    await rm(destination, { force: true });
    throw new Error(`Download failed for ${url}: ${error.stderr?.trim() || error.message}`, {
      cause: error,
    });
  }
}

/** Extract only the expected binary from a verified archive. */
async function extractBinary(archive, directory, binaryName) {
  await exec("tar", ["-xf", archive, "-C", directory, "--", binaryName], {
    timeout: 60_000,
    killSignal: "SIGKILL",
    windowsHide: true,
  });
}

/** Verify a pinned archive hash before extraction and atomically replace the binary. */
export async function installBinary({
  url,
  binDir,
  binaryName,
  expectedHash,
  download = downloadFile,
  extract = extractBinary,
}) {
  if (typeof expectedHash !== "string" || !/^[a-f0-9]{64}$/.test(expectedHash)) {
    throw new Error("Missing or invalid pinned SHA-256 checksum");
  }
  await mkdir(binDir, { recursive: true });
  const staging = await mkdtemp(join(binDir, ".install-"));
  try {
    const archive = join(staging, "archive");
    await download(url, archive);
    const actual = await sha256File(archive);
    if (actual !== expectedHash) {
      throw new Error(`SHA-256 mismatch for ${url}: expected ${expectedHash}, received ${actual}`);
    }
    const extracted = join(staging, "extracted");
    await mkdir(extracted);
    await extract(archive, extracted, binaryName);
    const binary = join(extracted, binaryName);
    const stat = await lstat(binary);
    if (!stat.isFile() || stat.nlink !== 1 || stat.size === 0) {
      throw new Error("Archive must contain a nonempty regular binary file");
    }
    await chmod(binary, 0o755);
    await rename(binary, join(binDir, binaryName));
  } finally {
    await rm(staging, { recursive: true, force: true });
  }
}
