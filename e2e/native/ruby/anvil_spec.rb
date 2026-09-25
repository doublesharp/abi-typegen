# frozen_string_literal: true

require_relative "build/generated-contracts/Token"

RSpec.describe "abi-typegen Ruby Anvil integration" do
  it "signs through eth.rb, reads back state, and decodes events" do
    url = ENV["ATG_RPC_URL"]
    skip "ATG_RPC_URL is only set by the Anvil harness" unless url

    client = Eth::Client.create(url)
    key = Eth::Key.new(priv: ENV.fetch("ATG_PRIVATE_KEY"))
    token = TokenContract.new(ENV.fetch("ATG_TOKEN_ADDRESS"), client)
    owner = key.address.to_s

    send_generated = lambda do |transaction|
      tx = Eth::Tx::Legacy.new(
        transaction.merge(
          from: owner,
          nonce: client.get_nonce(key.address),
          gas_price: client.eth_gas_price.fetch("result").to_i(16)
        ),
        client.chain_id
      )
      tx.sign(key)
      hash = client.eth_send_raw_transaction(tx.hex).fetch("result")
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 20
      loop do
        receipt = client.eth_get_transaction_receipt(hash).fetch("result")
        break [hash, receipt] unless receipt.nil?
        raise "Timed out waiting for Ruby transaction #{hash}" if Process.clock_gettime(Process::CLOCK_MONOTONIC) > deadline
        sleep 0.1
      end
    end

    amount = 1 << 128
    tx_hash, receipt = send_generated.call(token.mint(owner, amount, gas_limit: 300_000))
    expect(receipt.fetch("status")).to eq("0x1")

    expect(token.balance_of(owner)).to eq(amount)

    transfer = receipt.fetch("logs").filter_map { |log| token.decode_transfer_log(log) }.first
    expect(transfer).not_to be_nil
    expect(transfer["amount"] || transfer[:amount]).to eq(amount)

    logs = token.filter_transfer(from_block: 0, to_block: "latest", to: owner)
    expect(logs).not_to be_empty

    spender = "0x0000000000000000000000000000000000000003"
    _, approve_receipt = send_generated.call(token.approve(spender, amount, gas_limit: 300_000))
    expect(approve_receipt.fetch("status")).to eq("0x1")
    expect(token.allowance(owner, spender)).to eq(amount)
  end
end
