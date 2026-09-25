defmodule AbiTypegenElixir.AnvilTest do
  use ExUnit.Case, async: false

  @moduletag :anvil
  @receipt_attempts 50
  @receipt_delay_ms 100
  @amount 340_282_366_920_938_463_463_374_607_431_768_211_456

  test "local signer mints 2^128, approves, reads, decodes events, and propagates reverts" do
    url = System.fetch_env!("ATG_RPC_URL")
    contract = System.fetch_env!("ATG_TOKEN_ADDRESS")
    private_key = System.fetch_env!("ATG_PRIVATE_KEY")
    chain_id = System.fetch_env!("ATG_CHAIN_ID") |> String.to_integer()
    rpc_opts = [url: url]

    {:ok, [owner]} = Ethers.Signer.Local.accounts(private_key: private_key)

    send_opts = [
      to: contract,
      from: owner,
      chain_id: chain_id,
      gas: 300_000,
      signer: Ethers.Signer.Local,
      signer_opts: [private_key: private_key],
      rpc_opts: rpc_opts
    ]

    {:ok, mint_hash} = Token.mint(owner, @amount) |> Ethers.send_transaction(send_opts)
    assert %{"status" => "0x1"} = await_receipt!(mint_hash, rpc_opts)

    assert {:ok, @amount} =
             Token.balance_of(owner) |> Ethers.call(to: contract, rpc_opts: rpc_opts)

    {:ok, approve_hash} = Token.approve(owner, @amount) |> Ethers.send_transaction(send_opts)
    assert %{"status" => "0x1"} = await_receipt!(approve_hash, rpc_opts)

    assert {:ok, @amount} =
             Token.allowance(owner, owner) |> Ethers.call(to: contract, rpc_opts: rpc_opts)

    assert {:ok, [%Ethers.Event{} = transfer]} =
             Ethers.get_logs(Token.EventFilters.transfer(nil, owner),
               address: contract,
               from_block: 0,
               to_block: "latest",
               rpc_opts: rpc_opts
             )

    assert transfer.data == [@amount]

    zero = "0x0000000000000000000000000000000000000000"

    assert {:error, %Token.Errors.InvalidRecipient{}} =
             Token.transfer(zero, 1) |> Ethers.call(to: contract, from: owner, rpc_opts: rpc_opts)
  end

  defp await_receipt!(hash, rpc_opts, attempts \\ @receipt_attempts)

  defp await_receipt!(hash, _rpc_opts, 0) do
    flunk("transaction receipt not available: #{hash}")
  end

  defp await_receipt!(hash, rpc_opts, attempts) do
    case Ethers.get_transaction_receipt(hash, rpc_opts: rpc_opts) do
      {:error, :transaction_receipt_not_found} ->
        Process.sleep(@receipt_delay_ms)
        await_receipt!(hash, rpc_opts, attempts - 1)

      {:ok, receipt} when is_map(receipt) ->
        receipt

      {:error, reason} ->
        flunk("receipt lookup failed for #{hash}: #{inspect(reason)}")
    end
  end
end
