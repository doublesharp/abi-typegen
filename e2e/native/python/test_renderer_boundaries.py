"""Offline checks for generated Python names, documentation, and ABI literals."""
import json
from pathlib import Path

from eth_utils import keccak
from web3 import Web3

from Generated.PythonBranches import (
    PYTHONBRANCHES_ABI,
    PythonBranchesContract,
    PythonBranchesItem,
    PythonBranchesSettings,
    PythonBranchesDollar_Item,
    PythonBranchesDollar_Item2,
)

fixture = json.loads((Path(__file__).parent / "artifacts/PythonBranches.sol/PythonBranches.json").read_text())
assert PYTHONBRANCHES_ABI == fixture["abi"], "ABI text changed during source generation"
assert PythonBranchesContract.__doc__ == fixture["metadata"]["output"]["userdoc"]["notice"] + "."
assert list(PythonBranchesItem.__annotations__) == ["_class", "_class2", "foo_bar", "foo_bar2"]
assert list(PythonBranchesSettings.__annotations__) == ["owner"]
assert list(PythonBranchesDollar_Item.__annotations__) == ["n"]
assert list(PythonBranchesDollar_Item2.__annotations__) == ["ok"]

address = Web3.to_checksum_address("0x0000000000000000000000000000000000000001")
contract = PythonBranchesContract(address, Web3())
for method, signature in [("_from", "from()"), ("_class", "class()"), ("contract_2", "contract()"), ("build_deployment_2", "buildDeployment()")]:
    assert callable(getattr(contract, method)), f"hidden method {method}"
    encoded = getattr(contract, "encode_" + method)()
    assert encoded == "0x" + keccak(text=signature)[:4].hex(), f"wrong selector {signature}"
assert contract.encode_write((7, True, "text", b"\x01")).startswith("0x"), "tuple encoding"
print("Python renderer boundary consumer passed")
