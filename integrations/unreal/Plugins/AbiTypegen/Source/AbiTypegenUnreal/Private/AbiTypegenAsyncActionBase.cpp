#include "AbiTypegenAsyncActionBase.h"

#include "Engine/Engine.h"
#include "Engine/GameInstance.h"
#include "Engine/World.h"

void UAbiTypegenAsyncActionBase::InitializeLifetime(const UObject* WorldContextObject, UObject* InRequestOwner)
{
    if (WorldContextObject && GEngine)
    {
        if (UWorld* World = GEngine->GetWorldFromContextObject(WorldContextObject, EGetWorldErrorMode::ReturnNull))
        {
            if (UGameInstance* GameInstance = World->GetGameInstance())
            {
                RegisterWithGameInstance(GameInstance);
            }
        }
    }
    RequestOwner = InRequestOwner ? InRequestOwner : const_cast<UObject*>(WorldContextObject);
}

void UAbiTypegenAsyncActionBase::TrackRequest(TSharedRef<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> InRequest)
{
    Request = InRequest;
}

bool UAbiTypegenAsyncActionBase::IsRequestOwnerAlive() const
{
    return !RequestOwner.IsStale() && RequestOwner.IsValid();
}

void UAbiTypegenAsyncActionBase::Cancel()
{
    if (Request.IsValid()) Request->Cancel();
    Request.Reset();
    SetReadyToDestroy();
}

void UAbiTypegenAsyncActionBase::Finish()
{
    Request.Reset();
    SetReadyToDestroy();
}

void UAbiTypegenAsyncActionBase::BeginDestroy()
{
    if (Request.IsValid()) Request->Cancel();
    Request.Reset();
    Super::BeginDestroy();
}
