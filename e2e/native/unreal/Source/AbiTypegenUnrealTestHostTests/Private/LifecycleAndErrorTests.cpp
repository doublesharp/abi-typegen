#include "Misc/AutomationTest.h"
#include "AbiTypegenTestOwner.h"
#include "UObject/GarbageCollection.h"

#include "AbiTypegenRpc.h"
#include "AbiTypegenTypes.h"
#include "Generated/atg_Token.h"

#if WITH_DEV_AUTOMATION_TESTS

namespace
{
    FString TestEnv(const TCHAR* Name) { return FPlatformMisc::GetEnvironmentVariable(Name); }

    class FWaitSuppressedCallback final : public IAutomationLatentCommand
    {
    public:
        FWaitSuppressedCallback(FAutomationTestBase* InTest, bool bDestroyOwner)
            : Test(InTest), bDestroy(bDestroyOwner), Start(FPlatformTime::Seconds())
        {
            const FString RpcUrl = TestEnv(TEXT("ATG_RPC_URL"));
            if (RpcUrl.IsEmpty()) { bSkip = true; Test->AddError(TEXT("ATG_RPC_URL is required for lifecycle HTTP test.")); return; }
            Owner = NewObject<UAbiTypegenTestOwner>(GetTransientPackage());
            Owner->AddToRoot();
            Rpc = MakeUnique<FAbiTypegenRpcClient>(RpcUrl);
            Request = Rpc->Request(Owner, TEXT("web3_clientVersion"), {}, [this](FAbiTypegenRpcResponse&&) { bCalled = true; });
            if (bDestroy)
            {
                Owner->RemoveFromRoot();
                Owner->MarkAsGarbage();
                CollectGarbage(RF_NoFlags);
                Owner = nullptr;
            }
            else
            {
                Request->Cancel();
                WeakCanceledRequest = Request;
                Request.Reset();
            }
        }

        virtual ~FWaitSuppressedCallback()
        {
            if (Request.IsValid()) Request->Cancel();
            if (Owner && Owner->IsRooted()) Owner->RemoveFromRoot();
        }

        virtual bool Update() override
        {
            if (bSkip) return true;
            const double Elapsed = FPlatformTime::Seconds() - Start;
            if (bDestroy)
            {
                if (Elapsed < 0.25) return false;
                Test->TestFalse(TEXT("destroyed request owner receives no callback"), bCalled);
                return true;
            }
            if (WeakCanceledRequest.IsValid() && Elapsed < 5.0) return false;
            Test->TestFalse(TEXT("canceled request receives no callback"), bCalled);
            Test->TestFalse(TEXT("canceled request releases HTTP delegate ownership"), WeakCanceledRequest.IsValid());
            if (WeakCanceledRequest.IsValid()) return true;
            if (!bStartedSuccess)
            {
                bStartedSuccess = true;
                SuccessStartedAt = FPlatformTime::Seconds();
                Request = Rpc->Request(Owner, TEXT("web3_clientVersion"), {}, [this](FAbiTypegenRpcResponse&& Response)
                {
                    bSuccessResponse = Response.bOk;
                    bSuccessCallback = true;
                });
                WeakCompletedRequest = Request;
                return false;
            }
            if (!bSuccessCallback && FPlatformTime::Seconds() - SuccessStartedAt < 10.0) return false;
            if (!bSuccessCallback)
            {
                Test->AddError(TEXT("successful HTTP request timed out"));
                return true;
            }
            Request.Reset();
            if (WeakCompletedRequest.IsValid() && FPlatformTime::Seconds() - SuccessStartedAt < 10.0) return false;
            Test->TestTrue(TEXT("web3_clientVersion succeeds"), bSuccessResponse);
            Test->TestFalse(TEXT("completed request releases HTTP delegate ownership"), WeakCompletedRequest.IsValid());
            return true;
        }

    private:
        FAutomationTestBase* Test;
        bool bDestroy = false;
        bool bSkip = false;
        bool bCalled = false;
        bool bStartedSuccess = false;
        bool bSuccessCallback = false;
        bool bSuccessResponse = false;
        double Start;
        double SuccessStartedAt = 0.0;
        UObject* Owner = nullptr;
        TSharedPtr<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> Request;
        TWeakPtr<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> WeakCanceledRequest;
        TWeakPtr<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> WeakCompletedRequest;
        TUniquePtr<FAbiTypegenRpcClient> Rpc;
    };

    class FAnvilErrorFlow final : public IAutomationLatentCommand
    {
    public:
        explicit FAnvilErrorFlow(FAutomationTestBase* InTest)
            : Test(InTest), RpcUrl(TestEnv(TEXT("ATG_RPC_URL"))), Contract(TestEnv(TEXT("ATG_TOKEN_ADDRESS"))), Start(FPlatformTime::Seconds())
        {
            if (RpcUrl.IsEmpty() || Contract.IsEmpty()) { bSkip = true; Test->AddError(TEXT("ATG_RPC_URL/ATG_TOKEN_ADDRESS are required for RPC/revert test.")); return; }
            Owner = NewObject<UAbiTypegenTestOwner>(GetTransientPackage()); Owner->AddToRoot();
            Rpc = MakeUnique<FAbiTypegenRpcClient>(RpcUrl);
        }

        virtual ~FAnvilErrorFlow()
        {
            if (Request.IsValid()) Request->Cancel();
            if (Owner && Owner->IsRooted()) Owner->RemoveFromRoot();
        }

        virtual bool Update() override
        {
            if (bSkip) return true;
            if (FPlatformTime::Seconds() - Start > 15.0) { Test->AddError(TEXT("RPC/revert test timed out")); return true; }
            if (Stage == 0)
            {
                bDone = false;
                Request = Rpc->Request(Owner, TEXT("abi_typegen_method_that_does_not_exist"), {}, [this](FAbiTypegenRpcResponse&& R) { Last = MoveTemp(R); bDone = true; });
                Stage = 1; return false;
            }
            if (Stage == 1)
            {
                if (!bDone) return false;
                Test->TestFalse(TEXT("unknown JSON-RPC method is an error"), Last.bOk);
                Test->TestTrue(TEXT("RPC error has a message"), !Last.Error.IsEmpty());

                atg_Token_atg_transfer_params Params{};
                FString Error;
                AbiTypegen::ParseAddress(TEXT("0x000000000000000000000000000000000000dEaD"), Params.atg_to, Error);
                FAbiTypegenWord256::FromUInt64(1).TryToWord(Params.atg_amount, Error);
                FAbiTypegenResult Encoded(atg_Token_atg_transfer_encode(&Params));
                if (!Encoded.IsOk()) { Test->AddError(Encoded.Error()); return true; }
                const TArray<uint8> Bytes = Encoded.CopyBytes();
                bDone = false;
                Request = Rpc->EthCall(Owner, Contract, AbiTypegen::BytesToHex(Bytes.GetData(), Bytes.Num()), [this](FAbiTypegenRpcResponse&& R) { Last = MoveTemp(R); bDone = true; });
                Stage = 2; return false;
            }
            if (Stage == 2)
            {
                if (!bDone) return false;
                Test->TestFalse(TEXT("reverting eth_call is not reported as success"), Last.bOk);
                Test->TestTrue(TEXT("revert has an RPC error message or data"), !Last.Error.IsEmpty() || !Last.RevertData.IsEmpty());
                return true;
            }
            return true;
        }

    private:
        FAutomationTestBase* Test;
        FString RpcUrl, Contract;
        double Start;
        bool bSkip = false, bDone = false;
        int32 Stage = 0;
        UObject* Owner = nullptr;
        TUniquePtr<FAbiTypegenRpcClient> Rpc;
        TSharedPtr<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> Request;
        FAbiTypegenRpcResponse Last;
    };
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FAbiTypegenCancellationTest, "AbiTypegen.Unreal.Lifecycle.Cancellation", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)
bool FAbiTypegenCancellationTest::RunTest(const FString&) { ADD_LATENT_AUTOMATION_COMMAND(FWaitSuppressedCallback(this, false)); return true; }

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FAbiTypegenDestroyedOwnerTest, "AbiTypegen.Unreal.Lifecycle.DestroyedOwner", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)
bool FAbiTypegenDestroyedOwnerTest::RunTest(const FString&) { ADD_LATENT_AUTOMATION_COMMAND(FWaitSuppressedCallback(this, true)); return true; }

IMPLEMENT_SIMPLE_AUTOMATION_TEST(FAbiTypegenRpcErrorRevertTest, "AbiTypegen.Unreal.Anvil.RpcErrorsAndReverts", EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)
bool FAbiTypegenRpcErrorRevertTest::RunTest(const FString&) { ADD_LATENT_AUTOMATION_COMMAND(FAnvilErrorFlow(this)); return true; }

#endif
