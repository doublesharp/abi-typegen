#!/usr/bin/env node

import { mkdtemp, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { archiveName, platformTargets, sha256File, validateChecksums } from "./checksums.mjs";
import { downloadFile } from "./installer.mjs";

/** Generate a versioned manifest from the complete set of built release archives. */
export async function generateChecksums(version, directory) {
  const files = {};
  for (const target of Object.values(platformTargets)) {
    const name = archiveName(target);
    files[name] = await sha256File(join(directory, name));
  }
  return validateChecksums({ version, files }, version);
}

/** Verify every published archive and pin its digest into the npm package. */
export async function prepareChecksums({ version, packageDir, download = downloadFile }) {
  const destination = join(packageDir, "checksums.json");
  // A failed preparation must not leave an older manifest available for publication.
  await rm(destination, { force: true });
  const staging = await mkdtemp(join(packageDir, ".checksums-"));
  try {
    const base = `https://github.com/doublesharp/abi-typegen/releases/download/v${version}`;
    const manifestPath = join(staging, "checksums.json");
    await download(`${base}/checksums.json`, manifestPath);
    const manifest = validateChecksums(JSON.parse(await readFile(manifestPath, "utf8")), version);
    for (const [name, expected] of Object.entries(manifest.files)) {
      const archive = join(staging, name);
      await download(`${base}/${name}`, archive);
      if ((await sha256File(archive)) !== expected) {
        throw new Error(`SHA-256 mismatch for release asset ${name}`);
      }
    }
    await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
    await rename(manifestPath, destination);
  } finally {
    await rm(staging, { recursive: true, force: true });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const packageDir = dirname(dirname(fileURLToPath(import.meta.url)));
  try {
    const { version } = JSON.parse(await readFile(join(packageDir, "package.json"), "utf8"));
    switch (process.argv[2]) {
      case "generate": {
        if (process.env.GITHUB_REF_NAME !== `v${version}`) {
          throw new Error(`Release tag must match package version v${version}`);
        }
        const manifest = await generateChecksums(version, process.argv[3]);
        await writeFile(process.argv[4], `${JSON.stringify(manifest, null, 2)}\n`);
        break;
      }
      case "prepare":
        await prepareChecksums({ version, packageDir });
        break;
      case "validate":
        validateChecksums(
          JSON.parse(await readFile(join(packageDir, "checksums.json"), "utf8")),
          version,
        );
        break;
      default:
        throw new Error(
          "Usage: release-checksums.mjs generate <archive-dir> <output> | prepare | validate",
        );
    }
  } catch (error) {
    console.error(`abi-typegen: ${error.message}`);
    process.exitCode = 1;
  }
}
