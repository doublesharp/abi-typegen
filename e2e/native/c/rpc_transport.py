#!/usr/bin/env python3
"""Test transport adapter: C/C++ stdout requests become local Anvil RPC calls."""
import json
import os
import subprocess
import sys
import urllib.request

url = os.environ["ATG_RPC_URL"]
process = subprocess.Popen(sys.argv[1:], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
try:
    for line in process.stdout:
        if line.startswith("OK"):
            print(line.strip())
            continue
        operation, address, data = line.strip().split()
        if operation == "W":
            receipt = json.loads(subprocess.check_output(["cast", "send", "--rpc-url", url, "--private-key", os.environ["ATG_PRIVATE_KEY"], address, "--data", data, "--json"], text=True, timeout=30))
            if int(receipt["status"], 16) != 1:
                raise RuntimeError("native wrapper transaction reverted")
            response = receipt["transactionHash"]
        elif operation == "R":
            payload = {"jsonrpc": "2.0", "id": 1, "method": "eth_call", "params": [{"to": address, "data": data}, "latest"]}
            request = urllib.request.Request(url,json.dumps(payload).encode(),{"Content-Type":"application/json"})
            with urllib.request.urlopen(request,timeout=10) as result:
                response = json.load(result)
            if "error" in response:
                raise RuntimeError(response["error"])
            response = response["result"]
        else:
            raise RuntimeError(f"unknown operation {operation}")
        process.stdin.write(response+"\n")
        process.stdin.flush()
    raise SystemExit(process.wait(timeout=10))
finally:
    if process.poll() is None:
        process.kill()
        process.wait()
