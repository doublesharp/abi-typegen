#!/usr/bin/env python3
"""Extract receipt/revert bytes; ABI encoding and decoding stay in OCaml."""
import json
import os
import subprocess
import sys
import urllib.error
import urllib.request

url = os.environ["ATG_RPC_URL"]
if sys.argv[1] == "receipt":
    receipt = json.loads(subprocess.check_output(
        ["cast", "receipt", "--rpc-url", url, sys.argv[2], "--json"], text=True, timeout=30))
    assert int(receipt["status"], 16) == 1, "transaction reverted"
    assert receipt["transactionHash"].lower() == sys.argv[2].lower()
    logs = receipt["logs"]
    assert len(logs) == 1, "expected one Token event"
    log = logs[0]
    assert log["address"].lower() == os.environ["ATG_TOKEN_ADDRESS"].lower()
    print(log["data"])
    print("\n".join(log["topics"]))
elif sys.argv[1] == "revert":
    call = {"to": os.environ["ATG_TOKEN_ADDRESS"], "from": sys.argv[2], "data": sys.argv[3]}
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "eth_call", "params": [call, "latest"]}).encode()
    request = urllib.request.Request(url, body, {"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=10) as response:
        result = json.load(response)
    assert "error" in result, "expected eth_call revert"
    data = result["error"].get("data")
    assert isinstance(data, str) and data.startswith("0x"), result
    print(data)
else:
    raise ValueError("unknown operation")
