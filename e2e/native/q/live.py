#!/usr/bin/env python3
"""Mine Token logs on the disposable Anvil chain and decode them in actual q."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.request
from run_q import run_q

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(sys.argv[1]).resolve()
URL = os.environ["ATG_RPC_URL"]
ADDRESS = os.environ["ATG_TOKEN_ADDRESS"]


def rpc(method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(URL, body, {"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=10) as response:
        result = json.load(response)
    if "error" in result:
        raise RuntimeError(result["error"])
    return result["result"]


def send(sender, signature, *args):
    calldata = subprocess.check_output(
        ["cast", "calldata", signature, *map(str, args)], text=True, timeout=30
    ).strip()
    tx = rpc("eth_sendTransaction", [{"from": sender, "to": ADDRESS, "data": calldata, "gas": "0x7a120"}])
    for _ in range(100):
        receipt = rpc("eth_getTransactionReceipt", [tx])
        if receipt is not None:
            if int(receipt["status"], 16) != 1:
                raise RuntimeError("Anvil test transaction reverted")
            return receipt
        time.sleep(0.05)
    raise RuntimeError("Anvil receipt timeout")


def word(number):
    return "0x" + number.to_bytes(32, "big").hex()


def metadata(log):
    return "(" + ";".join([
        word(chain_id), log["address"], word(int(log["blockNumber"], 16)),
        log["blockHash"], log["transactionHash"], word(int(log["logIndex"], 16)),
        "1b" if log["removed"] else "0b",
    ]) + ")"


def append_log(log, table_name, expected=None):
    topics = "0x" + "".join(topic.removeprefix("0x") for topic in log["topics"])
    lines.extend([
        f"logmeta:.atgToken.metadataColumns!{metadata(log)};",
        f"row:.atgToken.{event}Row[logmeta;{topics};{log['data']}];",
        "assert[(value logmeta)~row .atgToken.metadataColumns];",
    ])
    if expected:
        sender, recipient, amount = expected
        lines.extend([
            f"assert[{sender}~row`f0from];",
            f"assert[{recipient}~row`f1to];",
            f"assert[{word(amount)}~row`f2amount];",
        ])
    lines.append(f"{table_name}:{table_name} upsert row;")


chain_id = int(rpc("eth_chainId", []), 16)
if chain_id != int(os.environ["ATG_CHAIN_ID"]):
    raise RuntimeError("unexpected test chain")
sender, recipient = rpc("eth_accounts", [])[:2]
minted = (1 << 200) + 17
transferred = (1 << 128) + 9
mint_receipt = send(sender, "mint(address,uint256)", sender, minted)
transfer_receipt = send(sender, "transfer(address,uint256)", recipient, transferred)
receipts = [mint_receipt, transfer_receipt]
if any(len(receipt["logs"]) != 1 for receipt in receipts):
    raise RuntimeError("expected one Transfer log per receipt")
abi = json.loads((ROOT / "e2e/foundry-sample/out/Token.sol/Token.json").read_text())["abi"]
events = [item for item in abi if item["type"] == "event"]
index = next(i for i, item in enumerate(events) if item["name"] == "Transfer")
event = f"e{index}Transfer"
logs = rpc("eth_getLogs", [{
    "address": ADDRESS,
    "fromBlock": mint_receipt["blockNumber"],
    "toBlock": transfer_receipt["blockNumber"],
    "topics": [mint_receipt["logs"][0]["topics"][0]],
}])
if logs != [receipt["logs"][0] for receipt in receipts]:
    raise RuntimeError("historical query differs from mined receipt logs")
lines = [
    (OUT / "Token.q").read_text(),
    ".atgToken.loadBridge[`:./abi_typegen_q];",
    'assert:{if[not x;\'"Anvil log assertion failed"]};',
    f"receiptTable:.atgToken.{event}Table;",
    f"queryTable:.atgToken.{event}Table;",
]
append_log(mint_receipt["logs"][0], "receiptTable", ("0x" + "00" * 20, sender, minted))
append_log(transfer_receipt["logs"][0], "receiptTable", (sender, recipient, transferred))
for log in logs:
    append_log(log, "queryTable")
lines.extend([
    "assert[2=count receiptTable];",
    "assert[receiptTable~queryTable];",
    '-1 "q Anvil mined receipt and historical log tables passed";',
    "exit 0;",
])
script = OUT / "live.q"
script.write_text("\n".join(lines) + "\n")
run_q(script, "q Anvil mined receipt and historical log tables passed")
