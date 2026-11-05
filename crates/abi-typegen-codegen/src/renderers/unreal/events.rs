//! Typed Blueprint event watchers over the shared canonical HTTP poller.
use super::cpp_string;
use super::reflected::Reflected;

#[allow(clippy::too_many_arguments)]
/// Emit one typed event action whose lifetime owns its polling session.
pub(super) fn emit(
    out: &mut Reflected,
    contract: &str,
    stem: &str,
    result: &str,
    api: &str,
    topic: &str,
    anonymous: bool,
) {
    let action = format!("U{contract}{stem}Watcher");
    let delegate = format!("F{contract}{stem}Received");
    let failure = format!("F{contract}{stem}WatchFailure");
    out.symbols
        .extend([action.clone(), delegate.clone(), failure.clone()]);
    let display = cpp_string(&format!("Watch {contract} {stem}"));
    out.types.push_str(&format!(r#"
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam({delegate}, {result}, Event);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam({failure}, FString, Error);
/** Watches confirmed logs from an explicit cursor. Cancel to stop; reorgs report failure. */
UCLASS()
class {api} {action} : public UAbiTypegenAsyncActionBase
{{
    GENERATED_BODY()
public:
    UPROPERTY(BlueprintAssignable) {delegate} OnEvent;
    UPROPERTY(BlueprintAssignable) {failure} OnFailure;
    UFUNCTION(BlueprintCallable, meta=(BlueprintInternalUseOnly="true", WorldContext="WorldContextObject", DisplayName={display}), Category="abi-typegen|{contract}")
    static {action}* Watch(UObject* WorldContextObject, UObject* RequestOwner, const FString& RpcUrl, FAbiTypegenAddress ContractAddress, int64 StartBlock, int32 Confirmations = 1, int32 BlocksPerRange = 100, float PollIntervalSeconds = 1.0f, float TimeoutSeconds = 120.0f);
    virtual void Activate() override;
    virtual void Cancel() override;
    virtual void BeginDestroy() override;
private:
    FString RpcUrlValue;
    FAbiTypegenAddress ContractAddressValue;
    int64 StartBlockValue = 0;
    FAbiTypegenPollOptions Options;
    TSharedPtr<FAbiTypegenPollSession> Session;
}};
"#));
    let topics = if anonymous {
        String::new()
    } else {
        format!("    Topics.Add(TEXT({}));\n", cpp_string(topic))
    };
    out.definitions.push_str(&format!(r#"
{action}* {action}::Watch(UObject* WorldContextObject, UObject* RequestOwner, const FString& RpcUrl, FAbiTypegenAddress ContractAddress, int64 StartBlock, int32 Confirmations, int32 BlocksPerRange, float PollIntervalSeconds, float TimeoutSeconds)
{{
    auto* Action = NewObject<{action}>(); Action->InitializeLifetime(WorldContextObject, RequestOwner);
    Action->RpcUrlValue = RpcUrl; Action->ContractAddressValue = ContractAddress; Action->StartBlockValue = StartBlock;
    Action->Options.Confirmations = Confirmations > 0 ? static_cast<uint64>(Confirmations) : 0;
    Action->Options.BlocksPerRange = BlocksPerRange > 0 ? static_cast<uint64>(BlocksPerRange) : 0;
    Action->Options.IntervalSeconds = PollIntervalSeconds; Action->Options.TimeoutSeconds = TimeoutSeconds; return Action;
}}
void {action}::Activate()
{{
    if (StartBlockValue < 0) {{ OnFailure.Broadcast(TEXT("start block must be nonnegative")); Finish(); return; }}
    FString Error; atg_address Address{{}};
    if (!ContractAddressValue.TryToAddress(Address, Error)) {{ OnFailure.Broadcast(Error); Finish(); return; }}
    TArray<FString> Topics;
{topics}    auto RpcTransport = FAbiTypegenPollSession::HttpTransport(RpcUrlValue, this);
    FAbiTypegenPollSession::FTransport Transport = [this, RpcTransport = MoveTemp(RpcTransport)](const FString& Method, const TArray<TSharedPtr<FJsonValue>>& Params, FAbiTypegenRpcClient::FCompletion Completion) mutable
    {{
        if (!IsRequestOwnerAlive())
        {{
            auto Request = MakeShared<FAbiTypegenRpcRequest, ESPMode::ThreadSafe>(); FAbiTypegenRpcResponse Response;
            Response.Error = TEXT("request owner destroyed"); Completion(MoveTemp(Response)); return Request;
        }}
        return RpcTransport(Method, Params, MoveTemp(Completion));
    }};
    Session = FAbiTypegenPollSession::WatchLogs(MoveTemp(Transport), AbiTypegen::AddressToHex(Address), Topics, static_cast<uint64>(StartBlockValue), Options,
        [this](const TArray<FAbiTypegenLog>& Logs, FString& DecodeError)
        {{
            if (!IsRequestOwnerAlive()) {{ Cancel(); return false; }}
            TArray<{result}> Events;
            for (const auto& Log : Logs)
            {{
                {result} Event;
                if (!U{contract}UnrealMetadata::Decode{stem}(Log.Data, Event, DecodeError, Log.Topics)) return false;
                Events.Add(MoveTemp(Event));
            }}
            for (const auto& Event : Events) {{ if (!Session || Session->IsFinished() || !IsRequestOwnerAlive()) {{ Cancel(); return false; }} OnEvent.Broadcast(Event); }}
            return true;
        }},
        [this](const FString& Failure) {{ if (IsRequestOwnerAlive()) OnFailure.Broadcast(Failure); Finish(); }});
}}
void {action}::Cancel() {{ if (Session) Session->Cancel(); Session.Reset(); Super::Cancel(); }}
void {action}::BeginDestroy() {{ if (Session) Session->Cancel(); Session.Reset(); Super::BeginDestroy(); }}
"#));
}
