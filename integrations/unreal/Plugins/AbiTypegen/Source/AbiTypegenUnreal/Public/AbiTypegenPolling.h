#pragma once

#include "CoreMinimal.h"
#include "Containers/Ticker.h"
#include "AbiTypegenRpc.h"
#include "AbiTypegenTypes.h"
#include "Dom/JsonObject.h"

/** Polling starts at the explicit block cursor. Confirmations includes the containing block. */
struct ABITYPEGENUNREAL_API FAbiTypegenPollOptions
{
    uint64 Confirmations = 1;
    uint64 BlocksPerRange = 100;
    double IntervalSeconds = 1.0;
    double TimeoutSeconds = 120.0;
};

/** Validated receipt; a mined revert is reported with bSucceeded=false. */
struct ABITYPEGENUNREAL_API FAbiTypegenReceipt
{
    FString TransactionHash;
    FString BlockHash;
    uint64 BlockNumber = 0;
    bool bSucceeded = false;
};

/** Validated canonical log. Dynamic indexed ABI fields remain topic hashes. */
struct ABITYPEGENUNREAL_API FAbiTypegenLog
{
    FString TransactionHash;
    FString BlockHash;
    uint64 BlockNumber = 0;
    uint64 LogIndex = 0;
    uint64 TransactionIndex = 0;
    TArray<FAbiTypegenWord256> Topics;
    FAbiTypegenBytes Data;
};

namespace AbiTypegen
{
    /** Strict lossless uint64 JSON-RPC quantity parser. */
    ABITYPEGENUNREAL_API bool ParseQuantity(const FString& Text, uint64& Out);
    /** Reject incomplete receipts and unknown execution status. */
    ABITYPEGENUNREAL_API bool ParseReceipt(const TSharedPtr<FJsonObject>& Object, FAbiTypegenReceipt& Out, FString& Error);
}

/** Shared HTTP polling session. Retain the returned handle; dropping it cancels requests.
 * Call all methods on the game thread. Cancellation suppresses callbacks. RPC/decode/reorg
 * errors terminate the session without advancing its cursor. A watcher checks the previous
 * range anchor each poll and verifies every log against a canonical block before delivery.
 * The app supplies signer/submission; this adapter never stores keys.
 */
class ABITYPEGENUNREAL_API FAbiTypegenPollSession : public TSharedFromThis<FAbiTypegenPollSession>
{
public:
    using FReceiptCompletion = TFunction<void(const FAbiTypegenReceipt&, const FString&)>;
    using FLogsCompletion = TFunction<bool(const TArray<FAbiTypegenLog>&, FString&)>;
    using FFailure = TFunction<void(const FString&)>;
    /** Injectable RPC transport for deterministic automation and custom HTTP clients. */
    using FTransport = TFunction<TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe>(const FString&, const TArray<TSharedPtr<FJsonValue>>&, FAbiTypegenRpcClient::FCompletion)>;

    static TSharedRef<FAbiTypegenPollSession> WaitReceipt(FTransport Transport, const FString& TransactionHash, const FAbiTypegenPollOptions& Options, FReceiptCompletion Completion);
    static TSharedRef<FAbiTypegenPollSession> WatchLogs(FTransport Transport, const FString& Address, const TArray<FString>& Topics, uint64 StartBlock, const FAbiTypegenPollOptions& Options, FLogsCompletion OnLogs, FFailure OnFailure);
    /** Construct a transport with owner lifetime checks; session also supports explicit Cancel. */
    static FTransport HttpTransport(const FString& RpcUrl, UObject* RequestOwner = nullptr);
    ~FAbiTypegenPollSession();
    void Cancel();
    uint64 GetNextBlock() const { return NextBlock; }
    bool IsFinished() const { return bFinished; }

private:
    FTransport Transport;
    FAbiTypegenPollOptions Options;
    FReceiptCompletion ReceiptCompletion;
    FLogsCompletion LogsCompletion;
    FFailure Failure;
    TSharedPtr<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> Request;
    FTSTicker::FDelegateHandle Ticker;
    double Deadline = 0;
    double NextPoll = 0;
    bool bFinished = false;
    bool bBusy = false;
    bool bReceipt = false;
    FString Hash;
    FString Address;
    TArray<FString> Topics;
    uint64 NextBlock = 0;
    uint64 RangeEnd = 0;
    uint64 AnchorBlock = 0;
    FString AnchorHash;
    FString RangeHash;
    FAbiTypegenReceipt Receipt;
    TArray<FAbiTypegenLog> PendingLogs;
    TMap<uint64, FString> CanonicalHashes;
    TArray<uint64> VerifyBlocks;
    int32 VerifyIndex = 0;

    void Start();
    void Poll();
    void FetchHead();
    void FetchReceipt();
    void FetchLogs(uint64 Head);
    void RequestLogs();
    void VerifyNextBlock();
    void DeliverLogs();
    void Send(const FString& Method, const TArray<TSharedPtr<FJsonValue>>& Params, TFunction<void(const TSharedPtr<FJsonValue>&)> Completion);
    void Fail(const FString& Error);
};
