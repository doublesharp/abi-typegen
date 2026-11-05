#include "AbiTypegenRpc.h"
#include "AbiTypegenSigner.h"

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

bool FAbiTypegenRpcClient::ParseGasEstimate(const FString& Quantity, FAbiTypegenWord256& Out, FString& Error)
{
    if (!Quantity.StartsWith(TEXT("0x")) || Quantity.Len() < 3 || Quantity.Len() > 66 || (Quantity.Len() > 3 && Quantity[2] == '0'))
    { Error = TEXT("invalid gas estimate quantity"); return false; }
    FAbiTypegenWord256 Word; Word.Hex = TEXT("0x") + FString::ChrN(66 - Quantity.Len(), '0') + Quantity.Mid(2);
    atg_word Native{};
    if (!Word.TryToWord(Native, Error)) return false;
    Out = FAbiTypegenWord256::FromWord(Native); Error.Reset(); return true;
}

bool FAbiTypegenRpcClient::BuildTransactionObject(const FAbiTypegenUnsignedTransaction& Transaction, const FString& From, TSharedPtr<FJsonObject>& Out, FString& Error)
{
    atg_address ToAddress{}, FromAddress{}; atg_word Value{}; TArray<uint8> Data;
    if (!AbiTypegen::ParseAddress(Transaction.To, ToAddress, Error) || !AbiTypegen::ParseAddress(From, FromAddress, Error) || !Transaction.Data.StartsWith(TEXT("0x")) || !AbiTypegen::HexToBytes(Transaction.Data, Data, Error) || !Transaction.Value.TryToWord(Value, Error))
    { if (Error.IsEmpty()) Error = TEXT("invalid transaction"); return false; }
    auto Object = MakeShared<FJsonObject>(); Object->SetStringField(TEXT("to"), AbiTypegen::AddressToHex(ToAddress)); Object->SetStringField(TEXT("from"), AbiTypegen::AddressToHex(FromAddress)); Object->SetStringField(TEXT("data"), AbiTypegen::BytesToHex(Data.GetData(), Data.Num()));
    FString Hex = FAbiTypegenWord256::FromWord(Value).Hex.Mid(2); int32 First = 0; while (First < Hex.Len() - 1 && Hex[First] == '0') ++First;
    Object->SetStringField(TEXT("value"), TEXT("0x") + Hex.Mid(First));
    if (Transaction.GasLimit.IsSet()) Object->SetStringField(TEXT("gas"), FString::Printf(TEXT("0x%llx"), static_cast<unsigned long long>(Transaction.GasLimit.GetValue())));
    Out = Object; Error.Reset(); return true;
}

namespace
{
    TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> TransactionFailure(UObject* Owner, const FString& Error, FAbiTypegenRpcClient::FCompletion Completion)
    {
        auto Request = MakeShared<FAbiTypegenRpcRequest, ESPMode::ThreadSafe>(); TWeakObjectPtr<UObject> WeakOwner(Owner); bool bHasOwner = Owner != nullptr;
        AsyncTask(ENamedThreads::GameThread, [Request, WeakOwner, bHasOwner, Error, Completion = MoveTemp(Completion)]() mutable
        {
            if (Request->IsCanceled() || (bHasOwner && !WeakOwner.IsValid())) return;
            FAbiTypegenRpcResponse Response; Response.Error = Error; Completion(MoveTemp(Response));
        }); return Request;
    }
}
TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> FAbiTypegenRpcClient::SimulateTransaction(UObject* RequestOwner, const FAbiTypegenUnsignedTransaction& Transaction, const FString& From, FCompletion Completion, const FString& Block) const
{
    TSharedPtr<FJsonObject> Object; FString Error;
    if (!BuildTransactionObject(Transaction, From, Object, Error)) return TransactionFailure(RequestOwner, Error, MoveTemp(Completion));
    return Request(RequestOwner, TEXT("eth_call"), { MakeShared<FJsonValueObject>(Object), MakeShared<FJsonValueString>(Block) }, [Completion = MoveTemp(Completion)](FAbiTypegenRpcResponse&& Response) mutable
    {
        TArray<uint8> Bytes; FString Error;
        if (Response.bOk && (!Response.Result || Response.Result->Type != EJson::String || !Response.Result->AsString().StartsWith(TEXT("0x")) || !AbiTypegen::HexToBytes(Response.Result->AsString(), Bytes, Error)))
        { Response.bOk = false; Response.Error = TEXT("invalid eth_call byte result"); }
        Completion(MoveTemp(Response));
    });
}
TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> FAbiTypegenRpcClient::EstimateGas(UObject* RequestOwner, const FAbiTypegenUnsignedTransaction& Transaction, const FString& From, FCompletion Completion) const
{
    TSharedPtr<FJsonObject> Object; FString Error;
    if (!BuildTransactionObject(Transaction, From, Object, Error)) return TransactionFailure(RequestOwner, Error, MoveTemp(Completion));
    return Request(RequestOwner, TEXT("eth_estimateGas"), { MakeShared<FJsonValueObject>(Object) }, [Completion = MoveTemp(Completion)](FAbiTypegenRpcResponse&& Response) mutable
    {
        FAbiTypegenWord256 Gas; FString Error;
        if (Response.bOk && (!Response.Result || Response.Result->Type != EJson::String || !ParseGasEstimate(Response.Result->AsString(), Gas, Error)))
        { Response.bOk = false; Response.Error = Error.IsEmpty() ? TEXT("invalid gas estimate result") : Error; }
        Completion(MoveTemp(Response));
    });
}
