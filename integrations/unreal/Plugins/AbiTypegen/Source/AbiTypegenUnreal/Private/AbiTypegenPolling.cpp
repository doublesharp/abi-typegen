#include "AbiTypegenPolling.h"
#include "HAL/PlatformTime.h"

namespace
{
    FString Quantity(uint64 N) { return FString::Printf(TEXT("0x%llx"), static_cast<unsigned long long>(N)); }
    bool HashField(const TSharedPtr<FJsonObject>& O, const TCHAR* Name, FString& Out)
    {
        FString Value; TArray<uint8> Bytes; FString Error;
        if (!O || !O->TryGetStringField(Name, Value) || !Value.StartsWith(TEXT("0x")) || Value.Len() != 66 || !AbiTypegen::HexToBytes(Value, Bytes, Error)) return false;
        Out = Value.ToLower(); return true;
    }
    bool QuantityField(const TSharedPtr<FJsonObject>& O, const TCHAR* Name, uint64& Out)
    {
        FString Text; return O && O->TryGetStringField(Name, Text) && AbiTypegen::ParseQuantity(Text, Out);
    }
    TSharedPtr<FJsonObject> Object(const TSharedPtr<FJsonValue>& V) { return V && V->Type == EJson::Object ? V->AsObject() : nullptr; }
    TArray<TSharedPtr<FJsonValue>> BlockParams(uint64 Number) { return { MakeShared<FJsonValueString>(Quantity(Number)), MakeShared<FJsonValueBoolean>(false) }; }
    bool ValidOptions(const FAbiTypegenPollOptions& Options)
    {
        return Options.Confirmations > 0 && Options.BlocksPerRange > 0 && Options.BlocksPerRange <= 10000 && FMath::IsFinite(Options.IntervalSeconds) && Options.IntervalSeconds > 0 && FMath::IsFinite(Options.TimeoutSeconds) && Options.TimeoutSeconds > 0;
    }
}

bool AbiTypegen::ParseQuantity(const FString& Text, uint64& Out)
{
    if (!Text.StartsWith(TEXT("0x")) || Text.Len() < 3 || Text.Len() > 18 || (Text.Len() > 3 && Text[2] == '0')) return false;
    uint64 Value = 0;
    for (int32 I = 2; I < Text.Len(); ++I)
    {
        const TCHAR C = Text[I];
        int32 Digit = C >= '0' && C <= '9' ? C - '0' : C >= 'a' && C <= 'f' ? C - 'a' + 10 : C >= 'A' && C <= 'F' ? C - 'A' + 10 : -1;
        if (Digit < 0) return false;
        Value = (Value << 4) | static_cast<uint64>(Digit);
    }
    Out = Value; return true;
}

bool AbiTypegen::ParseReceipt(const TSharedPtr<FJsonObject>& Object, FAbiTypegenReceipt& Out, FString& Error)
{
    FAbiTypegenReceipt Parsed; uint64 Status = 0;
    if (!HashField(Object, TEXT("transactionHash"), Parsed.TransactionHash) || !HashField(Object, TEXT("blockHash"), Parsed.BlockHash) || !QuantityField(Object, TEXT("blockNumber"), Parsed.BlockNumber) || !QuantityField(Object, TEXT("status"), Status) || Status > 1)
    { Error = TEXT("invalid transaction receipt"); return false; }
    Parsed.bSucceeded = Status == 1; Out = MoveTemp(Parsed); Error.Reset(); return true;
}

FAbiTypegenPollSession::FTransport FAbiTypegenPollSession::HttpTransport(const FString& RpcUrl, UObject* RequestOwner)
{
    TWeakObjectPtr<UObject> Owner(RequestOwner); const bool bHasOwner = RequestOwner != nullptr;
    return [RpcUrl, Owner, bHasOwner](const FString& Method, const TArray<TSharedPtr<FJsonValue>>& Params, FAbiTypegenRpcClient::FCompletion Completion)
    {
        if (bHasOwner && !Owner.IsValid())
        {
            auto Request = MakeShared<FAbiTypegenRpcRequest, ESPMode::ThreadSafe>();
            FAbiTypegenRpcResponse Response; Response.Error = TEXT("request owner destroyed"); Completion(MoveTemp(Response)); return Request;
        }
        // The session owns cancellation; it must receive a failure after owner destruction.
        return FAbiTypegenRpcClient(RpcUrl).Request(nullptr, Method, Params, [Owner, bHasOwner, Completion = MoveTemp(Completion)](FAbiTypegenRpcResponse&& Response) mutable
        {
            if (bHasOwner && !Owner.IsValid()) { Response.bOk = false; Response.Error = TEXT("request owner destroyed"); }
            Completion(MoveTemp(Response));
        });
    };
}

TSharedRef<FAbiTypegenPollSession> FAbiTypegenPollSession::WaitReceipt(FTransport Transport, const FString& TransactionHash, const FAbiTypegenPollOptions& Options, FReceiptCompletion Completion)
{
    auto Session = MakeShared<FAbiTypegenPollSession>(); Session->Transport = MoveTemp(Transport); Session->Hash = TransactionHash.ToLower(); Session->Options = Options; Session->ReceiptCompletion = MoveTemp(Completion); Session->bReceipt = true; Session->Start(); return Session;
}

TSharedRef<FAbiTypegenPollSession> FAbiTypegenPollSession::WatchLogs(FTransport Transport, const FString& Address, const TArray<FString>& Topics, uint64 StartBlock, const FAbiTypegenPollOptions& Options, FLogsCompletion OnLogs, FFailure OnFailure)
{
    auto Session = MakeShared<FAbiTypegenPollSession>(); Session->Transport = MoveTemp(Transport); Session->Address = Address; Session->Topics = Topics; Session->NextBlock = StartBlock; Session->Options = Options; Session->LogsCompletion = MoveTemp(OnLogs); Session->Failure = MoveTemp(OnFailure); Session->Start(); return Session;
}

FAbiTypegenPollSession::~FAbiTypegenPollSession() { Cancel(); }
void FAbiTypegenPollSession::Cancel()
{
    bFinished = true; if (Request) { Request->Cancel(); Request.Reset(); }
    if (Ticker.IsValid()) { FTSTicker::GetCoreTicker().RemoveTicker(Ticker); Ticker.Reset(); }
}
void FAbiTypegenPollSession::Fail(const FString& Error)
{
    if (bFinished) return; Cancel();
    if (bReceipt) { if (ReceiptCompletion) ReceiptCompletion(Receipt, Error); }
    else if (Failure) Failure(Error);
}
void FAbiTypegenPollSession::Start()
{
    Deadline = FPlatformTime::Seconds() + (ValidOptions(Options) ? Options.TimeoutSeconds : 1.0);
    TWeakPtr<FAbiTypegenPollSession> Weak = AsShared();
    Ticker = FTSTicker::GetCoreTicker().AddTicker(FTickerDelegate::CreateLambda([Weak](float)
    {
        auto Self = Weak.Pin(); if (!Self || Self->bFinished) return false;
        const double Now = FPlatformTime::Seconds();
        if (Now >= Self->Deadline) { Self->Fail(TEXT("polling timed out")); return false; }
        if (!Self->bBusy && Now >= Self->NextPoll) Self->Poll();
        return !Self->bFinished;
    }));
}
void FAbiTypegenPollSession::Send(const FString& Method, const TArray<TSharedPtr<FJsonValue>>& Params, TFunction<void(const TSharedPtr<FJsonValue>&)> Completion)
{
    TWeakPtr<FAbiTypegenPollSession> Weak = AsShared();
    Request = Transport(Method, Params, [Weak, Completion = MoveTemp(Completion)](FAbiTypegenRpcResponse&& Response)
    {
        auto Self = Weak.Pin(); if (!Self || Self->bFinished) return;
        if (!Response.bOk || !Response.Result) { Self->Fail(Response.Error.IsEmpty() ? TEXT("invalid RPC result") : Response.Error); return; }
        Completion(Response.Result);
    });
}
void FAbiTypegenPollSession::Poll()
{
    if (!Transport || !ValidOptions(Options)) { Fail(TEXT("invalid polling options or transport")); return; }
    bBusy = true; NextPoll = FPlatformTime::Seconds() + Options.IntervalSeconds;
    if (bReceipt)
    {
        TArray<uint8> Bytes; FString Error;
        if (Hash.Len() != 66 || !Hash.StartsWith(TEXT("0x")) || !AbiTypegen::HexToBytes(Hash, Bytes, Error)) { Fail(TEXT("invalid transaction hash")); return; }
        FetchReceipt(); return;
    }
    atg_address NativeAddress{}; FString Error;
    if (!AbiTypegen::ParseAddress(Address, NativeAddress, Error)) { Fail(Error); return; }
    if (Topics.Num() > 4) { Fail(TEXT("at most four event topics are supported")); return; }
    for (const FString& Topic : Topics)
    {
        TArray<uint8> Bytes;
        if (Topic.Len() != 66 || !Topic.StartsWith(TEXT("0x")) || !AbiTypegen::HexToBytes(Topic, Bytes, Error)) { Fail(TEXT("invalid event topic")); return; }
    }
    if (!AnchorHash.IsEmpty())
    {
        Send(TEXT("eth_getBlockByNumber"), BlockParams(AnchorBlock), [this](const TSharedPtr<FJsonValue>& V)
        {
            FString Canonical; uint64 Number = 0; if (!HashField(Object(V), TEXT("hash"), Canonical) || !QuantityField(Object(V), TEXT("number"), Number) || Number != AnchorBlock || Canonical != AnchorHash) { Fail(TEXT("event cursor reorg detected; restart from a safe block")); return; }
            FetchHead();
        });
    }
    else FetchHead();
}
void FAbiTypegenPollSession::FetchReceipt()
{
    Send(TEXT("eth_getTransactionReceipt"), { MakeShared<FJsonValueString>(Hash) }, [this](const TSharedPtr<FJsonValue>& V)
    {
        if (V->Type == EJson::Null) { bBusy = false; return; }
        FString Error;
        if (!AbiTypegen::ParseReceipt(Object(V), Receipt, Error) || Receipt.TransactionHash != Hash) { Fail(Error.IsEmpty() ? TEXT("receipt transaction hash mismatch") : Error); return; }
        FetchHead();
    });
}
void FAbiTypegenPollSession::FetchHead()
{
    Send(TEXT("eth_blockNumber"), {}, [this](const TSharedPtr<FJsonValue>& V)
    {
        uint64 Head = 0;
        if (V->Type != EJson::String || !AbiTypegen::ParseQuantity(V->AsString(), Head)) { Fail(TEXT("invalid block number")); return; }
        if (!bReceipt) { FetchLogs(Head); return; }
        if (Head < Receipt.BlockNumber || Head - Receipt.BlockNumber < Options.Confirmations - 1) { bBusy = false; return; }
        Send(TEXT("eth_getBlockByNumber"), BlockParams(Receipt.BlockNumber), [this](const TSharedPtr<FJsonValue>& Block)
        {
            FString Canonical; uint64 Number = 0; if (!HashField(Object(Block), TEXT("hash"), Canonical) || !QuantityField(Object(Block), TEXT("number"), Number) || Number != Receipt.BlockNumber || Canonical != Receipt.BlockHash) { Fail(TEXT("receipt reorg detected")); return; }
            // Re-fetch the receipt after canonical validation to detect movement or disappearance.
            const FAbiTypegenReceipt Expected = Receipt;
            Send(TEXT("eth_getTransactionReceipt"), { MakeShared<FJsonValueString>(Hash) }, [this, Expected](const TSharedPtr<FJsonValue>& Value)
            {
                FAbiTypegenReceipt Current; FString Error;
                if (!AbiTypegen::ParseReceipt(Object(Value), Current, Error) || Current.BlockHash != Expected.BlockHash || Current.TransactionHash != Hash || Current.BlockNumber != Expected.BlockNumber || Current.bSucceeded != Expected.bSucceeded) { Fail(TEXT("receipt changed during confirmation")); return; }
                Send(TEXT("eth_getBlockByNumber"), BlockParams(Current.BlockNumber), [this, Current](const TSharedPtr<FJsonValue>& CanonicalBlock)
                {
                    FString CanonicalHash; uint64 CanonicalNumber = 0;
                    if (!HashField(Object(CanonicalBlock), TEXT("hash"), CanonicalHash) || !QuantityField(Object(CanonicalBlock), TEXT("number"), CanonicalNumber) || CanonicalNumber != Current.BlockNumber || CanonicalHash != Current.BlockHash) { Fail(TEXT("receipt reorg detected during confirmation")); return; }
                    Send(TEXT("eth_blockNumber"), {}, [this, Current](const TSharedPtr<FJsonValue>& HeadValue)
                    {
                        uint64 ConfirmedHead = 0;
                        if (HeadValue->Type != EJson::String || !AbiTypegen::ParseQuantity(HeadValue->AsString(), ConfirmedHead)) { Fail(TEXT("invalid confirmation head")); return; }
                        if (ConfirmedHead < Current.BlockNumber || ConfirmedHead - Current.BlockNumber < Options.Confirmations - 1) { bBusy = false; return; }
                        Cancel(); if (ReceiptCompletion) ReceiptCompletion(Current, FString());
                    });
                });
            });
        });
    });
}
void FAbiTypegenPollSession::FetchLogs(uint64 Head)
{
    if (Head < Options.Confirmations - 1 || Head - (Options.Confirmations - 1) < NextBlock) { bBusy = false; return; }
    const uint64 SafeHead = Head - (Options.Confirmations - 1);
    RangeEnd = NextBlock + FMath::Min(Options.BlocksPerRange - 1, SafeHead - NextBlock);
    Send(TEXT("eth_getBlockByNumber"), BlockParams(RangeEnd), [this](const TSharedPtr<FJsonValue>& V)
    {
        uint64 Number = 0;
        if (!HashField(Object(V), TEXT("hash"), RangeHash) || !QuantityField(Object(V), TEXT("number"), Number) || Number != RangeEnd) { Fail(TEXT("range checkpoint unavailable")); return; }
        RequestLogs();
    });
}
void FAbiTypegenPollSession::RequestLogs()
{
    auto Filter = MakeShared<FJsonObject>(); Filter->SetStringField(TEXT("address"), Address); Filter->SetStringField(TEXT("fromBlock"), Quantity(NextBlock)); Filter->SetStringField(TEXT("toBlock"), Quantity(RangeEnd));
    TArray<TSharedPtr<FJsonValue>> TopicValues; for (const auto& Topic : Topics) TopicValues.Add(MakeShared<FJsonValueString>(Topic)); Filter->SetArrayField(TEXT("topics"), TopicValues);
    Send(TEXT("eth_getLogs"), { MakeShared<FJsonValueObject>(Filter) }, [this](const TSharedPtr<FJsonValue>& V)
    {
        if (V->Type != EJson::Array) { Fail(TEXT("invalid logs result")); return; }
        PendingLogs.Reset(); CanonicalHashes.Reset(); VerifyBlocks.Reset(); VerifyIndex = 0;
        TSet<FString> Identities;
        for (const auto& Entry : V->AsArray())
        {
            auto O = Object(Entry); FAbiTypegenLog Log; FString Data; FString Error; bool Removed = true; FString LogAddress;
            const TArray<TSharedPtr<FJsonValue>>* JsonTopics = nullptr;
            if (!O || !O->TryGetBoolField(TEXT("removed"), Removed) || Removed || !O->TryGetStringField(TEXT("address"), LogAddress) || !LogAddress.Equals(Address, ESearchCase::IgnoreCase) || !HashField(O, TEXT("transactionHash"), Log.TransactionHash) || !HashField(O, TEXT("blockHash"), Log.BlockHash) || !QuantityField(O, TEXT("blockNumber"), Log.BlockNumber) || !QuantityField(O, TEXT("logIndex"), Log.LogIndex) || !QuantityField(O, TEXT("transactionIndex"), Log.TransactionIndex) || Log.BlockNumber < NextBlock || Log.BlockNumber > RangeEnd || !O->TryGetStringField(TEXT("data"), Data) || !AbiTypegen::HexToBytes(Data, Log.Data.Data, Error) || !O->TryGetArrayField(TEXT("topics"), JsonTopics) || JsonTopics->Num() > 4)
            { Fail(TEXT("invalid, removed, or out-of-range event log")); return; }
            for (const auto& Topic : *JsonTopics)
            {
                FAbiTypegenWord256 Word; atg_word Native{};
                if (!Topic || Topic->Type != EJson::String) { Fail(TEXT("invalid log topic")); return; }
                Word.Hex = Topic->AsString();
                if (!Word.TryToWord(Native, Error)) { Fail(Error); return; }
                Log.Topics.Add(Word);
            }
            if (Log.Topics.Num() < Topics.Num()) { Fail(TEXT("event topics do not match filter")); return; }
            for (int32 I = 0; I < Topics.Num(); ++I) if (!Log.Topics[I].Hex.Equals(Topics[I], ESearchCase::IgnoreCase)) { Fail(TEXT("event topics do not match filter")); return; }
            const FString Identity = Log.BlockHash + TEXT(":") + Quantity(Log.LogIndex);
            if (Identities.Contains(Identity)) { Fail(TEXT("duplicate event log in RPC range")); return; }
            Identities.Add(Identity); VerifyBlocks.AddUnique(Log.BlockNumber); PendingLogs.Add(MoveTemp(Log));
        }
        VerifyBlocks.AddUnique(RangeEnd); VerifyNextBlock();
    });
}
void FAbiTypegenPollSession::VerifyNextBlock()
{
    if (VerifyIndex == VerifyBlocks.Num()) { DeliverLogs(); return; }
    const uint64 Number = VerifyBlocks[VerifyIndex++];
    Send(TEXT("eth_getBlockByNumber"), BlockParams(Number), [this, Number](const TSharedPtr<FJsonValue>& V)
    {
        FString Canonical;
        uint64 Actual = 0;
        if (!HashField(Object(V), TEXT("hash"), Canonical) || !QuantityField(Object(V), TEXT("number"), Actual) || Actual != Number) { Fail(TEXT("canonical event block unavailable")); return; }
        CanonicalHashes.Add(Number, Canonical); VerifyNextBlock();
    });
}
void FAbiTypegenPollSession::DeliverLogs()
{
    if (CanonicalHashes.FindChecked(RangeEnd) != RangeHash) { Fail(TEXT("event range changed during query; reorg detected")); return; }
    for (const auto& Log : PendingLogs) if (CanonicalHashes.FindChecked(Log.BlockNumber) != Log.BlockHash) { Fail(TEXT("noncanonical event log; reorg detected")); return; }
    PendingLogs.Sort([](const FAbiTypegenLog& A, const FAbiTypegenLog& B) { return A.BlockNumber == B.BlockNumber ? A.LogIndex < B.LogIndex : A.BlockNumber < B.BlockNumber; });
    if (RangeEnd == MAX_uint64) { Fail(TEXT("event cursor overflow")); return; }
    FString Error;
    if (LogsCompletion && !LogsCompletion(PendingLogs, Error)) { Fail(Error.IsEmpty() ? TEXT("event decode failed") : Error); return; }
    if (bFinished) return;
    AnchorBlock = RangeEnd; AnchorHash = CanonicalHashes.FindChecked(RangeEnd);
    NextBlock = RangeEnd + 1; bBusy = false;

}
