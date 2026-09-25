#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintAsyncActionBase.h"
#include "AbiTypegenRpc.h"
#include "AbiTypegenAsyncActionBase.generated.h"

UCLASS(Abstract)
class ABITYPEGENUNREAL_API UAbiTypegenAsyncActionBase : public UBlueprintAsyncActionBase
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintCallable, Category="abi-typegen")
    virtual void Cancel();

    virtual void BeginDestroy() override;

protected:
    void InitializeLifetime(const UObject* WorldContextObject, UObject* InRequestOwner);
    void TrackRequest(TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> InRequest);
    bool IsRequestOwnerAlive() const;
    void Finish();

private:
    TWeakObjectPtr<UObject> RequestOwner;
    TSharedPtr<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> Request;
};
