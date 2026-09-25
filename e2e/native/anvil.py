#!/usr/bin/env python3
"""Run one generated consumer suite against its own disposable Anvil chain."""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
# Public default Anvil development key. Never fund or use on a public chain.
DEV_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
CHAIN_ID = 313371337


def rpc(url, method):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": []}).encode()
    req = urllib.request.Request(url, body, {"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=1) as response:
        result = json.load(response)
    if "error" in result:
        raise RuntimeError(result["error"])
    return result["result"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cwd", default=".")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.command:
        parser.error("supply a consumer test command")
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    url = f"http://127.0.0.1:{port}"
    node = subprocess.Popen(["anvil", "--host", "127.0.0.1", "--port", str(port), "--chain-id", str(CHAIN_ID), "--silent"], stdout=subprocess.DEVNULL)
    try:
        for _ in range(100):
            if node.poll() is not None:
                raise RuntimeError("test Anvil process exited before readiness")
            try:
                if int(rpc(url, "eth_chainId"), 16) == CHAIN_ID:
                    break
            except (OSError, ValueError):
                pass
            time.sleep(0.05)
        else:
            raise RuntimeError("test Anvil did not become ready")
        artifact = json.loads((ROOT / "e2e/foundry-sample/out/Token.sol/Token.json").read_text())
        bytecode = artifact["bytecode"]["object"]
        constructor = subprocess.check_output(["cast", "abi-encode", "constructor(string,string,uint8)", "Native test", "NAT", "18"], text=True, timeout=30).strip()
        initcode = bytecode + constructor.removeprefix("0x")
        receipt = json.loads(subprocess.check_output(["cast", "send", "--rpc-url", url, "--private-key", DEV_KEY, "--json", "--create", initcode], text=True, timeout=30))
        address = receipt.get("contractAddress")
        if not address or int(receipt["status"], 16) != 1:
            raise RuntimeError("fixture deployment failed")
        env = dict(os.environ, ATG_RPC_URL=url, ATG_TOKEN_ADDRESS=address, ATG_PRIVATE_KEY=DEV_KEY, ATG_CHAIN_ID=str(CHAIN_ID))
        completed = subprocess.run(args.command, cwd=ROOT / args.cwd, env=env, timeout=300)
        return completed.returncode
    finally:
        node.terminate()
        try:
            node.wait(timeout=5)
        except subprocess.TimeoutExpired:
            node.kill()
            node.wait()


if __name__ == "__main__":
    raise SystemExit(main())
