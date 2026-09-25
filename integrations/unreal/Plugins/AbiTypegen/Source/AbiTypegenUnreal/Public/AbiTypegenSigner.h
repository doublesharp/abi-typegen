#pragma once

#include "CoreMinimal.h"
#include "AbiTypegenRpc.h"
#include "AbiTypegenTypes.h"

struct ABITYPEGENUNREAL_API FAbiTypegenUnsignedTransaction
{
    FString To;
    FString Data;
    FAbiTypegenWord256 Value;
    TOptional<uint64> GasLimit;
};

/**
 * Application-owned signing/submission boundary.
 * The abi-typegen Unreal runtime never stores production private keys and does not implement signing.
 */
class ABITYPEGENUNREAL_API IAbiTypegenSigner
{
public:
    virtual ~IAbiTypegenSigner() = default;
    virtual TSharedPtr<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> SubmitAsync(
        UObject* RequestOwner,
        const FAbiTypegenRpcClient& Rpc,
        const FAbiTypegenUnsignedTransaction& Transaction,
        FAbiTypegenRpcClient::FCompletion Completion) = 0;
};
