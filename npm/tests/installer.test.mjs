import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:http";
import { cp, mkdtemp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { archiveName, platformTargets } from "../abi-typegen/scripts/checksums.mjs";
import { downloadFile, installBinary } from "../abi-typegen/scripts/installer.mjs";

const exec = promisify(execFile);
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");

async function fixture(t, handler) {
  const dir = await mkdtemp(join(tmpdir(), "abi installer spaces-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const server = createServer(handler);
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  t.after(() => new Promise((resolve) => server.close(resolve)));
  // Exercise real curl against a local fixture. Production permits HTTPS only.
  const run = (command, args, options) =>
    exec(
      command,
      args.map((arg) =>
        arg === "=https"
          ? "=http"
          : arg.startsWith("https://github.com/")
            ? `http://127.0.0.1:${server.address().port}/archive`
            : arg,
      ),
      options,
    );
  return { dir, run };
}

test("504 downloads retry and recover using real curl", async (t) => {
  let requests = 0;
  const { dir, run } = await fixture(t, (_req, res) => {
    res.writeHead(++requests === 1 ? 504 : 200);
    res.end(requests === 1 ? "gateway timeout" : "complete archive");
  });
  const destination = join(dir, "archive");
  await downloadFile("https://github.com/archive", destination, { run });
  assert.equal(requests, 2);
  assert.equal(await readFile(destination, "utf8"), "complete archive");
});

test("persistent 504 stops after four attempts and removes partial output", async (t) => {
  let requests = 0;
  const { dir, run } = await fixture(t, (_req, res) => {
    requests++;
    res.writeHead(504);
    res.end("gateway timeout");
  });
  const destination = join(dir, "archive");
  await assert.rejects(downloadFile("https://github.com/archive", destination, { run }), /504/);
  assert.equal(requests, 4);
  await assert.rejects(readFile(destination), { code: "ENOENT" });
});

test("404 is not retried", async (t) => {
  let requests = 0;
  const { dir, run } = await fixture(t, (_req, res) => {
    requests++;
    res.writeHead(404);
    res.end();
  });
  await assert.rejects(
    downloadFile("https://github.com/archive", join(dir, "archive"), { run }),
    /404/,
  );
  assert.equal(requests, 1);
});

test("production curl refuses plain HTTP downloads", async (t) => {
  const dir = await mkdtemp(join(tmpdir(), "abi-https-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  await assert.rejects(
    downloadFile("http://127.0.0.1:1/archive", join(dir, "archive")),
    /[Pp]rotocol.*(?:disabled|not supported)/,
  );
  assert.deepEqual(await readdir(dir), []);
});

test("production redirect policy rejects an HTTP downgrade", async (t) => {
  let requests = 0;
  const { dir, run } = await fixture(t, (_req, res) => {
    requests++;
    res.writeHead(302, { Location: "http://127.0.0.1:1/insecure" });
    res.end();
  });
  await assert.rejects(
    downloadFile("https://github.com/archive", join(dir, "archive"), {
      run: (command, args, options) =>
        run(
          command,
          args.map((arg, index) =>
            // The fixture rewrites '=https' for its local initial request. Preserve
            // the production redirect policy using curl's equivalent '+https' form.
            args[index - 1] === "--proto-redir" ? "-all,+https" : arg,
          ),
          options,
        ),
    }),
    /[Pp]rotocol.*(?:disabled|not supported)/,
  );
  assert.equal(requests, 1);
  assert.deepEqual(await readdir(dir), []);
});

async function installation(t, content = "verified binary") {
  const dir = await mkdtemp(join(tmpdir(), "abi install-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const source = join(dir, "source");
  await mkdir(source);
  await writeFile(join(source, "abi-typegen"), content);
  const archive = join(dir, "fixture.tar.gz");
  await exec("tar", ["-czf", archive, "-C", source, "abi-typegen"]);
  const bytes = await readFile(archive);
  const binDir = join(dir, "bin");
  const options = {
    url: "https://github.com/archive",
    binDir,
    binaryName: "abi-typegen",
    expectedHash: hash(bytes),
    download: (_url, destination) => writeFile(destination, bytes),
  };
  return { binDir, options };
}

test("verified archive installs and leaves only the binary", async (t) => {
  const { binDir, options } = await installation(t);
  await installBinary(options);
  assert.equal(await readFile(join(binDir, "abi-typegen"), "utf8"), "verified binary");
  assert.deepEqual(await readdir(binDir), ["abi-typegen"]);
});

test("hash mismatch never extracts and preserves an existing binary", async (t) => {
  const { binDir, options } = await installation(t);
  await mkdir(binDir);
  await writeFile(join(binDir, "abi-typegen"), "existing binary");
  let extracted = false;
  await assert.rejects(
    installBinary({
      ...options,
      expectedHash: "0".repeat(64),
      extract: async () => {
        extracted = true;
      },
    }),
    /SHA-256 mismatch/,
  );
  assert.equal(extracted, false);
  assert.equal(await readFile(join(binDir, "abi-typegen"), "utf8"), "existing binary");
  assert.deepEqual(await readdir(binDir), ["abi-typegen"]);
});

test("missing or malformed hashes fail before download", async (t) => {
  const { options } = await installation(t);
  for (const expectedHash of [undefined, "", "invalid", "0".repeat(63)]) {
    await assert.rejects(
      installBinary({
        ...options,
        expectedHash,
        download: () => assert.fail("must not download without a valid pinned hash"),
      }),
      /SHA-256/,
    );
  }
});

test("truncated downloads do not reach extraction or leave files", async (t) => {
  const { binDir, options } = await installation(t);
  await assert.rejects(
    installBinary({
      ...options,
      download: async (_url, destination) => {
        await writeFile(destination, "partial");
        throw new Error("connection reset");
      },
      extract: () => assert.fail("must not extract failed download"),
    }),
    /connection reset/,
  );
  assert.deepEqual(await readdir(binDir), []);
});

test("corrupt archive with matching hash fails extraction and cleans up", async (t) => {
  const { binDir, options } = await installation(t);
  await assert.rejects(
    installBinary({
      ...options,
      expectedHash: hash("not an archive"),
      download: (_url, destination) => writeFile(destination, "not an archive"),
    }),
  );
  assert.deepEqual(await readdir(binDir), []);
});

test("an existing binary does not bypass archive verification", async (t) => {
  const { binDir, options } = await installation(t);
  await mkdir(binDir);
  await writeFile(join(binDir, "abi-typegen"), "unverified existing binary");
  await installBinary(options);
  assert.equal(await readFile(join(binDir, "abi-typegen"), "utf8"), "verified binary");
});

test("empty binaries are rejected even when the archive hash matches", async (t) => {
  const { binDir, options } = await installation(t, "");
  await assert.rejects(installBinary(options), /nonempty regular binary/);
  assert.deepEqual(await readdir(binDir), []);
});

test(
  "Windows ZIP archives are verified and extracted with bsdtar",
  {
    skip: process.platform !== "win32" && process.platform !== "darwin",
  },
  async (t) => {
    const { binDir, options } = await installation(t);
    const bytes = Buffer.from(
      "UEsDBBQAAAAAAKpELl32ZXyJFwAAABcAAAAPAAAAYWJpLXR5cGVnZW4uZXhldmVyaWZpZWQgd2luZG93cyBiaW5hcnlQSwECFAMUAAAAAACqRC5d9mV8iRcAAAAXAAAADwAAAAAAAAAAAAAAgAEAAAAAYWJpLXR5cGVnZW4uZXhlUEsFBgAAAAABAAEAPQAAAEQAAAAAAA==",
      "base64",
    );
    await installBinary({
      ...options,
      binaryName: "abi-typegen.exe",
      expectedHash: hash(bytes),
      download: (_url, destination) => writeFile(destination, bytes),
    });
    assert.equal(
      await readFile(join(binDir, "abi-typegen.exe"), "utf8"),
      "verified windows binary",
    );
    assert.deepEqual(await readdir(binDir), ["abi-typegen.exe"]);
  },
);

test(
  "postinstall reads pinned package hashes and rejects replacement archives",
  {
    skip: process.platform === "win32",
  },
  async (t) => {
    const { binDir } = await installation(t);
    const packageDir = join(binDir, "..");
    const scripts = fileURLToPath(new URL("../abi-typegen/scripts/", import.meta.url));
    await cp(scripts, join(packageDir, "scripts"), { recursive: true });
    const archive = join(packageDir, "fixture.tar.gz");
    const bytes = await readFile(archive);
    const version = "1.2.3";
    await writeFile(join(packageDir, "package.json"), JSON.stringify({ version }));
    await writeFile(
      join(packageDir, "checksums.json"),
      JSON.stringify({
        version,
        files: Object.fromEntries(
          Object.values(platformTargets).map((target) => [archiveName(target), hash(bytes)]),
        ),
      }),
    );
    const tools = join(packageDir, "tools");
    await mkdir(tools);
    // Only the network boundary is replaced. Run the actual postinstall, hashing,
    // extraction, and replacement code in a separate Node process.
    await writeFile(
      join(tools, "curl"),
      `#!${process.execPath}\nconst fs = require('node:fs');\nfs.copyFileSync(process.env.ABI_TEST_ARCHIVE, process.argv[process.argv.indexOf('--output') + 1]);\n`,
      { mode: 0o755 },
    );
    const env = { ...process.env, PATH: `${tools}:${process.env.PATH}`, ABI_TEST_ARCHIVE: archive };
    const entrypoint = join(packageDir, "scripts", "install.mjs");
    const result = await exec(process.execPath, [entrypoint], { env });
    assert.match(result.stdout, /SHA-256 verified/);
    assert.equal(await readFile(join(binDir, "abi-typegen"), "utf8"), "verified binary");
    await writeFile(archive, "replaced archive");
    await assert.rejects(exec(process.execPath, [entrypoint], { env }), /SHA-256 mismatch/);
    assert.equal(await readFile(join(binDir, "abi-typegen"), "utf8"), "verified binary");
    assert.deepEqual(await readdir(binDir), ["abi-typegen"]);
  },
);
