# frozen_string_literal: true

require "tmpdir"
require "open3"

RSpec.describe "abi-typegen Ruby renderer regressions" do
  before(:context) do
    binary = ENV.fetch("ABI_TYPEGEN_BIN", File.expand_path("../../../target/debug/abi-typegen", __dir__))
    @generated_dir = Dir.mktmpdir("abi-typegen-ruby-")
    output, status = Open3.capture2e(binary, "generate", "--artifacts", File.join(__dir__, "artifacts"),
      "--out", @generated_dir, "--target", "ruby")
    raise "Ruby fixture generation failed: #{output}" unless status.success?
    require File.join(@generated_dir, "RubyCoverage")
    require File.join(@generated_dir, "RubyZero")
  end

  after(:context) do
    FileUtils.remove_entry(@generated_dir) if @generated_dir
  end

  let(:address) { "0x0000000000000000000000000000000000000002" }
  let(:binding) { RubyCoverageContract.new(address, Object.new) }
  let(:word) { ->(n) { n.to_s(16).rjust(64, "0") } }
  let(:topic_hash) { "0x" + "ab" * 32 }

  it "encodes constructor-only tuple Structs even when a field shadows Struct#to_a" do
    settings = RubyCoverageSettings.new(to_a: 17, owner: Eth::Address.new(address))
    transaction = { value: 2, gas_limit: 100_000 }
    deployment = RubyCoverageContract.build_deployment("0X6000", settings, transaction)
    expected = "0x6000" + word.call(17) + address.delete_prefix("0x").rjust(64, "0")
    expect(deployment).to eq(data: expected, value: 2, gas_limit: 100_000)
    expect(transaction).to eq(value: 2, gas_limit: 100_000)
    expect(binding.encode_store_settings(settings)).to eq(RubyCoverageContract::STORE_SETTINGS_SELECTOR + expected.delete_prefix("0x6000"))
  end

  it "checks payable and nonpayable transaction values without changing caller options" do
    options = { "value" => 5, gas_limit: 40_000 }
    expect(binding.deposit(7, options)).to include(value: 5, gas_limit: 40_000, to: Eth::Address.new(address).to_s)
    expect(options).to eq("value" => 5, gas_limit: 40_000)
    [false, -1, "0x1"].each do |value|
      expect { binding.deposit(7, value: value) }.to raise_error(ArgumentError)
      expect { RubyCoverageContract.build_deployment("6000", [1, address], value: value) }.to raise_error(ArgumentError)
    end
    expect(binding.write).to include(value: 0, data: RubyCoverageContract::WRITE_SELECTOR)
    expect { binding.write(value: 1) }.to raise_error(ArgumentError)
    expect(RubyZeroContract.build_deployment("6000")).to eq(data: "0x6000", value: 0)
    expect { RubyZeroContract.build_deployment("6000", value: 1) }.to raise_error(ArgumentError)
  end

  it "decodes multiple return values and rejects data for void functions" do
    expect(binding.decode_echo_result("0x" + word.call(7) + word.call(1))).to eq([7, true])
    [nil, "", "0x"].each { |data| expect(binding.decode_noop_result(data)).to be_nil }
    expect { binding.decode_noop_result("0x" + word.call(0)) }.to raise_error(ArgumentError)
  end

  it "decodes anonymous events without signature topics" do
    owner_topic = "0x" + address.delete_prefix("0x").rjust(64, "0")
    log = { topics: [owner_topic], data: "0x" + word.call(7) }
    expect(binding.decode_anonymous_log(log)).to eq("owner" => Eth::Address.new(address).to_s, "amount" => 7)
    expect { binding.decode_anonymous_log(topics: [], data: log[:data]) }.to raise_error(ArgumentError)
    expect { binding.decode_anonymous_log(topics: [owner_topic], data: log[:data] + word.call(0)) }.to raise_error(ArgumentError)
  end

  it "builds indexed reference filters with exact hashes and decodes returned logs" do
    filters = []
    client = Object.new
    client.define_singleton_method(:eth_get_logs) do |filter|
      filters << filter
      { "result" => [{ "topics" => filter[:topics], "data" => "0x" }] }
    end
    contract = RubyCoverageContract.new(address, client)
    label_hash = Eth::Util.bin_to_prefixed_hex(Eth::Util.keccak256("hello"))
    bytes_hash = Eth::Util.bin_to_prefixed_hex(Eth::Util.keccak256("\xab".b))
    expect(contract.filter_refs(label: "hello", payload: "0xab")).to eq([{"label" => label_hash, "payload" => bytes_hash}])
    expect(filters.last[:topics]).to eq([RubyCoverageContract::REFS_EVENT_TOPIC, label_hash, bytes_hash])
    expect(contract.filter_collections(items: topic_hash, point: topic_hash)).to eq([{"items" => topic_hash, "point" => topic_hash}])
    expect { contract.filter_collections(items: [1]) }.to raise_error(ArgumentError, /topic hash/)
    expect { contract.filter_refs(label: 1) }.to raise_error(ArgumentError)
    expect { contract.filter_refs(payload: 1) }.to raise_error(ArgumentError)
  end

  it "rejects noncanonical indexed scalar topics" do
    key = "0x12345678" + "00" * 28
    log = { topics: [RubyCoverageContract::SCALAR_EVENT_TOPIC, "0x" + word.call(1), key], data: "0x" }
    expect(binding.decode_scalar_log(log)).to eq("ok" => true, "key" => "\x12\x34\x56\x78".b)
    expect { binding.decode_scalar_log(log.merge(topics: [log[:topics][0], "0x" + word.call(2), key])) }.to raise_error(ArgumentError)
    expect { binding.decode_scalar_log(log.merge(topics: [log[:topics][0], log[:topics][1], key[0...-2] + "01"])) }.to raise_error(ArgumentError)
  end

  it "keeps overloaded error decoders and function helper collisions callable" do
    expect(binding.decode_denied_error(RubyCoverageContract::DENIED_1_ERROR_SELECTOR)).to eq([])
    expect(binding.decode_denied_2_error(RubyCoverageContract::DENIED_2_ERROR_SELECTOR + word.call(7))).to eq([7])
    expect(binding.decode_denied_error(RubyCoverageContract::DENIED_2_ERROR_SELECTOR + word.call(7))).to be_nil
    expect { binding.decode_denied_error(RubyCoverageContract::DENIED_1_ERROR_SELECTOR + word.call(0)) }.to raise_error(ArgumentError)
    expect(binding.encode_foo).to eq(RubyCoverageContract::FOO_SELECTOR)
    expect(binding.encode_encode_foo_2).to eq(RubyCoverageContract::ENCODE_FOO_SELECTOR)
    expect(binding.encode_decode_foo_result_2).to eq(RubyCoverageContract::DECODE_FOO_RESULT_SELECTOR)
    expect(binding.encode_class_).to eq(RubyCoverageContract::CLASS_SELECTOR)
    expect(binding.encode_lookup).to eq(RubyCoverageContract::LOOKUP_1_SELECTOR)
    expect(binding.encode_lookup_uint256(7)).to eq(RubyCoverageContract::LOOKUP_2_SELECTOR + word.call(7))
  end
end
