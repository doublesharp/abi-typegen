#include "Misc/AutomationTest.h"
#include "AbiTypegenPolling.h"

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FAbiTypegenPollingQuantities, "AbiTypegen.Unreal.Polling.StrictQuantities", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)
bool FAbiTypegenPollingQuantities::RunTest(const FString& Parameters)
{
    uint64 Number = 0;
    TestTrue(TEXT("quantity"), AbiTypegen::ParseQuantity(TEXT("0xffffffffffffffff"), Number));
    TestEqual(TEXT("lossless"), Number, MAX_uint64);
    TestFalse(TEXT("leading zero"), AbiTypegen::ParseQuantity(TEXT("0x00"), Number));
    TestFalse(TEXT("overflow"), AbiTypegen::ParseQuantity(TEXT("0x10000000000000000"), Number));
    TestFalse(TEXT("empty"), AbiTypegen::ParseQuantity(TEXT("0x"), Number));
    TestFalse(TEXT("negative"), AbiTypegen::ParseQuantity(TEXT("-1"), Number));
    TestFalse(TEXT("trailing junk"), AbiTypegen::ParseQuantity(TEXT("0x1z"), Number));
    return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FAbiTypegenPollingReceipt, "AbiTypegen.Unreal.Polling.StrictReceipt", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)
bool FAbiTypegenPollingReceipt::RunTest(const FString& Parameters)
{
    TSharedRef<FJsonObject> Receipt = MakeShared<FJsonObject>();
    const FString Hash = TEXT("0x") + FString::ChrN(64, 'a');
    Receipt->SetStringField(TEXT("transactionHash"), Hash);
    Receipt->SetStringField(TEXT("blockHash"), Hash);
    Receipt->SetStringField(TEXT("blockNumber"), TEXT("0x4"));
    Receipt->SetStringField(TEXT("status"), TEXT("0x0"));
    FAbiTypegenReceipt Parsed; FString Error;
    TestTrue(TEXT("reverted receipt is a receipt"), AbiTypegen::ParseReceipt(Receipt, Parsed, Error));
    TestFalse(TEXT("reverted status retained"), Parsed.bSucceeded);
    TestEqual(TEXT("block retained"), Parsed.BlockNumber, static_cast<uint64>(4));
    Receipt->SetStringField(TEXT("status"), TEXT("0x2"));
    TestFalse(TEXT("unknown status rejected"), AbiTypegen::ParseReceipt(Receipt, Parsed, Error));
    Receipt->SetStringField(TEXT("status"), TEXT("0x1"));
    Receipt->SetStringField(TEXT("blockHash"), TEXT("0x1234"));
    TestFalse(TEXT("truncated block hash rejected"), AbiTypegen::ParseReceipt(Receipt, Parsed, Error));
    return true;
}

#include "AbiTypegenSigner.h"
IMPLEMENT_SIMPLE_AUTOMATION_TEST(FAbiTypegenTransactionObject, "AbiTypegen.Unreal.Polling.TransactionBoundary", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)
bool FAbiTypegenTransactionObject::RunTest(const FString& Parameters)
{
    FAbiTypegenUnsignedTransaction Tx; Tx.To = TEXT("0x") + FString::ChrN(40, 'a'); Tx.Data = TEXT("0x1234");
    Tx.Value.Hex = TEXT("0x0000000000000100000000000000000000000000000000000000000000001234");
    Tx.GasLimit = 21000;
    TSharedPtr<FJsonObject> Object; FString Error;
    TestTrue(TEXT("valid transaction"), FAbiTypegenRpcClient::BuildTransactionObject(Tx, Tx.To, Object, Error));
    TestEqual(TEXT("preserves full value"), Object->GetStringField(TEXT("value")), TEXT("0x100000000000000000000000000000000000000000000001234"));
    TestEqual(TEXT("gas is a quantity"), Object->GetStringField(TEXT("gas")), TEXT("0x5208"));
    FAbiTypegenWord256 Gas;
    TestTrue(TEXT("gas full width"), FAbiTypegenRpcClient::ParseGasEstimate(TEXT("0x") + FString::ChrN(64, 'f'), Gas, Error));
    TestEqual(TEXT("gas retains all bits"), Gas.Hex, TEXT("0x") + FString::ChrN(64, 'f'));
    TestFalse(TEXT("gas overflow"), FAbiTypegenRpcClient::ParseGasEstimate(TEXT("0x1") + FString::ChrN(64, '0'), Gas, Error));
    TestFalse(TEXT("gas leading zero"), FAbiTypegenRpcClient::ParseGasEstimate(TEXT("0x00"), Gas, Error));
    Tx.Data = TEXT("0x1z");
    TestFalse(TEXT("invalid calldata"), FAbiTypegenRpcClient::BuildTransactionObject(Tx, Tx.To, Object, Error));
    return true;
}

namespace
{
    struct FScriptRpc
    {
        TArray<FString> Methods;
        TArray<FString> Results;
        int32 Next = 0;
        FAutomationTestBase* Test = nullptr;
        FAbiTypegenPollSession::FTransport Transport()
        {
            return [this](const FString& Method, const TArray<TSharedPtr<FJsonValue>>&, FAbiTypegenRpcClient::FCompletion Completion)
            {
                auto Request = MakeShared<FAbiTypegenRpcRequest, ESPMode::ThreadSafe>();
                if (Next >= Methods.Num()) { Test->AddError(TEXT("unexpected polling request: ") + Method); return Request; }
                Test->TestEqual(TEXT("request order"), Method, Methods[Next]);
                auto Response = FAbiTypegenRpcClient::ParseResponse(TEXT("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":") + Results[Next++] + TEXT("}"), 200, 1);
                Completion(MoveTemp(Response)); return Request;
            };
        }
    };
    FString TestHash(TCHAR C) { return TEXT("0x") + FString::ChrN(64, C); }
    FString ReceiptJson(const FString& Hash) { return TEXT("{\"transactionHash\":\"") + Hash + TEXT("\",\"blockHash\":\"") + Hash + TEXT("\",\"blockNumber\":\"0x4\",\"status\":\"0x0\"}"); }
    FString BlockJson(const FString& Hash, const FString& Number) { return TEXT("{\"hash\":\"") + Hash + TEXT("\",\"number\":\"") + Number + TEXT("\"}"); }
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FAbiTypegenPollingBehavior, "AbiTypegen.Unreal.Polling.ConfirmationsCursorCancellation", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)
bool FAbiTypegenPollingBehavior::RunTest(const FString& Parameters)
{
    const FString Hash = TestHash('a');
    FAbiTypegenPollOptions Options; Options.IntervalSeconds = 0.000001; Options.Confirmations = 2;
    FScriptRpc Rpc{{ TEXT("eth_getTransactionReceipt"), TEXT("eth_blockNumber"), TEXT("eth_getBlockByNumber"), TEXT("eth_getTransactionReceipt"), TEXT("eth_getBlockByNumber"), TEXT("eth_blockNumber") }, { ReceiptJson(Hash), TEXT("\"0x5\""), BlockJson(Hash, TEXT("0x4")), ReceiptJson(Hash), BlockJson(Hash, TEXT("0x4")), TEXT("\"0x5\"") }, 0, this};
    bool Completed = false;
    auto Receipt = FAbiTypegenPollSession::WaitReceipt(Rpc.Transport(), Hash, Options, [&Completed, this](const FAbiTypegenReceipt& Value, const FString& Error)
    {
        Completed = true; TestTrue(TEXT("receipt completed without transport error"), Error.IsEmpty()); TestFalse(TEXT("mined revert retained"), Value.bSucceeded);
    });
    FTSTicker::GetCoreTicker().Tick(0.01f);
    TestTrue(TEXT("confirmed canonical receipt delivered"), Completed); TestTrue(TEXT("receipt session ends"), Receipt->IsFinished()); TestEqual(TEXT("receipt revalidated"), Rpc.Next, 6);

    FScriptRpc Rollback{{ TEXT("eth_getTransactionReceipt"), TEXT("eth_blockNumber"), TEXT("eth_getBlockByNumber"), TEXT("eth_getTransactionReceipt"), TEXT("eth_getBlockByNumber"), TEXT("eth_blockNumber") }, { ReceiptJson(Hash), TEXT("\"0x5\""), BlockJson(Hash, TEXT("0x4")), ReceiptJson(Hash), BlockJson(Hash, TEXT("0x4")), TEXT("\"0x4\"") }, 0, this};
    bool RollbackCompleted = false;
    auto RolledBack = FAbiTypegenPollSession::WaitReceipt(Rollback.Transport(), Hash, Options, [&RollbackCompleted](const FAbiTypegenReceipt&, const FString&) { RollbackCompleted = true; });
    FTSTicker::GetCoreTicker().Tick(0.01f);
    TestFalse(TEXT("tip rollback reduces confirmations"), RollbackCompleted); TestFalse(TEXT("insufficient receipt stays pending"), RolledBack->IsFinished()); RolledBack->Cancel();

    FScriptRpc CancelRpc{{}, {}, 0, this};
    auto Canceled = FAbiTypegenPollSession::WaitReceipt(CancelRpc.Transport(), Hash, Options, [this](const FAbiTypegenReceipt&, const FString&) { AddError(TEXT("canceled receipt callback")); });
    Canceled->Cancel(); FTSTicker::GetCoreTicker().Tick(0.01f); TestEqual(TEXT("cancellation prevents RPC"), CancelRpc.Next, 0);

    Options.Confirmations = 1;
    const FString Address = TEXT("0x") + FString::ChrN(40, 'a');
    FScriptRpc LogsRpc{{ TEXT("eth_blockNumber"), TEXT("eth_getBlockByNumber"), TEXT("eth_getLogs"), TEXT("eth_getBlockByNumber") }, { TEXT("\"0x8\""), BlockJson(Hash, TEXT("0x8")), TEXT("[]"), BlockJson(Hash, TEXT("0x8")) }, 0, this};
    bool DecodeFailed = false;
    auto FailedRange = FAbiTypegenPollSession::WatchLogs(LogsRpc.Transport(), Address, {}, 7, Options, [](const TArray<FAbiTypegenLog>&, FString& Error) { Error = TEXT("decoder rejected range"); return false; }, [&DecodeFailed](const FString&) { DecodeFailed = true; });
    FTSTicker::GetCoreTicker().Tick(0.01f);
    TestTrue(TEXT("decode error fails watcher"), DecodeFailed); TestEqual(TEXT("failed range cursor retained"), FailedRange->GetNextBlock(), static_cast<uint64>(7));

    FScriptRpc WatchRpc{{ TEXT("eth_blockNumber"), TEXT("eth_getBlockByNumber"), TEXT("eth_getLogs"), TEXT("eth_getBlockByNumber"), TEXT("eth_getBlockByNumber") }, { TEXT("\"0x8\""), BlockJson(Hash, TEXT("0x8")), TEXT("[]"), BlockJson(Hash, TEXT("0x8")), BlockJson(TestHash('b'), TEXT("0x8")) }, 0, this};
    int32 Delivered = 0; bool Reorg = false;
    auto Watch = FAbiTypegenPollSession::WatchLogs(WatchRpc.Transport(), Address, {}, 7, Options, [&Delivered](const TArray<FAbiTypegenLog>&, FString&) { ++Delivered; return true; }, [&Reorg](const FString&) { Reorg = true; });
    FTSTicker::GetCoreTicker().Tick(0.01f); FTSTicker::GetCoreTicker().Tick(0.01f);
    TestEqual(TEXT("successful range advances"), Watch->GetNextBlock(), static_cast<uint64>(9)); TestEqual(TEXT("range delivered once"), Delivered, 1); TestTrue(TEXT("anchor reorg fails before new delivery"), Reorg);
    return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FAbiTypegenRangeReorg, "AbiTypegen.Unreal.Polling.RangeReorg", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)
bool FAbiTypegenRangeReorg::RunTest(const FString& Parameters)
{
    const FString Hash = TestHash('a'); const FString Address = TEXT("0x") + FString::ChrN(40, 'a');
    FAbiTypegenPollOptions Options; FString Failure; int32 Delivered = 0;
    FScriptRpc Rpc{{ TEXT("eth_blockNumber"), TEXT("eth_getBlockByNumber"), TEXT("eth_getLogs"), TEXT("eth_getBlockByNumber") }, { TEXT("\"0x8\""), BlockJson(Hash, TEXT("0x8")), TEXT("[]"), BlockJson(TestHash('b'), TEXT("0x8")) }, 0, this};
    auto Watch = FAbiTypegenPollSession::WatchLogs(Rpc.Transport(), Address, {}, 7, Options, [&Delivered](const TArray<FAbiTypegenLog>&, FString&) { ++Delivered; return true; }, [&Failure](const FString& Error) { Failure = Error; });
    FTSTicker::GetCoreTicker().Tick(0.01f);
    TestEqual(TEXT("noncanonical range not delivered"), Delivered, 0);
    TestEqual(TEXT("range reorg preserves cursor"), Watch->GetNextBlock(), static_cast<uint64>(7));
    TestTrue(TEXT("range reorg explicitly reported"), Failure.Contains(TEXT("range changed")));
    return true;
}
