#include "Misc/AutomationTest.h"
#include "Generated/TupleCasesUnreal.h"
#include "Generated/TokenUnreal.h"
#include "Generated/NativeCasesUnreal.h"

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FAbiTypegenReflectedCodec, "AbiTypegen.Unreal.Codec.ReflectedGenericAbi", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)
bool FAbiTypegenReflectedCodec::RunTest(const FString& Parameters)
{
    FAbiTypegenWord256 Large; Large.Hex = TEXT("0x0000000000000100000000000000000000000000000000000000000000001234");
    FTupleCasesSetMatrix_Input_Matrix_Item Row; Row.Values = {Large, FAbiTypegenWord256::FromUInt64(7)};
    FAbiTypegenBytes Encoded; FString Error;
    TestTrue(TEXT("nested matrix encodes"), UTupleCasesUnrealMetadata::EncodeSetMatrix(Encoded, Error, {Row}));
    atg_TupleCases_atg_setMatrix_params Params{}; atg_word Words[2]{};
    TestTrue(TEXT("word conversion"), Large.TryToWord(Words[0], Error)); FAbiTypegenWord256::FromUInt64(7).TryToWord(Words[1], Error);
    atg_TupleCases_type_6 NativeRows[1]{}; NativeRows[0].data = Words; NativeRows[0].len = 2; Params.atg_matrix.data = NativeRows; Params.atg_matrix.len = 1;
    FAbiTypegenResult Native(atg_TupleCases_atg_setMatrix_encode(&Params));
    TestTrue(TEXT("matches shared native codec"), Encoded.Data == Native.CopyBytes());
    FTupleCasesSetRows_Input_Rows_Item Fixed; Fixed.Values = {true};
    TestFalse(TEXT("wrong fixed array length rejected"), UTupleCasesUnrealMetadata::EncodeSetRows(Encoded, Error, {Fixed}));
    TestTrue(TEXT("failed encode clears previous bytes"), Encoded.Data.IsEmpty());
    Fixed.Values.Add(false); TestTrue(TEXT("valid fixed array encodes"), UTupleCasesUnrealMetadata::EncodeSetRows(Encoded, Error, {Fixed}));

    FAbiTypegenBytes ReturnData; atg_word Amount{}; Large.TryToWord(Amount, Error); ReturnData.Data.Append(Amount.bytes, 32);
    FTokenBalanceOfResult Balance; TestTrue(TEXT("typed read result"), UTokenUnrealMetadata::DecodeBalanceOf(ReturnData, Balance, Error)); TestEqual(TEXT("result word stays lossless"), Balance.Value0.Hex, Large.Hex);

    FAbiTypegenBytes EventData = ReturnData;
    TArray<FAbiTypegenWord256> Topics;
    FAbiTypegenWord256 Topic0; Topic0.Hex = AbiTypegen::BytesToHex(atg_Token_atg_Transfer_event_topic, 32); Topics.Add(Topic0);
    Topics.Add(FAbiTypegenWord256::FromUInt64(1)); Topics.Add(FAbiTypegenWord256::FromUInt64(2));
    FTokenTransferEvent Event;
    TestTrue(TEXT("typed event decode"), UTokenUnrealMetadata::DecodeTransferEvent(EventData, Event, Error, Topics));
    TestEqual(TEXT("typed event lossless amount"), Event.Amount.Hex, Large.Hex);
    TestEqual(TEXT("typed event indexed address"), Event.From.Hex, TEXT("0x0000000000000000000000000000000000000001"));
    return true;
}
