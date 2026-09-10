#!/usr/bin/env node
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const name = process.platform === "win32" ? "abi-typegen.exe" : "abi-typegen";
const binary = fileURLToPath(new URL(`../bin/${name}`, import.meta.url));
const child = spawn(binary, process.argv.slice(2), { stdio: "inherit" });
const signals = ["SIGINT", "SIGTERM", "SIGHUP"];
const handlers = new Map(signals.map((signal) => [signal, () => child.kill(signal)]));
for (const [signal, handler] of handlers) {
  process.on(signal, handler);
}

child.on("error", (error) => {
  console.error(`Unable to run abi-typegen: ${error.message}`);
  process.exit(1);
});
child.on("exit", (status, signal) => {
  for (const [name, handler] of handlers) {
    process.removeListener(name, handler);
  }
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(status ?? 1);
  }
});
