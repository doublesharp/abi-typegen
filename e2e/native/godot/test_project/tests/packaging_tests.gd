extends RefCounted

const MetadataOnly = preload("res://generated/metadata_only_token.gd")
const TestUtil = preload("res://tests/test_util.gd")

func run() -> Array[String]:
    var t = TestUtil.new()
    t.check(ClassDB.class_exists("AbiTypegenCodec"), "AbiTypegenCodec is registered")
    t.check(ClassDB.class_exists("AbiTypegenClient"), "AbiTypegenClient is registered")
    t.check(ClassDB.class_exists("AbiTypegenRequest"), "AbiTypegenRequest is registered")
    t.equal(AbiTypegenCodec.runtime_abi_version(), 1, "GDExtension is built against runtime ABI v1")
    t.equal(MetadataOnly.BALANCE_OF_SIGNATURE, "balanceOf(address)", "metadata-only signature")
    t.equal(MetadataOnly.BALANCE_OF_SELECTOR, "0x70a08231", "metadata-only selector")

    var source := FileAccess.get_file_as_string("res://generated/metadata_only_token.gd")
    t.check(not source.contains("AbiTypegenCodec"), "metadata-only file has no codec dependency")
    t.check(not source.contains("AbiTypegenClient"), "metadata-only file has no RPC dependency")
    t.check(not source.contains("static func encode_"), "metadata-only file has no callable wrappers")

    t.check(FileAccess.file_exists("res://bin/abi_typegen.gdextension"), "GDExtension manifest is packaged")
    return t.failures
