#pragma once
#include "CoreMinimal.h"
#include "Generated/TokenUnreal.h"
#include "BlueprintEventProbe.generated.h"

UCLASS()
class UAbiTypegenBlueprintEventProbe : public UObject
{
    GENERATED_BODY()
public:
    bool bReceived = false;
    int32 ReceivedCount = 0;
    UTokenTransferEventWatcher* CancelTarget = nullptr;
    FString Error;
    FTokenTransferEvent Event;
    UFUNCTION() void Received(FTokenTransferEvent Value) { Event = Value; bReceived = true; ++ReceivedCount; if (CancelTarget) CancelTarget->Cancel(); }
    UFUNCTION() void Failed(FString Value) { Error = Value; }
};
