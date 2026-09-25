#pragma once

#include "AbiTypegenSigner.h"
#include "Dom/JsonObject.h"

/** Test-only signer/submission adapter. It relies on Anvil's unlocked development account. */
class FAnvilUnlockedDevelopmentSigner final : public IAbiTypegenSigner
{
public:
    explicit FAnvilUnlockedDevelopmentSigner(FString InFrom) : From(MoveTemp(InFrom)) {}

    virtual TSharedPtr<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> SubmitAsync(
        UObject* RequestOwner,
        const FAbiTypegenRpcClient& Rpc,
        const FAbiTypegenUnsignedTransaction& Transaction,
        FAbiTypegenRpcClient::FCompletion Completion) override
    {
        TSharedRef<FJsonObject> Tx = MakeShared<FJsonObject>();
        Tx->SetStringField(TEXT("from"), From);
        Tx->SetStringField(TEXT("to"), Transaction.To);
        Tx->SetStringField(TEXT("data"), Transaction.Data);
        Tx->SetStringField(TEXT("value"), Transaction.Value.Hex);
        if (Transaction.GasLimit.IsSet())
        {
            Tx->SetStringField(TEXT("gas"), FString::Printf(TEXT("0x%llx"), Transaction.GasLimit.GetValue()));
        }

        TArray<TSharedPtr<FJsonValue>> Params;
        Params.Add(MakeShared<FJsonValueObject>(Tx));
        return Rpc.Request(RequestOwner, TEXT("eth_sendTransaction"), Params, MoveTemp(Completion));
    }

private:
    FString From;
};
