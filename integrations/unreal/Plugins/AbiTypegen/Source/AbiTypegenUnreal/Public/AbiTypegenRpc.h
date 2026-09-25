#pragma once

#include "CoreMinimal.h"
#include "Dom/JsonValue.h"
#include "Interfaces/IHttpRequest.h"
#include "Templates/Function.h"

struct ABITYPEGENUNREAL_API FAbiTypegenRpcResponse
{
    bool bOk = false;
    bool bCanceled = false;
    int32 HttpStatus = 0;
    int64 RpcErrorCode = 0;
    FString Error;
    FString RevertData;
    TSharedPtr<FJsonValue> Result;
};

class ABITYPEGENUNREAL_API FAbiTypegenRpcRequest : public TSharedFromThis<FAbiTypegenRpcRequest, ESPMode::ThreadSafe>
{
public:
    FAbiTypegenRpcRequest() = default;
    void Cancel();
    bool IsCanceled() const { return bCanceled; }

private:
    friend class FAbiTypegenRpcClient;
    TSharedPtr<IHttpRequest, ESPMode::ThreadSafe> HttpRequest;
    TAtomic<bool> bCanceled{false};
    TAtomic<bool> bCompleted{false};
};

class ABITYPEGENUNREAL_API FAbiTypegenRpcClient
{
public:
    using FCompletion = TFunction<void(FAbiTypegenRpcResponse&&)>;

    /** Validate one HTTP JSON-RPC response against its request id. */
    static FAbiTypegenRpcResponse ParseResponse(const FString& Body, int32 HttpStatus, int32 ExpectedId);

    explicit FAbiTypegenRpcClient(FString InUrl) : Url(MoveTemp(InUrl)) {}

    TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> Request(
        UObject* RequestOwner,
        const FString& Method,
        const TArray<TSharedPtr<FJsonValue>>& Params,
        FCompletion Completion) const;

    TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> EthCall(
        UObject* RequestOwner,
        const FString& To,
        const FString& Data,
        FCompletion Completion,
        const FString& Block = TEXT("latest")) const;

    const FString& GetUrl() const { return Url; }

private:
    FString Url;
};
