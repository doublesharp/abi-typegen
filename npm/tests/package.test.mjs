import assert from "node:assert/strict";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { once } from "node:events";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { archiveName, platformTargets } from "../abi-typegen/scripts/checksums.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url));
const npm = process.platform === "win32" ? "npm.cmd" : "npm";

test("packed package links its command before the native binary is installed", () => {
  const dir = mkdtempSync(join(tmpdir(), "abi-typegen-package-"));
  try {
    const staging = join(dir, "package");
    mkdirSync(staging);
    const source = join(root, "npm", "abi-typegen");
    for (const name of ["package.json", "README.md", "scripts"]) {
      cpSync(join(source, name), join(staging, name), { recursive: true });
    }
    const version = JSON.parse(readFileSync(join(staging, "package.json"), "utf8")).version;
    const manifest = {
      version,
      files: Object.fromEntries(
        Object.values(platformTargets).map((target) => [archiveName(target), "0".repeat(64)]),
      ),
    };
    // Packing without pinned hashes must fail, even though installation runs later.
    const missingHashes = spawnSync(npm, ["pack", staging, "--pack-destination", dir], {
      encoding: "utf8",
    });
    assert.notEqual(missingHashes.status, 0);
    assert.match(missingHashes.stderr, /checksums.json/);
    writeFileSync(join(staging, "checksums.json"), JSON.stringify(manifest));
    mkdirSync(join(staging, "bin"));
    writeFileSync(join(staging, "bin", "unverified-local-binary"), "must not be packaged");
    const packed = JSON.parse(
      execFileSync(npm, ["pack", staging, "--json", "--pack-destination", dir], {
        encoding: "utf8",
      }),
    );
    assert.ok(packed[0].files.some((file) => file.path === "checksums.json"));
    assert.ok(!packed[0].files.some((file) => file.path.startsWith("bin/")));
    const consumer = join(dir, "consumer");
    execFileSync(npm, [
      "install",
      "--prefix",
      consumer,
      "--ignore-scripts",
      "--no-audit",
      "--no-fund",
      join(dir, packed[0].filename),
    ]);
    const command = join(
      consumer,
      "node_modules",
      ".bin",
      process.platform === "win32" ? "abi-typegen.cmd" : "abi-typegen",
    );
    assert.ok(
      existsSync(command),
      "npm must link the command even before postinstall downloads a binary",
    );
    const installed = join(consumer, "node_modules", "@0xdoublesharp", "abi-typegen");
    assert.deepEqual(JSON.parse(readFileSync(join(installed, "checksums.json"), "utf8")), manifest);
    const binary = process.platform === "win32" ? "abi-typegen.exe" : "abi-typegen";
    mkdirSync(join(installed, "bin"));
    cpSync(join(root, "target", "debug", binary), join(installed, "bin", binary));
    const executable = process.platform === "win32" ? process.execPath : command;
    const prefix = process.platform === "win32" ? [join(installed, "scripts", "run.mjs")] : [];
    assert.equal(
      execFileSync(executable, [...prefix, "--version"], { encoding: "utf8" }).trim(),
      `abi-typegen ${version}`,
    );
    const invalid = spawnSync(executable, [...prefix, "--not-an-option"], { encoding: "utf8" });
    assert.equal(invalid.status, 2, "the launcher must preserve the native CLI exit code");
    assert.match(invalid.stderr, /unexpected argument/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test(
  "terminating the launcher also terminates its native child",
  { skip: process.platform === "win32", timeout: 10000 },
  async () => {
    const dir = mkdtempSync(join(tmpdir(), "abi-typegen-signals-"));
    let launcher;
    let childPid;
    try {
      mkdirSync(join(dir, "scripts"));
      mkdirSync(join(dir, "bin"));
      cpSync(
        join(root, "npm", "abi-typegen", "scripts", "run.mjs"),
        join(dir, "scripts", "run.mjs"),
      );
      writeFileSync(
        join(dir, "bin", "abi-typegen"),
        "#!/usr/bin/env node\nconsole.log(process.pid);\nsetInterval(() => {}, 1000);\n",
        { mode: 0o755 },
      );
      launcher = spawn(process.execPath, [join(dir, "scripts", "run.mjs")]);
      const exited = once(launcher, "exit");
      const [data] = await once(launcher.stdout, "data");
      childPid = Number(data.toString().trim());
      assert.ok(Number.isInteger(childPid) && childPid > 0);
      launcher.kill("SIGTERM");
      const [status, signal] = await exited;
      assert.equal(status, null);
      assert.equal(signal, "SIGTERM");
      assert.throws(
        () => process.kill(childPid, 0),
        { code: "ESRCH" },
        "the native child must not survive the launcher",
      );
    } finally {
      launcher?.kill("SIGKILL");
      if (childPid) {
        try {
          process.kill(childPid, "SIGKILL");
        } catch (error) {
          assert.equal(error.code, "ESRCH");
        }
      }
      rmSync(dir, { recursive: true, force: true });
    }
  },
);
