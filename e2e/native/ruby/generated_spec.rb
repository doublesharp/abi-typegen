# frozen_string_literal: true

require_relative "build/generated-contracts/Token"
require_relative "build/generated-contracts/Vault"
require_relative "build/generated-contracts/NativeCases"
require_relative "build/generated-contracts/EdgeCases"
require_relative "build/generated-contracts/NamingCases"

RSpec.describe "abi-typegen Ruby generated bindings" do
  let(:owner) { "0x0000000000000000000000000000000000000001" }
  let(:contract_address) { "0x0000000000000000000000000000000000000002" }
  let(:token) { TokenContract.new(contract_address, Object.new) }

  it "encodes calldata and decodes results offline" do
    expect(token.encode_balance_of(owner)[0, 10]).to eq(TokenContract::BALANCE_OF_SELECTOR)

    raw = "0x" + 10.to_s(16).rjust(64, "0")
    expect(token.decode_balance_of_result(raw)).to eq(10)

    expect(token.encode_transfer(owner, 10)[0, 10]).to eq(TokenContract::TRANSFER_SELECTOR)
  end

  it "builds write transaction hashes without owning signing" do
    tx = token.transfer(owner, 10, gas_limit: 100_000)
    expect(tx[:to]).to eq(Eth::Address.new(contract_address).to_s)
    expect(tx[:data][0, 10]).to eq(TokenContract::TRANSFER_SELECTOR)
    expect(tx[:value]).to eq(0)
    expect(tx[:gas_limit]).to eq(100_000)
    expect { token.transfer(owner, 10, value: "0x1") }.to raise_error(ArgumentError, /not payable/)
  end

  it "decodes custom errors" do
    vault = VaultContract.new(contract_address, Object.new)
    data = VaultContract::INSUFFICIENT_SHARES_ERROR_SELECTOR +
      10.to_s(16).rjust(64, "0") +
      1.to_s(16).rjust(64, "0")
    values = vault.decode_insufficient_shares_error(data)
    expect(values[0]).to eq(10)
  end

  it "encodes a zero-input call as only its selector" do
    expect(token.encode_name).to eq(TokenContract::NAME_SELECTOR)
  end

  it "returns indexed reference topics as hashes" do
    native = NativeCasesContract.new(contract_address, Object.new)
    hash = "0x" + "ab" * 32
    log = { "topics" => [NativeCasesContract::INDEXED_REFERENCES_EVENT_TOPIC, hash, hash, hash], "data" => "0x" }
    decoded = native.decode_indexed_references_log(log)
    expect(decoded["label"]).to eq(hash)
    expect(decoded["payload"]).to eq(hash)
    expect(decoded["values"]).to eq(hash)
  end

  it "preserves wide integers and decodes nested array results" do
    huge = 1 << 128
    encoded = Eth::Util.bin_to_prefixed_hex(Eth::Abi.encode(["uint256"], [huge]))
    expect(token.decode_balance_of_result(encoded)).to eq(huge)
    edge = EdgeCasesContract.new(contract_address, Object.new)
    nested = Eth::Util.bin_to_prefixed_hex(Eth::Abi.encode(["uint256[][]"], [[[1, 2]]]))
    expect(edge.decode_nested_array_result(nested)).to eq([[1, 2]])
    expect { edge.decode_nested_array_result(nested + "00" * 32) }.to raise_error(ArgumentError)
  end

  it "encodes nested tuples and distinct overload selectors" do
    native = NativeCasesContract.new(contract_address, Object.new)
    grid = NativeCasesGrid.new(rows: [[1, 2]], flags: [true, false], tags: ["0x" + "ab" * 32])
    batch = NativeCasesBatch.new(calls: [], grid: grid, fee: 17, delta: -3)
    expect(native.encode_aggregate(batch)[0, 10]).to eq(NativeCasesContract::AGGREGATE_SELECTOR)
    naming = NamingCasesContract.new(contract_address, Object.new)
    expect(naming.encode_foo_bar(true)[0, 10]).not_to eq(naming.encode_foo_bar_2(1)[0, 10])
  end

  it "rejects malformed payloads for zero-argument custom errors" do
    expect(token.decode_invalid_recipient_error(TokenContract::INVALID_RECIPIENT_ERROR_SELECTOR)).to eq([])
    expect { token.decode_invalid_recipient_error(TokenContract::INVALID_RECIPIENT_ERROR_SELECTOR + "00" * 32) }.to raise_error(ArgumentError)
  end
end
