import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import {
  archiveName,
  platformTargets,
  validateChecksums,
} from "../abi-typegen/scripts/checksums.mjs";
import { generateChecksums, prepareChecksums } from "../abi-typegen/scripts/release-checksums.mjs";

const version = "1.2.3";
const names = Object.values(platformTargets).map(archiveName);
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const manifest = () => ({
  version,
  files: Object.fromEntries(names.map((name) => [name, digest(name)])),
});

async function temp(t) {
  const dir = await mkdtemp(join(tmpdir(), "abi-checksums-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  return dir;
}

test("manifest rejects wrong version, missing archives, invalid digests and unexpected files", () => {
  assert.deepEqual(validateChecksums(manifest(), version), manifest());
  assert.throws(() => validateChecksums(manifest(), "1.2.4"));
  for (const change of [
    (m) => {
      delete m.files[names[0]];
    },
    (m) => {
      m.files[names[0]] = "not a hash";
    },
    (m) => {
      m.files["../unexpected"] = "0".repeat(64);
    },
  ]) {
    const value = manifest();
    change(value);
    assert.throws(() => validateChecksums(value, version));
  }
});

test("release generation hashes every archive and refuses missing assets", async (t) => {
  const dir = await temp(t);
  await assert.rejects(generateChecksums(version, dir));
  for (const name of names) await writeFile(join(dir, name), name);
  assert.deepEqual(await generateChecksums(version, dir), manifest());
});

test("npm preparation verifies all release assets before embedding hashes", async (t) => {
  const packageDir = await temp(t);
  const urls = [];
  await prepareChecksums({
    version,
    packageDir,
    download: async (url, path) => {
      urls.push(url);
      const name = url.split("/").at(-1);
      await writeFile(path, name === "checksums.json" ? JSON.stringify(manifest()) : name);
    },
  });
  assert.equal(urls.length, 6);
  assert.ok(
    urls.every((url) =>
      url.startsWith(`https://github.com/doublesharp/abi-typegen/releases/download/v${version}/`),
    ),
  );
  assert.deepEqual(
    JSON.parse(await readFile(join(packageDir, "checksums.json"), "utf8")),
    manifest(),
  );
  assert.deepEqual(await readdir(packageDir), ["checksums.json"]);
});

test("npm preparation refuses substituted assets and removes stale pinned hashes", async (t) => {
  const packageDir = await temp(t);
  await writeFile(join(packageDir, "checksums.json"), JSON.stringify(manifest()));
  await assert.rejects(
    prepareChecksums({
      version,
      packageDir,
      download: (_url, path) =>
        writeFile(
          path,
          path.endsWith("checksums.json") ? JSON.stringify(manifest()) : "substituted",
        ),
    }),
    /SHA-256 mismatch/,
  );
  assert.deepEqual(await readdir(packageDir), []);
});
