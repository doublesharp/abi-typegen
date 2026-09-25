#include "AbiTypegenRpc.h"

#include "Async/Async.h"
#include "Dom/JsonObject.h"
#include "HAL/ThreadSafeCounter.h"
#include "HttpModule.h"
#include "Interfaces/IHttpResponse.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"

namespace
{
    FThreadSafeCounter NextRequestId;

    FString ExtractRpcData(const TSharedPtr<FJsonObject>& ErrorObject)
    {
        if (!ErrorObject) return FString();
        const TSharedPtr<FJsonValue>* DataValue = ErrorObject->Values.Find(TEXT("data"));
        if (!DataValue || !DataValue->IsValid()) return FString();
        if ((*DataValue)->Type == EJson::String) return (*DataValue)->AsString();
        if ((*DataValue)->Type == EJson::Object)
        {
            const TSharedPtr<FJsonObject> DataObject = (*DataValue)->AsObject();
            FString Nested;
            if (DataObject && DataObject->TryGetStringField(TEXT("data"), Nested)) return Nested;
        }
        return FString();
    }
}

FAbiTypegenRpcResponse FAbiTypegenRpcClient::ParseResponse(const FString& Body, int32 HttpStatus, int32 ExpectedId)
{
    FAbiTypegenRpcResponse Parsed;
    Parsed.HttpStatus = HttpStatus;
    if (HttpStatus < 200 || HttpStatus >= 300)
    {
        Parsed.Error = FString::Printf(TEXT("HTTP status %d"), HttpStatus);
        return Parsed;
    }

    TSharedPtr<FJsonObject> Json;
    const TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Body);
    if (!FJsonSerializer::Deserialize(Reader, Json) || !Json.IsValid())
    {
        Parsed.Error = TEXT("invalid JSON-RPC response");
        return Parsed;
    }

    FString Version;
    const TSharedPtr<FJsonValue>* Id = Json->Values.Find(TEXT("id"));
    if (!Json->TryGetStringField(TEXT("jsonrpc"), Version) || Version != TEXT("2.0") ||
        !Id || !Id->IsValid() || (*Id)->Type != EJson::Number || (*Id)->AsNumber() != ExpectedId)
    {
        Parsed.Error = TEXT("JSON-RPC response version or id mismatch");
        return Parsed;
    }

    const TSharedPtr<FJsonValue>* ErrorValue = Json->Values.Find(TEXT("error"));
    const TSharedPtr<FJsonValue>* ResultValue = Json->Values.Find(TEXT("result"));
    if ((ErrorValue != nullptr) == (ResultValue != nullptr))
    {
        Parsed.Error = TEXT("JSON-RPC response must contain exactly one result or error");
        return Parsed;
    }
    if (ErrorValue)
    {
        const TSharedPtr<FJsonObject> ErrorObject = ErrorValue->IsValid() && (*ErrorValue)->Type == EJson::Object
            ? (*ErrorValue)->AsObject() : nullptr;
        double Code = 0.0;
        if (!ErrorObject.IsValid() || !ErrorObject->TryGetNumberField(TEXT("code"), Code) ||
            !FMath::IsFinite(Code) || Code < MIN_int32 || Code > MAX_int32 ||
            static_cast<int32>(Code) != Code ||
            !ErrorObject->TryGetStringField(TEXT("message"), Parsed.Error))
        {
            Parsed.Error = TEXT("invalid JSON-RPC error object");
            return Parsed;
        }
        Parsed.RpcErrorCode = static_cast<int32>(Code);
        Parsed.RevertData = ExtractRpcData(ErrorObject);
        return Parsed;
    }
    if (!ResultValue->IsValid())
    {
        Parsed.Error = TEXT("invalid JSON-RPC result");
        return Parsed;
    }
    Parsed.bOk = true;
    Parsed.Result = *ResultValue;
    return Parsed;
}

void FAbiTypegenRpcRequest::Cancel()
{
    bCanceled = true;
    if (HttpRequest.IsValid())
    {
        HttpRequest->OnProcessRequestComplete().Unbind();
        HttpRequest->CancelRequest();
        HttpRequest.Reset();
    }
}

TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> FAbiTypegenRpcClient::Request(
    UObject* RequestOwner,
    const FString& Method,
    const TArray<TSharedPtr<FJsonValue>>& Params,
    FCompletion Completion) const
{
    TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> State = MakeShared<FAbiTypegenRpcRequest, ESPMode::ThreadSafe>();
    const int32 RequestId = NextRequestId.Increment();
    TWeakObjectPtr<UObject> WeakOwner(RequestOwner);
    const bool bHasOwner = RequestOwner != nullptr;
    TSharedRef<FCompletion, ESPMode::ThreadSafe> SharedCompletion = MakeShared<FCompletion, ESPMode::ThreadSafe>(MoveTemp(Completion));

    TSharedRef<FJsonObject> Root = MakeShared<FJsonObject>();
    Root->SetStringField(TEXT("jsonrpc"), TEXT("2.0"));
    Root->SetNumberField(TEXT("id"), RequestId);
    Root->SetStringField(TEXT("method"), Method);
    Root->SetArrayField(TEXT("params"), Params);

    FString Body;
    TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Body);
    FJsonSerializer::Serialize(Root, Writer);

    TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Http = FHttpModule::Get().CreateRequest();
    Http->SetDelegateThreadPolicy(EHttpRequestDelegateThreadPolicy::CompleteOnGameThread);
    State->HttpRequest = Http;
    Http->SetURL(Url);
    Http->SetVerb(TEXT("POST"));
    Http->SetHeader(TEXT("Content-Type"), TEXT("application/json"));
    Http->SetContentAsString(Body);

    Http->OnProcessRequestComplete().BindLambda(
        [State, WeakOwner, bHasOwner, SharedCompletion, RequestId](FHttpRequestPtr, FHttpResponsePtr Response, bool bSucceeded) mutable
        {
            // The delegate owns State and State owns the HTTP request. Break that cycle
            // before posting completion to the game thread.
            State->HttpRequest.Reset();
            FAbiTypegenRpcResponse Parsed;
            Parsed.bCanceled = State->bCanceled;
            if (Response.IsValid()) Parsed.HttpStatus = Response->GetResponseCode();

            if (Parsed.bCanceled)
            {
                Parsed.Error = TEXT("request canceled");
            }
            else if (!bSucceeded || !Response.IsValid())
            {
                Parsed.Error = TEXT("HTTP request failed");
            }
            else
            {
                Parsed = ParseResponse(Response->GetContentAsString(), Response->GetResponseCode(), RequestId);
            }

            AsyncTask(ENamedThreads::GameThread, [State, WeakOwner, bHasOwner, Parsed = MoveTemp(Parsed), SharedCompletion]() mutable
            {
                if (State->bCanceled || State->bCompleted) return;
                State->bCompleted = true;
                if (bHasOwner && !WeakOwner.IsValid()) return;
                (*SharedCompletion)(MoveTemp(Parsed));
            });
        });

    if (!Http->ProcessRequest())
    {
        Http->OnProcessRequestComplete().Unbind();
        State->HttpRequest.Reset();
        FAbiTypegenRpcResponse Failure;
        Failure.Error = TEXT("HTTP request could not be started");
        AsyncTask(ENamedThreads::GameThread, [State, WeakOwner, bHasOwner, Failure = MoveTemp(Failure), SharedCompletion]() mutable
        {
            if (State->bCanceled || State->bCompleted) return;
            State->bCompleted = true;
            if (bHasOwner && !WeakOwner.IsValid()) return;
            (*SharedCompletion)(MoveTemp(Failure));
        });
    }

    return State;
}

TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> FAbiTypegenRpcClient::EthCall(
    UObject* RequestOwner,
    const FString& To,
    const FString& Data,
    FCompletion Completion,
    const FString& Block) const
{
    TSharedRef<FJsonObject> Call = MakeShared<FJsonObject>();
    Call->SetStringField(TEXT("to"), To);
    Call->SetStringField(TEXT("data"), Data);
    TArray<TSharedPtr<FJsonValue>> Params;
    Params.Add(MakeShared<FJsonValueObject>(Call));
    Params.Add(MakeShared<FJsonValueString>(Block));
    return Request(RequestOwner, TEXT("eth_call"), Params, MoveTemp(Completion));
}
