#include "Misc/AutomationTest.h"
#include "AbiTypegenTestOwner.h"

#include "AbiTypegenRpc.h"
#include "AbiTypegenTypes.h"
#include "AnvilDevelopmentSigner.h"
#include "BlueprintReadProbe.h"
#include "Generated/TokenUnreal.h"
#include "Generated/atg_Token.h"

#if WITH_DEV_AUTOMATION_TESTS

namespace
{
    FString Env(const TCHAR* Name)
    {
        return FPlatformMisc::GetEnvironmentVariable(Name);
    }

    bool JsonString(const FAbiTypegenRpcResponse& Response, FString& Out)
    {
        if (!Response.bOk || !Response.Result.IsValid() || Response.Result->Type != EJson::String) return false;
        Out = Response.Result->AsString();
        return true;
    }

    FAbiTypegenWord256 U64(uint64 Value)
    {
        return FAbiTypegenWord256::FromUInt64(Value);
    }

    FAbiTypegenWord256 LargeAmount()
    {
        FAbiTypegenWord256 Amount;
        Amount.Hex = TEXT("0x0000000000000100000000000000000000000000000000000000000000001234");
        return Amount; // 2^200 + 0x1234.
    }

    class FAnvilReadWriteCommand final : public IAutomationLatentCommand
    {
    public:
        explicit FAnvilReadWriteCommand(FAutomationTestBase* InTest)
            : Test(InTest), RpcUrl(Env(TEXT("ATG_RPC_URL"))), ContractHex(Env(TEXT("ATG_TOKEN_ADDRESS"))), StartedAt(FPlatformTime::Seconds())
        {
            if (RpcUrl.IsEmpty() || ContractHex.IsEmpty())
            {
                bSkipped = true;
                Test->AddError(TEXT("ATG_RPC_URL/ATG_TOKEN_ADDRESS are required for Anvil live test."));
                return;
            }
            Owner = NewObject<UAbiTypegenTestOwner>(GetTransientPackage());
            Owner->AddToRoot();
            Rpc = MakeUnique<FAbiTypegenRpcClient>(RpcUrl);
        }

        virtual ~FAnvilReadWriteCommand()
        {
            if (Request.IsValid()) Request->Cancel();
            if (Owner && Owner->IsRooted()) Owner->RemoveFromRoot();
        }

        virtual bool Update() override
        {
            if (bSkipped) return true;
            if (FPlatformTime::Seconds() - StartedAt > 45.0)
            {
                Test->AddError(TEXT("Timed out during Anvil read/write flow"));
                Cleanup();
                return true;
            }

            switch (Stage)
            {
                case 0: StartAccounts(); return false;
                case 1: if (!ConsumeCallback()) return false; return HandleAccounts();
                case 2: StartMint(); return false;
                case 3: if (!ConsumeCallback()) return false; return HandleTxHash(4);
                case 4: return PollReceipt(5);
                case 5: StartBalanceRead(); return false;
                case 6: if (!ConsumeCallback()) return false; return HandleBalance();
                case 7: StartApprove(); return false;
                case 8: if (!ConsumeCallback()) return false; return HandleTxHash(9);
                case 9: return PollReceipt(10);
                case 10: StartAllowanceRead(); return false;
                case 11: if (!ConsumeCallback()) return false; return HandleAllowance();
                default: Cleanup(); return true;
            }
        }

    private:
        void StartAccounts()
        {
            Begin(TEXT("eth_accounts"), {});
            Stage = 1;
        }

        bool HandleAccounts()
        {
            if (!Last.bOk || !Last.Result.IsValid() || Last.Result->Type != EJson::Array)
            {
                Fail(Last.Error.IsEmpty() ? TEXT("eth_accounts failed") : Last.Error);
                return true;
            }
            const TArray<TSharedPtr<FJsonValue>>& Accounts = Last.Result->AsArray();
            if (Accounts.IsEmpty() || Accounts[0]->Type != EJson::String)
            {
                Fail(TEXT("Anvil returned no development account"));
                return true;
            }
            From = Accounts[0]->AsString();
            Signer = MakeUnique<FAnvilUnlockedDevelopmentSigner>(From);
            Stage = 2;
            return false;
        }

        void StartMint()
        {
            atg_Token_atg_mint_params Params{};
            FString Error;
            if (!AbiTypegen::ParseAddress(From, Params.atg_to, Error) || !LargeAmount().TryToWord(Params.atg_amount, Error))
            {
                Fail(Error); return;
            }
            FAbiTypegenResult Encoded(atg_Token_atg_mint_encode(&Params));
            if (!Encoded.IsOk()) { Fail(Encoded.Error()); return; }
            const TArray<uint8> Bytes = Encoded.CopyBytes();
            FAbiTypegenUnsignedTransaction Tx;
            Tx.To = ContractHex;
            Tx.Data = AbiTypegen::BytesToHex(Bytes.GetData(), Bytes.Num(), true);
            Tx.Value = U64(0);
            Tx.GasLimit = 300000;
            bCallback = false;
            Request = Signer->SubmitAsync(Owner, *Rpc, Tx, [this](FAbiTypegenRpcResponse&& R) { Last = MoveTemp(R); bCallback = true; });
            Stage = 3;
        }

        bool HandleTxHash(int32 NextReceiptStage)
        {
            FString Hash;
            if (!JsonString(Last, Hash))
            {
                Fail(Last.Error.IsEmpty() ? TEXT("transaction submission failed") : Last.Error);
                return true;
            }
            TxHash = Hash;
            NextPollAt = 0.0;
            Stage = NextReceiptStage;
            return false;
        }

        bool PollReceipt(int32 NextStage)
        {
            const double Now = FPlatformTime::Seconds();
            if (bWaitingReceipt)
            {
                if (!ConsumeCallback()) return false;
                bWaitingReceipt = false;
                if (!Last.bOk)
                {
                    Fail(Last.Error); return true;
                }
                if (!Last.Result.IsValid() || Last.Result->Type == EJson::Null)
                {
                    NextPollAt = Now + 0.1;
                    return false;
                }
                if (Last.Result->Type != EJson::Object)
                {
                    Fail(TEXT("eth_getTransactionReceipt returned a non-object")); return true;
                }
                const TSharedPtr<FJsonObject> Receipt = Last.Result->AsObject();
                FString Status;
                if (!Receipt.IsValid() || !Receipt->TryGetStringField(TEXT("status"), Status) || !Status.Equals(TEXT("0x1"), ESearchCase::IgnoreCase))
                {
                    Fail(TEXT("transaction receipt was not successful")); return true;
                }
                if (!VerifyReceiptEvent(*Receipt, Stage == 4)) return true;
                Stage = NextStage;
                return false;
            }
            if (Now < NextPollAt) return false;
            TArray<TSharedPtr<FJsonValue>> Params;
            Params.Add(MakeShared<FJsonValueString>(TxHash));
            Begin(TEXT("eth_getTransactionReceipt"), Params);
            bWaitingReceipt = true;
            return false;
        }

        void StartBalanceRead()
        {
            atg_Token_atg_balanceOf_params Params{};
            FString Error;
            if (!AbiTypegen::ParseAddress(From, Params.atg_field, Error)) { Fail(Error); return; }
            FAbiTypegenResult Encoded(atg_Token_atg_balanceOf_encode(&Params));
            if (!Encoded.IsOk()) { Fail(Encoded.Error()); return; }
            const TArray<uint8> Bytes = Encoded.CopyBytes();
            bCallback = false;
            Request = Rpc->EthCall(Owner, ContractHex, AbiTypegen::BytesToHex(Bytes.GetData(), Bytes.Num()), [this](FAbiTypegenRpcResponse&& R) { Last = MoveTemp(R); bCallback = true; });
            Stage = 6;
        }

        bool HandleBalance()
        {
            if (!DecodeWordResult<atg_Token_atg_balanceOf_returns>(Last, atg_Token_atg_balanceOf_decode, LargeAmount(), TEXT("balanceOf"))) return true;
            Stage = 7;
            return false;
        }

        void StartApprove()
        {
            atg_Token_atg_approve_params Params{};
            FString Error;
            if (!AbiTypegen::ParseAddress(From, Params.atg_spender, Error) || !LargeAmount().TryToWord(Params.atg_amount, Error)) { Fail(Error); return; }
            FAbiTypegenResult Encoded(atg_Token_atg_approve_encode(&Params));
            if (!Encoded.IsOk()) { Fail(Encoded.Error()); return; }
            const TArray<uint8> Bytes = Encoded.CopyBytes();
            FAbiTypegenUnsignedTransaction Tx;
            Tx.To = ContractHex;
            Tx.Data = AbiTypegen::BytesToHex(Bytes.GetData(), Bytes.Num());
            Tx.Value = U64(0);
            Tx.GasLimit = 300000;
            bCallback = false;
            Request = Signer->SubmitAsync(Owner, *Rpc, Tx, [this](FAbiTypegenRpcResponse&& R) { Last = MoveTemp(R); bCallback = true; });
            Stage = 8;
        }

        void StartAllowanceRead()
        {
            atg_Token_atg_allowance_params Params{};
            FString Error;
            if (!AbiTypegen::ParseAddress(From, Params.atg_field, Error) || !AbiTypegen::ParseAddress(From, Params.atg_field2, Error)) { Fail(Error); return; }
            FAbiTypegenResult Encoded(atg_Token_atg_allowance_encode(&Params));
            if (!Encoded.IsOk()) { Fail(Encoded.Error()); return; }
            const TArray<uint8> Bytes = Encoded.CopyBytes();
            bCallback = false;
            Request = Rpc->EthCall(Owner, ContractHex, AbiTypegen::BytesToHex(Bytes.GetData(), Bytes.Num()), [this](FAbiTypegenRpcResponse&& R) { Last = MoveTemp(R); bCallback = true; });
            Stage = 11;
        }

        bool HandleAllowance()
        {
            if (!DecodeWordResult<atg_Token_atg_allowance_returns>(Last, atg_Token_atg_allowance_decode, LargeAmount(), TEXT("allowance"))) return true;
            Stage = 12;
            return false;
        }

        template <typename TReturns>
        bool DecodeWordResult(const FAbiTypegenRpcResponse& Response, atg_result* (*Decoder)(const uint8_t*, size_t, TReturns*), const FAbiTypegenWord256& Expected, const TCHAR* Label)
        {
            FString RawHex;
            if (!JsonString(Response, RawHex)) { Fail(Response.Error.IsEmpty() ? FString::Printf(TEXT("%s RPC failed"), Label) : Response.Error); return false; }
            TArray<uint8> Raw;
            FString Error;
            if (!AbiTypegen::HexToBytes(RawHex, Raw, Error)) { Fail(Error); return false; }
            TReturns Native{};
            FAbiTypegenResult Decoded(Decoder(Raw.GetData(), Raw.Num(), &Native));
            if (!Decoded.IsOk()) { Fail(Decoded.Error()); return false; }
            Test->TestEqual(Label, FAbiTypegenWord256::FromWord(Native.atg_field).Hex, Expected.Hex);
            return true;
        }

        bool VerifyReceiptEvent(const FJsonObject& Receipt, bool bTransfer)
        {
            const TArray<TSharedPtr<FJsonValue>>* Logs = nullptr;
            if (!Receipt.TryGetArrayField(TEXT("logs"), Logs) || !Logs)
            {
                Fail(TEXT("receipt is missing logs")); return false;
            }
            const FString ExpectedTopic = AbiTypegen::BytesToHex(
                bTransfer ? atg_Token_atg_Transfer_event_topic : atg_Token_atg_Approval_event_topic, 32, true);
            atg_address FromAddress{};
            FString Error;
            if (!AbiTypegen::ParseAddress(From, FromAddress, Error)) { Fail(Error); return false; }
            for (const TSharedPtr<FJsonValue>& Entry : *Logs)
            {
                if (!Entry.IsValid() || Entry->Type != EJson::Object) continue;
                const TSharedPtr<FJsonObject> Log = Entry->AsObject();
                FString Address, DataHex;
                const TArray<TSharedPtr<FJsonValue>>* Topics = nullptr;
                if (!Log.IsValid() || !Log->TryGetStringField(TEXT("address"), Address) ||
                    !Address.Equals(ContractHex, ESearchCase::IgnoreCase) ||
                    !Log->TryGetStringField(TEXT("data"), DataHex) ||
                    !Log->TryGetArrayField(TEXT("topics"), Topics) || !Topics || Topics->Num() != 3 ||
                    !(*Topics)[0].IsValid() || (*Topics)[0]->Type != EJson::String ||
                    !(*Topics)[0]->AsString().Equals(ExpectedTopic, ESearchCase::IgnoreCase)) continue;
                TArray<uint8> TopicBytes, DataBytes;
                for (const TSharedPtr<FJsonValue>& Topic : *Topics)
                {
                    TArray<uint8> Word;
                    if (!Topic.IsValid() || Topic->Type != EJson::String ||
                        !AbiTypegen::HexToBytes(Topic->AsString(), Word, Error) || Word.Num() != 32)
                    {
                        Fail(TEXT("malformed receipt event topic")); return false;
                    }
                    TopicBytes.Append(Word);
                }
                if (!AbiTypegen::HexToBytes(DataHex, DataBytes, Error)) { Fail(Error); return false; }
                atg_word ExpectedWord{};
                if (!LargeAmount().TryToWord(ExpectedWord, Error)) { Fail(Error); return false; }
                if (bTransfer)
                {
                    atg_Token_atg_Transfer_event_fields Event{};
                    FAbiTypegenResult Decoded(atg_Token_atg_Transfer_event_decode(TopicBytes.GetData(), 3, DataBytes.GetData(), DataBytes.Num(), &Event));
                    if (!Decoded.IsOk()) { Fail(Decoded.Error()); return false; }
                    Test->TestTrue(TEXT("Transfer from zero address"), FMemory::Memcmp(Event.atg_from.bytes, atg_address{}.bytes, 20) == 0);
                    Test->TestTrue(TEXT("Transfer recipient"), FMemory::Memcmp(Event.atg_to.bytes, FromAddress.bytes, 20) == 0);
                    Test->TestTrue(TEXT("Transfer amount"), FMemory::Memcmp(Event.atg_amount.bytes, ExpectedWord.bytes, 32) == 0);
                }
                else
                {
                    atg_Token_atg_Approval_event_fields Event{};
                    FAbiTypegenResult Decoded(atg_Token_atg_Approval_event_decode(TopicBytes.GetData(), 3, DataBytes.GetData(), DataBytes.Num(), &Event));
                    if (!Decoded.IsOk()) { Fail(Decoded.Error()); return false; }
                    Test->TestTrue(TEXT("Approval owner"), FMemory::Memcmp(Event.atg_owner.bytes, FromAddress.bytes, 20) == 0);
                    Test->TestTrue(TEXT("Approval spender"), FMemory::Memcmp(Event.atg_spender.bytes, FromAddress.bytes, 20) == 0);
                    Test->TestTrue(TEXT("Approval amount"), FMemory::Memcmp(Event.atg_amount.bytes, ExpectedWord.bytes, 32) == 0);
                }
                return true;
            }
            Fail(bTransfer ? TEXT("Transfer log not found in mint receipt") : TEXT("Approval log not found in approve receipt"));
            return false;
        }

        void Begin(const FString& Method, const TArray<TSharedPtr<FJsonValue>>& Params)
        {
            bCallback = false;
            Request = Rpc->Request(Owner, Method, Params, [this](FAbiTypegenRpcResponse&& R) { Last = MoveTemp(R); bCallback = true; });
        }

        bool ConsumeCallback()
        {
            if (!bCallback) return false;
            bCallback = false;
            Request.Reset();
            return true;
        }

        void Fail(const FString& Message)
        {
            Test->AddError(Message.IsEmpty() ? TEXT("unknown Anvil test failure") : Message);
            Stage = 99;
        }

        void Cleanup()
        {
            if (Request.IsValid()) Request->Cancel();
            Request.Reset();
            if (Owner && Owner->IsRooted()) Owner->RemoveFromRoot();
            Owner = nullptr;
        }

        FAutomationTestBase* Test = nullptr;
        FString RpcUrl;
        FString ContractHex;
        FString From;
        FString TxHash;
        UObject* Owner = nullptr;
        TUniquePtr<FAbiTypegenRpcClient> Rpc;
        TUniquePtr<FAnvilUnlockedDevelopmentSigner> Signer;
        TSharedPtr<FAbiTypegenRpcRequest, ESPMode::ThreadSafe> Request;
        FAbiTypegenRpcResponse Last;
        int32 Stage = 0;
        bool bCallback = false;
        bool bWaitingReceipt = false;
        bool bSkipped = false;
        double NextPollAt = 0.0;
        double StartedAt = 0.0;
    };

    class FBlueprintReadCommand final : public IAutomationLatentCommand
    {
    public:
        explicit FBlueprintReadCommand(FAutomationTestBase* InTest)
            : Test(InTest), RpcUrl(Env(TEXT("ATG_RPC_URL"))), ContractHex(Env(TEXT("ATG_TOKEN_ADDRESS"))), StartedAt(FPlatformTime::Seconds())
        {
            if (RpcUrl.IsEmpty() || ContractHex.IsEmpty())
            {
                bSkipped = true;
                Test->AddError(TEXT("ATG_RPC_URL/ATG_TOKEN_ADDRESS are required for Blueprint async read test."));
                return;
            }
            Probe = NewObject<UAbiTypegenBlueprintReadProbe>(GetTransientPackage()); Probe->AddToRoot();
            Owner = NewObject<UAbiTypegenTestOwner>(GetTransientPackage()); Owner->AddToRoot();

            FAbiTypegenAddress Contract; Contract.Hex = ContractHex;
            FAbiTypegenAddress ZeroOwner; ZeroOwner.Hex = TEXT("0x000000000000000000000000000000000000dEaD");

            // Invoke the BlueprintCallable factory through UObject reflection rather than a direct C++ call.
            // This exercises the same UFunction boundary used by a Blueprint async node without shipping a binary .uasset.
            UFunction* Factory = UTokenBalanceOfAsyncAction::StaticClass()->FindFunctionByName(TEXT("BalanceOf"));
            if (!Factory)
            {
                Test->AddError(TEXT("Reflected BalanceOf factory was not found"));
                bSkipped = true;
                return;
            }
            struct FFactoryParams
            {
                UObject* WorldContextObject;
                UObject* RequestOwner;
                FString RpcUrl;
                FAbiTypegenAddress ContractAddress;
                FAbiTypegenAddress OwnerAddress;
                UTokenBalanceOfAsyncAction* ReturnValue;
            } Params{nullptr, Owner, RpcUrl, Contract, ZeroOwner, nullptr};
            UTokenBalanceOfAsyncAction::StaticClass()->GetDefaultObject()->ProcessEvent(Factory, &Params);
            Action = Params.ReturnValue;
            if (!Action)
            {
                Test->AddError(TEXT("Reflected BalanceOf factory returned null"));
                bSkipped = true;
                return;
            }
            Action->AddToRoot();
            Action->OnSuccess.AddDynamic(Probe, &UAbiTypegenBlueprintReadProbe::HandleSuccess);
            Action->OnFailure.AddDynamic(Probe, &UAbiTypegenBlueprintReadProbe::HandleFailure);
            Action->Activate();
        }

        virtual ~FBlueprintReadCommand() { Cleanup(); }

        virtual bool Update() override
        {
            if (bSkipped) return true;
            if (Probe->bDone)
            {
                Test->TestTrue(TEXT("Blueprint-accessible async node succeeded"), Probe->bSucceeded);
                if (!Probe->bSucceeded) Test->AddError(Probe->Error);
                Test->TestEqual(TEXT("unused address balance is zero"), Probe->Result.Value0.Hex, U64(0).Hex);
                Cleanup();
                return true;
            }
            if (FPlatformTime::Seconds() - StartedAt > 15.0)
            {
                Test->AddError(TEXT("Blueprint async read timed out"));
                if (Action) Action->Cancel();
                Cleanup();
                return true;
            }
            return false;
        }

    private:
        void Cleanup()
        {
            if (Action && Action->IsRooted()) Action->RemoveFromRoot();
            if (Probe && Probe->IsRooted()) Probe->RemoveFromRoot();
            if (Owner && Owner->IsRooted()) Owner->RemoveFromRoot();
            Action = nullptr; Probe = nullptr; Owner = nullptr;
        }

        FAutomationTestBase* Test;
        FString RpcUrl;
        FString ContractHex;
        double StartedAt;
        bool bSkipped = false;
        UAbiTypegenBlueprintReadProbe* Probe = nullptr;
        UObject* Owner = nullptr;
        UTokenBalanceOfAsyncAction* Action = nullptr;
    };
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(
    FAbiTypegenAnvilReadWriteTest,
    "AbiTypegen.Unreal.Anvil.ReadWriteReceiptState",
    EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FAbiTypegenAnvilReadWriteTest::RunTest(const FString&)
{
    ADD_LATENT_AUTOMATION_COMMAND(FAnvilReadWriteCommand(this));
    return true;
}

IMPLEMENT_SIMPLE_AUTOMATION_TEST(
    FAbiTypegenBlueprintReadTest,
    "AbiTypegen.Unreal.Anvil.BlueprintAsyncBalanceOf",
    EAutomationTestFlags::EditorContext | EAutomationTestFlags::EngineFilter)

bool FAbiTypegenBlueprintReadTest::RunTest(const FString&)
{
    ADD_LATENT_AUTOMATION_COMMAND(FBlueprintReadCommand(this));
    return true;
}

#endif
