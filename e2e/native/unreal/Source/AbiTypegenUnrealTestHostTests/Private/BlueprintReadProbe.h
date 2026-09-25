#pragma once

#include "CoreMinimal.h"
#include "Generated/TokenUnreal.h"
#include "BlueprintReadProbe.generated.h"

UCLASS()
class UAbiTypegenBlueprintReadProbe : public UObject
{
    GENERATED_BODY()
public:
    bool bDone = false;
    bool bSucceeded = false;
    FTokenBalanceOfResult Result;
    FString Error;

    UFUNCTION()
    void HandleSuccess(FTokenBalanceOfResult InResult)
    {
        bDone = true;
        bSucceeded = true;
        Result = InResult;
    }

    UFUNCTION()
    void HandleFailure(FString InError)
    {
        bDone = true;
        bSucceeded = false;
        Error = MoveTemp(InError);
    }
};
