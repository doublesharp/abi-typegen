import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";

/** Supported platforms and their release targets. */
export const platformTargets = Object.freeze({
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "win32-x64": "x86_64-pc-windows-msvc",
});

/** Return the published archive name for a release target. */
export function archiveName(target) {
  return `abi-typegen-${target}.${target.endsWith("windows-msvc") ? "zip" : "tar.gz"}`;
}

/** Compute a file's SHA-256 without buffering the archive in memory. */
export async function sha256File(path) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest("hex");
}

/** Reject wrong versions, incomplete target sets, and invalid SHA-256 digests. */
export function validateChecksums(manifest, version) {
  if (
    !manifest ||
    manifest.version !== version ||
    !manifest.files ||
    typeof manifest.files !== "object" ||
    Array.isArray(manifest.files)
  ) {
    throw new Error(`Invalid checksum manifest for version ${version}`);
  }
  const names = Object.values(platformTargets).map(archiveName);
  if (Object.keys(manifest.files).length !== names.length) {
    throw new Error("Checksum manifest must contain exactly the supported release archives");
  }
  for (const name of names) {
    if (typeof manifest.files[name] !== "string" || !/^[a-f0-9]{64}$/.test(manifest.files[name])) {
      throw new Error(`Missing or invalid SHA-256 checksum for ${name}`);
    }
  }
  return manifest;
}
