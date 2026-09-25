"""Executable offline and disposable-Anvil tests for generated web3.py bindings."""
import os

from eth_abi import encode as abi_encode
from eth_utils import keccak
from web3 import Web3
from web3.exceptions import ContractLogicError

from Generated.Token import TokenContract
from Generated.EdgeCases import EdgeCasesContract
from Generated.NamingCases import NamingCasesContract


def check(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def offline() -> None:
    w3 = Web3()
    address = Web3.to_checksum_address("0x0000000000000000000000000000000000000001")
    spender = Web3.to_checksum_address("0x0000000000000000000000000000000000000002")
    token = TokenContract(address, w3)
    calldata = token.encode_approve(spender, 42)
    check(calldata.startswith("0x095ea7b3") and len(calldata) == 138, "typed calldata")
    check(token.decode_balance_of_result("0x" + abi_encode(["uint256"], [42]).hex()) == 42, "typed output")
    for data in ("0x" + abi_encode(["uint256"], [42]).hex() + "00" * 32, "0x00"):
        try:
            if len(data) > 10:
                token.decode_balance_of_result(data)
            else:
                token.decode_mint_result(data)
        except ValueError:
            pass
        else:
            raise AssertionError("malformed output accepted")
    check(token.decode_invalid_recipient_error("0x9c8d2cd2") == (), "custom error")
    try:
        token.approve(spender, 42, {"value": 1})
    except ValueError as error:
        check("not payable" in str(error), "nonpayable guard")
    else:
        raise AssertionError("nonpayable value accepted")
    edge = EdgeCasesContract(address, w3)
    check(edge.encode_process_complex((42, spender, bytes(32), True, "tuple", 7)).startswith("0x"), "tuple encoding")
    nested = abi_encode(["uint256[][]"], [[[1, 2]]])
    check(edge.decode_nested_array_result("0x" + nested.hex()) == ((1, 2),), "nested array decoding")
    error = keccak(text="InvalidInput(string,bytes)")[:4] + abi_encode(["string", "bytes"], ["bad", b"\x01"])
    check(edge.decode_invalid_input_error("0x" + error.hex()) == ("bad", b"\x01"), "typed custom error")
    naming = NamingCasesContract(address, w3)
    check(naming.encode_foo_bar(True)[:10] != naming.encode_foo_bar_2(1)[:10], "distinct canonical overloads")


def rpc() -> None:
    url = os.getenv("ATG_RPC_URL")
    if not url:
        return
    w3 = Web3(Web3.HTTPProvider(url))
    chain_id = int(os.environ["ATG_CHAIN_ID"])
    check(w3.eth.chain_id == chain_id, "chain ID")
    account = w3.eth.account.from_key(os.environ["ATG_PRIVATE_KEY"])
    address = Web3.to_checksum_address(os.environ["ATG_TOKEN_ADDRESS"])
    token = TokenContract(address, w3)
    amount = 1 << 128

    def submit(tx: dict) -> dict:
        signed = account.sign_transaction(tx)
        hash_ = w3.eth.send_raw_transaction(signed.raw_transaction)
        receipt = w3.eth.wait_for_transaction_receipt(hash_, timeout=20)
        check(receipt.status == 1, "transaction reverted")
        return receipt

    base = {"from": account.address, "chainId": chain_id, "gas": 500_000, "gasPrice": w3.eth.gas_price}
    mint_tx = token.mint(account.address, amount, dict(base, nonce=w3.eth.get_transaction_count(account.address)))
    mint_receipt = submit(mint_tx)
    check(token.balance_of(account.address) == amount, "typed read after mint")
    check(len(mint_receipt.logs) == 1, "missing mint event")
    transfer = token.decode_transfer_log(mint_receipt.logs[0])
    check(transfer["args"]["to"] == account.address and transfer["args"]["amount"] == amount, "typed transfer decode")
    filtered = token.filter_transfer(from_block=mint_receipt.blockNumber, to_block=mint_receipt.blockNumber)
    check(len(filtered) == 1, "historical filter")

    approve_tx = token.approve(account.address, 17, dict(base, nonce=w3.eth.get_transaction_count(account.address)))
    approve_receipt = submit(approve_tx)
    check(token.allowance(account.address, account.address) == 17, "typed allowance")
    check(token.decode_approval_log(approve_receipt.logs[0])["args"]["amount"] == 17, "approval event")

    zero = Web3.to_checksum_address("0x0000000000000000000000000000000000000000")
    try:
        w3.eth.call({"to": address, "from": account.address, "data": token.encode_transfer(zero, 1)})
    except (ContractLogicError, ValueError):
        pass
    else:
        raise AssertionError("expected transfer revert")


if __name__ == "__main__":
    offline()
    rpc()
    print("Python generated web3.py consumer passed")
