#include "Misc/AutomationTest.h"

#include "AbiTypegenTypes.h"
#include "AbiTypegenRpc.h"
#include "Generated/TokenUnreal.h"
#include "Generated/atg_Token.h"

#if WITH_DEV_AUTOMATION_TESTS

IMPLEMENT_SIMPLE_AUTOMATION_TEST(
    FAbiTypegenRpcEnvelopeTest,
    "AbiTypegen.Unreal.Rpc.StrictEnvelope",
    EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FAbiTypegenRpcEnvelopeTest::RunTest(const FString&)
{
    const int32 ExpectedId = 7;
    const FAbiTypegenRpcResponse Good = FAbiTypegenRpcClient::ParseResponse(
        TEXT("{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":\"0x1\"}"), 200, ExpectedId);
    TestTrue(TEXT("matching response succeeds"), Good.bOk);
    TestFalse(TEXT("wrong id rejected"), FAbiTypegenRpcClient::ParseResponse(
        TEXT("{\"jsonrpc\":\"2.0\",\"id\":8,\"result\":\"0x1\"}"), 200, ExpectedId).bOk);
    TestFalse(TEXT("missing id rejected"), FAbiTypegenRpcClient::ParseResponse(
        TEXT("{\"jsonrpc\":\"2.0\",\"result\":\"0x1\"}"), 200, ExpectedId).bOk);
    TestFalse(TEXT("wrong version rejected"), FAbiTypegenRpcClient::ParseResponse(
        TEXT("{\"jsonrpc\":\"1.0\",\"id\":7,\"result\":\"0x1\"}"), 200, ExpectedId).bOk);
    TestFalse(TEXT("HTTP 503 result rejected"), FAbiTypegenRpcClient::ParseResponse(
        TEXT("{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":\"0x1\"}"), 503, ExpectedId).bOk);
    TestFalse(TEXT("result and error rejected"), FAbiTypegenRpcClient::ParseResponse(
        TEXT("{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":\"0x1\",\"error\":{\"code\":-1,\"message\":\"bad\"}}"), 200, ExpectedId).bOk);
    TestFalse(TEXT("malformed JSON rejected"), FAbiTypegenRpcClient::ParseResponse(TEXT("{"), 200, ExpectedId).bOk);
    const FAbiTypegenRpcResponse Error = FAbiTypegenRpcClient::ParseResponse(
        TEXT("{\"jsonrpc\":\"2.0\",\"id\":7,\"error\":{\"code\":-32000,\"message\":\"reverted\",\"data\":\"0xdead\"}}"), 200, ExpectedId);
    TestFalse(TEXT("RPC error is not success"), Error.bOk);
    TestEqual(TEXT("RPC error code"), Error.RpcErrorCode, static_cast<int64>(-32000));
    TestEqual(TEXT("RPC revert data"), Error.RevertData, TEXT("0xdead"));
    return true;
}

namespace
{
    atg_word WordFromHexChecked(const TCHAR* Hex)
    {
        FAbiTypegenWord256 Reflected;
        Reflected.Hex = Hex;
        atg_word Word{};
        FString Error;
        check(Reflected.TryToWord(Word, Error));
        return Word;
    }

    atg_value* WordValue(const atg_word& Word)
    {
        return atg_value_word(Word.bytes);
    }

    bool PushOwned(atg_value* Sequence, atg_value* Child)
    {
        if (!Sequence || !Child) { atg_value_free(Child); return false; }
        const bool bOk = atg_value_push(Sequence, Child) == 0;
        atg_value_free(Child);
        return bOk;
    }
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(
    FAbiTypegenMetadataTest,
    "AbiTypegen.Unreal.Codec.MetadataAndGeneratedNames",
    EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FAbiTypegenMetadataTest::RunTest(const FString&)
{
    TestEqual(TEXT("runtime ABI version"), ATG_ABI_VERSION, 1);
    TestEqual(TEXT("canonical signature"), FString(UTF8_TO_TCHAR(atg_Token_atg_balanceOf_signature)), TEXT("balanceOf(address)"));
    TestEqual(TEXT("balanceOf selector byte 0"), atg_Token_atg_balanceOf_selector[0], static_cast<uint8>(0x70));
    TestEqual(TEXT("balanceOf selector byte 3"), atg_Token_atg_balanceOf_selector[3], static_cast<uint8>(0x31));
    TestTrue(TEXT("embedded ABI includes balanceOf"), UTokenUnrealMetadata::GetAbi().Contains(TEXT("balanceOf")));
    TestNotNull(TEXT("deterministic reflected async class exists"), UTokenBalanceOfAsyncAction::StaticClass());
    const UFunction* Factory = UTokenBalanceOfAsyncAction::StaticClass()->FindFunctionByName(TEXT("BalanceOf"));
    TestNotNull(TEXT("BalanceOf reflected factory exists"), Factory);
    if (Factory) TestTrue(TEXT("BalanceOf is BlueprintCallable"), Factory->HasAnyFunctionFlags(FUNC_BlueprintCallable));
    TestTrue(TEXT("write operations are explicitly reported as non-Blueprint"), UTokenUnrealMetadata::GetBlueprintUnsupportedOperations().Num() >= 3);
    return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(
    FAbiTypegenUint256ExactTest,
    "AbiTypegen.Unreal.Codec.Uint256Above128Bits",
    EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FAbiTypegenUint256ExactTest::RunTest(const FString&)
{
    const uint8 Byte = 1;
    TestTrue(TEXT("empty byte span fits"), FAbiTypegenResult::CanCopyBytes(0, nullptr));
    TestFalse(TEXT("nonempty byte span needs storage"), FAbiTypegenResult::CanCopyBytes(1, nullptr));
    TestFalse(TEXT("oversized byte span is rejected before Unreal array conversion"),
        FAbiTypegenResult::CanCopyBytes(static_cast<size_t>(MAX_int32) + 1, &Byte));
    FAbiTypegenWord256 EmptyWord;
    FString InvalidWord;
    EmptyWord.Hex = TEXT("0x");
    atg_word Rejected{};
    TestFalse(TEXT("empty uint256 literal is invalid"), EmptyWord.TryToWord(Rejected, InvalidWord));

    // 2^200 + 0x3039, well above 2^128 and exactly representable as an ABI word.
    const atg_word Expected = WordFromHexChecked(TEXT("0x0000000000000100000000000000000000000000000000000000000000003039"));
    static const char Abi[] = "[{\"type\":\"function\",\"name\":\"echo\",\"inputs\":[{\"name\":\"value\",\"type\":\"uint256\"}],\"outputs\":[{\"name\":\"value\",\"type\":\"uint256\"}],\"stateMutability\":\"pure\"}]";

    atg_value* Args = atg_value_seq();
    TestTrue(TEXT("argument sequence allocated"), Args != nullptr);
    if (!Args) return false;
    TestTrue(TEXT("large integer pushed"), PushOwned(Args, WordValue(Expected)));

    FAbiTypegenResult Encoded(atg_encode(Abi, "echo(uint256)", Args));
    atg_value_free(Args);
    TestTrue(TEXT("runtime encoded large uint256"), Encoded.IsOk());
    if (!Encoded.IsOk()) { AddError(Encoded.Error()); return false; }

    const TArray<uint8> CallData = Encoded.CopyBytes();
    TestTrue(TEXT("calldata contains selector and one word"), CallData.Num() == 36);
    if (CallData.Num() != 36) return false;

    FAbiTypegenResult Decoded(atg_decode(Abi, "echo(uint256)", CallData.GetData() + 4, 32));
    TestTrue(TEXT("runtime decoded large uint256"), Decoded.IsOk());
    if (!Decoded.IsOk()) { AddError(Decoded.Error()); return false; }

    const atg_value* Outputs = Decoded.Value();
    TestTrue(TEXT("one decoded output"), Outputs && atg_value_kind(Outputs) == 4 && atg_value_len(Outputs) == 1);
    if (!Outputs || atg_value_len(Outputs) != 1) return false;
    const atg_value* Value = atg_value_at(Outputs, 0);
    TestTrue(TEXT("decoded value is a word"), Value && atg_value_kind(Value) == 2 && atg_value_len(Value) == 32);
    if (!Value) return false;
    TestTrue(TEXT("all 256 bits preserved"), FMemory::Memcmp(atg_value_data(Value), Expected.bytes, 32) == 0);
    return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(
    FAbiTypegenOverloadTupleArrayTest,
    "AbiTypegen.Unreal.Codec.OverloadsTuplesArrays",
    EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FAbiTypegenOverloadTupleArrayTest::RunTest(const FString&)
{
    static const char Abi[] = "["
        "{\"type\":\"function\",\"name\":\"foo\",\"inputs\":[{\"name\":\"value\",\"type\":\"uint256\"}],\"outputs\":[],\"stateMutability\":\"pure\"},"
        "{\"type\":\"function\",\"name\":\"foo\",\"inputs\":[{\"name\":\"value\",\"type\":\"address\"}],\"outputs\":[],\"stateMutability\":\"pure\"},"
        "{\"type\":\"function\",\"name\":\"complex\",\"inputs\":[{\"name\":\"pair\",\"type\":\"tuple\",\"components\":[{\"name\":\"n\",\"type\":\"uint256\"},{\"name\":\"flag\",\"type\":\"bool\"}]},{\"name\":\"values\",\"type\":\"uint256[]\"}],\"outputs\":[{\"name\":\"pair\",\"type\":\"tuple\",\"components\":[{\"name\":\"n\",\"type\":\"uint256\"},{\"name\":\"flag\",\"type\":\"bool\"}]},{\"name\":\"values\",\"type\":\"uint256[]\"}],\"stateMutability\":\"pure\"}"
        "]";

    const atg_word One = WordFromHexChecked(TEXT("0x01"));
    atg_value* UintArgs = atg_value_seq();
    PushOwned(UintArgs, WordValue(One));
    FAbiTypegenResult UintCall(atg_encode(Abi, "foo(uint256)", UintArgs));
    atg_value_free(UintArgs);

    atg_word AddressWord{};
    AddressWord.bytes[31] = 1;
    atg_value* AddressArgs = atg_value_seq();
    PushOwned(AddressArgs, WordValue(AddressWord));
    FAbiTypegenResult AddressCall(atg_encode(Abi, "foo(address)", AddressArgs));
    atg_value_free(AddressArgs);

    TestTrue(TEXT("uint overload encodes"), UintCall.IsOk());
    TestTrue(TEXT("address overload encodes"), AddressCall.IsOk());
    const TArray<uint8> A = UintCall.CopyBytes();
    const TArray<uint8> B = AddressCall.CopyBytes();
    TestTrue(TEXT("overload selectors are distinct"), A.Num() >= 4 && B.Num() >= 4 && FMemory::Memcmp(A.GetData(), B.GetData(), 4) != 0);

    atg_value* Pair = atg_value_seq();
    PushOwned(Pair, WordValue(One));
    PushOwned(Pair, atg_value_bool(1));
    atg_value* Values = atg_value_seq();
    PushOwned(Values, WordValue(One));
    const atg_word Ten = WordFromHexChecked(TEXT("0x0a"));
    PushOwned(Values, WordValue(Ten));
    atg_value* ComplexArgs = atg_value_seq();
    PushOwned(ComplexArgs, Pair);
    PushOwned(ComplexArgs, Values);

    FAbiTypegenResult ComplexCall(atg_encode(Abi, "complex((uint256,bool),uint256[])", ComplexArgs));
    atg_value_free(ComplexArgs);
    TestTrue(TEXT("tuple + array encode"), ComplexCall.IsOk());
    if (!ComplexCall.IsOk()) { AddError(ComplexCall.Error()); return false; }
    const TArray<uint8> C = ComplexCall.CopyBytes();
    FAbiTypegenResult ComplexDecoded(atg_decode(Abi, "complex((uint256,bool),uint256[])", C.GetData() + 4, static_cast<size_t>(C.Num() - 4)));
    TestTrue(TEXT("tuple + array decode"), ComplexDecoded.IsOk());
    if (!ComplexDecoded.IsOk()) { AddError(ComplexDecoded.Error()); return false; }
    const atg_value* Output = ComplexDecoded.Value();
    TestTrue(TEXT("two decoded outputs"), Output && atg_value_kind(Output) == 4 && atg_value_len(Output) == 2);
    if (!Output || atg_value_len(Output) != 2) return false;
    const atg_value* DecodedPair = atg_value_at(Output, 0);
    const atg_value* DecodedValues = atg_value_at(Output, 1);
    TestTrue(TEXT("tuple has two values"), DecodedPair && atg_value_kind(DecodedPair) == 4 && atg_value_len(DecodedPair) == 2);
    TestTrue(TEXT("array has two values"), DecodedValues && atg_value_kind(DecodedValues) == 4 && atg_value_len(DecodedValues) == 2);
    if (!DecodedPair || !DecodedValues || atg_value_len(DecodedPair) != 2 || atg_value_len(DecodedValues) != 2) return false;
    const atg_value* DecodedN = atg_value_at(DecodedPair, 0);
    const atg_value* DecodedFlag = atg_value_at(DecodedPair, 1);
    const atg_value* DecodedFirst = atg_value_at(DecodedValues, 0);
    const atg_value* DecodedSecond = atg_value_at(DecodedValues, 1);
    TestTrue(TEXT("tuple integer exact"), DecodedN && atg_value_kind(DecodedN) == 2 && FMemory::Memcmp(atg_value_data(DecodedN), One.bytes, 32) == 0);
    TestTrue(TEXT("tuple boolean true"), DecodedFlag && atg_value_kind(DecodedFlag) == 1 && atg_value_get_bool(DecodedFlag) == 1);
    TestTrue(TEXT("array first exact"), DecodedFirst && atg_value_kind(DecodedFirst) == 2 && FMemory::Memcmp(atg_value_data(DecodedFirst), One.bytes, 32) == 0);
    TestTrue(TEXT("array second exact"), DecodedSecond && atg_value_kind(DecodedSecond) == 2 && FMemory::Memcmp(atg_value_data(DecodedSecond), Ten.bytes, 32) == 0);
    return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(
    FAbiTypegenEventErrorTest,
    "AbiTypegen.Unreal.Codec.EventsAndCustomErrors",
    EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FAbiTypegenEventErrorTest::RunTest(const FString&)
{
    const atg_word Ten = WordFromHexChecked(TEXT("0x0a"));
    uint8 Topics[96] = {0};
    FMemory::Memcpy(Topics, atg_Token_atg_Transfer_event_topic, 32);
    Topics[63] = 1; // indexed from address ...01
    Topics[95] = 2; // indexed to address ...02
    atg_Token_atg_Transfer_event_fields Event{};
    FAbiTypegenResult EventDecoded(atg_Token_atg_Transfer_event_decode(Topics, 3, Ten.bytes, 32, &Event));
    TestTrue(TEXT("Transfer event decodes"), EventDecoded.IsOk());
    TestEqual(TEXT("Transfer amount exact"), FAbiTypegenWord256::FromWord(Event.atg_amount).Hex, FAbiTypegenWord256::FromWord(Ten).Hex);

    // Known Solidity selector for InsufficientBalance(uint256,uint256): 0xcf479181.
    static const char ErrorAbi[] = "[{\"type\":\"error\",\"name\":\"InsufficientBalance\",\"inputs\":[{\"name\":\"balance\",\"type\":\"uint256\"},{\"name\":\"needed\",\"type\":\"uint256\"}]}]";
    TArray<uint8> Payload;
    Payload.Add(0xcf); Payload.Add(0x47); Payload.Add(0x91); Payload.Add(0x81);
    Payload.Append(Ten.bytes, 32);
    const atg_word One = WordFromHexChecked(TEXT("0x01"));
    Payload.Append(One.bytes, 32);
    FAbiTypegenResult ErrorDecoded(atg_decode_error(ErrorAbi, "InsufficientBalance(uint256,uint256)", Payload.GetData(), Payload.Num()));
    TestTrue(TEXT("custom error decodes"), ErrorDecoded.IsOk());
    if (!ErrorDecoded.IsOk()) AddError(ErrorDecoded.Error());
    return ErrorDecoded.IsOk();
}

#endif
