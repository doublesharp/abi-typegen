//! Reflected ABI DTOs and codecs. Nested arrays use named USTRUCTs for UHT.
use super::{contract_name, cpp_string, reflected_names, unreal_function_stems};
use abi_typegen_core::types::{ContractIr, SolType};

#[derive(Default)]
/// Generated primary DTOs and optional adapters.
pub(super) struct Reflected {
    /// Primary reflected ABI value declarations.
    pub types: String,
    /// Blueprint codec methods, gated by wrappers.
    pub declarations: String,
    /// Owned-value conversion and codec definitions.
    pub definitions: String,
    /// All reflected names for host-module collision checks.
    pub symbols: Vec<String>,
}

/// Reflected C++ type using named wrappers for nested containers.
pub(super) fn type_name(ty: &SolType, path: &str) -> String {
    match ty {
        SolType::Bool => "bool".into(),
        SolType::Address => "FAbiTypegenAddress".into(),
        SolType::Uint(_) | SolType::Int(_) => "FAbiTypegenWord256".into(),
        SolType::StringType => "FString".into(),
        SolType::Bytes | SolType::BytesN(_) => "FAbiTypegenBytes".into(),
        SolType::Tuple(_) => path.into(),
        SolType::Array(inner) | SolType::FixedArray(inner, _) => {
            let child = format!("{path}_Item");
            if matches!(
                inner.as_ref(),
                SolType::Array(_) | SolType::FixedArray(_, _)
            ) {
                format!("TArray<{child}>")
            } else {
                format!("TArray<{}>", type_name(inner, &child))
            }
        }
    }
}

fn fields<'a>(
    params: impl Iterator<Item = (&'a str, &'a SolType)>,
    record: &str,
) -> Vec<(String, SolType)> {
    let params = params.collect::<Vec<_>>();
    let names = reflected_names(
        params.iter().map(|(n, _)| *n),
        &["StaticStruct", "bCompleted", record],
    );
    params
        .into_iter()
        .zip(names)
        .map(|((_, t), n)| (n, t.clone()))
        .collect()
}

/// Render DTOs always; callable adapters only when requested.
pub(super) fn render(ir: &ContractIr, api: &str, wrappers: bool) -> Reflected {
    let mut out = Reflected::default();

    let contract = contract_name(&ir.name);
    let class = format!("U{contract}UnrealMetadata");
    let abi = super::namespace_name(&ir.name);
    for (function, stem) in ir.functions.iter().zip(unreal_function_stems(ir)) {
        // Factory parameters and codec parameters share the same names.
        let input = super::read_input_names(function, &contract, &stem);
        let inputs = function
            .inputs
            .iter()
            .zip(input)
            .map(|(p, n)| (n, p.ty.clone()))
            .collect::<Vec<_>>();
        let result = format!("F{contract}{stem}Result");
        let outputs = fields(
            function.outputs.iter().map(|p| (p.name.as_str(), &p.ty)),
            &result,
        );
        for (name, ty) in &inputs {
            emit_type(
                &mut out,
                ty,
                &format!("F{contract}{stem}_Input_{name}"),
                api,
                false,
            );
        }
        emit_record(
            &mut out,
            &result,
            &format!("F{contract}{stem}_Output_"),
            &outputs,
            api,
        );
        let tail = inputs
            .iter()
            .map(|(n, t)| {
                format!(
                    ", const {}& {n}",
                    type_name(t, &format!("F{contract}{stem}_Input_{n}"))
                )
            })
            .collect::<String>();
        let encode = format!("Encode{stem}");
        out.declarations.push_str(&format!("    /** Encode calldata without signing or submitting it. */\n    UFUNCTION(BlueprintCallable, Category=\"abi-typegen|{contract}\")\n    static bool {encode}(FAbiTypegenBytes& Encoded, FString& Error{tail});\n"));
        out.definitions.push_str(&format!("\nbool {class}::{encode}(FAbiTypegenBytes& Encoded, FString& Error{tail})\n{{\n    Encoded.Data.Reset(); Error.Reset();\n    FAbiTypegenValue Args(atg_value_seq());\n    if (!Args.Get()) {{ Error = TEXT(\"ABI allocation failure\"); return false; }}\n"));
        for (n, _) in &inputs {
            out.definitions.push_str(&format!("    {{ FAbiTypegenValue Item(BuildF{contract}{stem}_Input_{n}({n}, Error)); if (!Item.Get() || atg_value_push(Args.Get(), Item.Get())) {{ if (Error.IsEmpty()) Error = TEXT(\"ABI allocation failure\"); return false; }} }}\n"));
        }
        out.definitions.push_str(&format!("    FAbiTypegenResult Native(atg_encode({abi}_abi, {}, Args.Get()));\n    if (!Native.IsOk()) {{ Error = Native.Error(); return false; }}\n    Encoded.Data = Native.CopyBytes(); return true;\n}}\n", cpp_string(&function.signature())));
        emit_decode(
            &mut out,
            &class,
            &contract,
            &format!("Decode{stem}"),
            &result,
            &format!("F{contract}{stem}_Output_"),
            &outputs,
            &format!(
                "atg_decode({abi}_abi, {}, Data.Data.GetData(), Data.Data.Num())",
                cpp_string(&function.signature())
            ),
            false,
        );
    }
    // Event/error overloads also get collision-free, case-insensitive method stems.
    let mut used = unreal_function_stems(ir)
        .into_iter()
        .map(|s| s.to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();
    for event in &ir.events {
        let stem = claim(&mut used, &format!("{}Event", contract_name(&event.name)));
        let result = format!("F{contract}{stem}");
        let outputs = fields(
            event.inputs.iter().map(|p| (p.name.as_str(), &p.ty)),
            &result,
        )
        .into_iter()
        .zip(&event.inputs)
        .map(|((n, t), p)| {
            (
                n,
                if p.indexed
                    && matches!(
                        t,
                        SolType::StringType
                            | SolType::Bytes
                            | SolType::Array(_)
                            | SolType::FixedArray(_, _)
                            | SolType::Tuple(_)
                    )
                {
                    SolType::BytesN(32)
                } else {
                    t
                },
            )
        })
        .collect::<Vec<_>>();
        emit_record(&mut out, &result, &format!("{result}_"), &outputs, api);
        emit_decode(
            &mut out,
            &class,
            &contract,
            &format!("Decode{stem}"),
            &result,
            &format!("{result}_"),
            &outputs,
            &format!(
                "atg_decode_event({abi}_abi, {}, TopicBytes.GetData(), Topics.Num(), Data.Data.GetData(), Data.Data.Num())",
                cpp_string(&event.signature())
            ),
            true,
        );
        if wrappers {
            super::events::emit(
                &mut out,
                &contract,
                &stem,
                &result,
                api,
                &event.topic0().to_string(),
                event.anonymous,
            );
        }
    }
    for error in &ir.errors {
        let stem = claim(&mut used, &format!("{}Error", contract_name(&error.name)));
        let result = format!("F{contract}{stem}");
        let outputs = fields(
            error.inputs.iter().map(|p| (p.name.as_str(), &p.ty)),
            &result,
        );
        emit_record(&mut out, &result, &format!("{result}_"), &outputs, api);
        emit_decode(
            &mut out,
            &class,
            &contract,
            &format!("Decode{stem}"),
            &result,
            &format!("{result}_"),
            &outputs,
            &format!(
                "atg_decode_error({abi}_abi, {}, Data.Data.GetData(), Data.Data.Num())",
                cpp_string(&error.signature())
            ),
            false,
        );
    }
    if !wrappers {
        out.declarations.clear();
        out.definitions.clear();
    }
    out
}

fn claim(used: &mut std::collections::HashSet<String>, base: &str) -> String {
    let mut n = base.to_owned();
    let mut i = 2;
    while !used.insert(n.to_ascii_lowercase()) {
        n = format!("{base}{i}");
        i += 1;
    }
    n
}

fn emit_record(
    out: &mut Reflected,
    name: &str,
    prefix: &str,
    fields: &[(String, SolType)],
    api: &str,
) {
    for (n, t) in fields {
        emit_type(out, t, &format!("{prefix}{n}"), api, false);
    }
    out.symbols.push(name.into());
    out.types.push_str(&format!(
        "\nUSTRUCT(BlueprintType)\nstruct {api} {name}\n{{\n    GENERATED_BODY()\n"
    ));
    for (n, t) in fields {
        out.types.push_str(&format!("    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category=\"abi-typegen\")\n    {} {n}{{}};\n", type_name(t, &format!("{prefix}{n}"))));
    }
    if fields.is_empty() {
        out.types.push_str("    UPROPERTY(BlueprintReadOnly, Category=\"abi-typegen\")\n    bool bCompleted = true;\n");
    }
    out.types.push_str("};\n");
}

fn emit_type(out: &mut Reflected, ty: &SolType, path: &str, api: &str, wrap: bool) {
    let mut cpp = type_name(ty, path);
    match ty {
        SolType::Tuple(parts) => {
            let children = fields(parts.iter().map(|p| (p.name.as_str(), &p.ty)), path);
            emit_record(out, path, &format!("{path}_"), &children, api);
        }
        SolType::Array(inner) | SolType::FixedArray(inner, _) => {
            let child = format!("{path}_Item");
            emit_type(
                out,
                inner,
                &child,
                api,
                matches!(
                    inner.as_ref(),
                    SolType::Array(_) | SolType::FixedArray(_, _)
                ),
            );
            if wrap {
                out.symbols.push(path.into());
                out.types.push_str(&format!("\n/** UHT wrapper for a nested ABI array. */\nUSTRUCT(BlueprintType)\nstruct {api} {path}\n{{\n    GENERATED_BODY()\n    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category=\"abi-typegen\")\n    {cpp} Values;\n}};\n"));
                cpp = path.into();
            }
        }
        _ => {}
    }
    let access = if wrap { "X.Values" } else { "X" };
    let mut build = String::new();
    let mut read = String::new();
    match ty {
        SolType::Bool => {
            build.push_str("    return atg_value_bool(X);\n");
            read.push_str("    X = atg_value_get_bool(V) != 0; return true;\n");
        }
        SolType::Uint(_) | SolType::Int(_) => {
            build.push_str("    atg_word Word{}; if (!X.TryToWord(Word, Error)) return nullptr; return atg_value_word(Word.bytes);\n");
            read.push_str("    atg_word Word{}; FMemory::Memcpy(Word.bytes, atg_value_data(V), 32); X = FAbiTypegenWord256::FromWord(Word); return true;\n");
        }
        SolType::Address => {
            build.push_str("    atg_address Address{}; if (!X.TryToAddress(Address, Error)) return nullptr; uint8 Word[32]{}; FMemory::Memcpy(Word + 12, Address.bytes, 20); return atg_value_word(Word);\n");
            read.push_str("    atg_address Address{}; FMemory::Memcpy(Address.bytes, atg_value_data(V) + 12, 20); X = FAbiTypegenAddress::FromAddress(Address); return true;\n");
        }
        SolType::StringType => {
            build.push_str("    FTCHARToUTF8 Utf8(*X); return atg_value_bytes(reinterpret_cast<const uint8*>(Utf8.Get()), Utf8.Length());\n");
            read.push_str("    if (atg_value_len(V) > MAX_int32) { Error = TEXT(\"decoded string exceeds Unreal limits\"); return false; } FUTF8ToTCHAR Utf8(reinterpret_cast<const ANSICHAR*>(atg_value_data(V)), static_cast<int32>(atg_value_len(V))); X = FString(Utf8.Length(), Utf8.Get()); return true;\n");
        }
        SolType::Bytes | SolType::BytesN(_) => {
            if let SolType::BytesN(n) = ty {
                build.push_str(&format!("    if (X.Data.Num() != {n}) {{ Error = TEXT(\"bytes{n} requires exactly {n} bytes\"); return nullptr; }}\n"));
            }
            build.push_str("    return atg_value_bytes(X.Data.GetData(), X.Data.Num());\n");
            read.push_str("    if (atg_value_len(V) > MAX_int32) { Error = TEXT(\"decoded bytes exceed Unreal limits\"); return false; } X.Data.Reset(); X.Data.Append(atg_value_data(V), static_cast<int32>(atg_value_len(V))); return true;\n");
        }
        SolType::Array(_) | SolType::FixedArray(_, _) => {
            if let SolType::FixedArray(_, n) = ty {
                build.push_str(&format!("    if ({access}.Num() != {n}) {{ Error = TEXT(\"fixed array requires {n} elements\"); return nullptr; }}\n"));
            }
            build.push_str(&format!("    FAbiTypegenValue Seq(atg_value_seq()); if (!Seq.Get()) return nullptr;\n    for (const auto& Item : {access}) {{ FAbiTypegenValue Child(Build{path}_Item(Item, Error)); if (!Child.Get() || atg_value_push(Seq.Get(), Child.Get())) return nullptr; }} return Seq.Release();\n"));
            read.push_str(&format!("    if (atg_value_len(V) > MAX_int32) {{ Error = TEXT(\"decoded array exceeds Unreal limits\"); return false; }} {access}.SetNum(static_cast<int32>(atg_value_len(V)));\n    for (int32 I = 0; I < {access}.Num(); ++I) if (!Read{path}_Item(atg_value_at(V, I), {access}[I], Error)) return false; return true;\n"));
        }
        SolType::Tuple(parts) => {
            build.push_str(
                "    FAbiTypegenValue Seq(atg_value_seq()); if (!Seq.Get()) return nullptr;\n",
            );
            for (i, (n, _)) in fields(parts.iter().map(|p| (p.name.as_str(), &p.ty)), path)
                .iter()
                .enumerate()
            {
                build.push_str(&format!("    {{ FAbiTypegenValue Child(Build{path}_{n}(X.{n}, Error)); if (!Child.Get() || atg_value_push(Seq.Get(), Child.Get())) return nullptr; }}\n"));
                read.push_str(&format!(
                    "    if (!Read{path}_{n}(atg_value_at(V, {i}), X.{n}, Error)) return false;\n"
                ));
            }
            build.push_str("    return Seq.Release();\n");
            read.push_str("    return true;\n");
        }
    }
    // Runtime decode validates the ABI shape before these copying helpers run.
    out.definitions.push_str(&format!("\n[[maybe_unused]] static atg_value* Build{path}(const {cpp}& X, FString& Error)\n{{\n    (void)X; (void)Error;\n{build}}}\n[[maybe_unused]] static bool Read{path}(const atg_value* V, {cpp}& X, FString& Error)\n{{\n    (void)X; (void)Error;\n{read}}}\n"));
}

#[allow(clippy::too_many_arguments)]
fn emit_decode(
    out: &mut Reflected,
    class: &str,
    contract: &str,
    method: &str,
    result: &str,
    prefix: &str,
    fields: &[(String, SolType)],
    native: &str,
    event: bool,
) {
    let topics = if event {
        ", const TArray<FAbiTypegenWord256>& Topics"
    } else {
        ""
    };
    out.declarations.push_str(&format!("    /** Decode ABI bytes into owned reflected values. Indexed dynamic fields contain the topic hash. */\n    UFUNCTION(BlueprintCallable, Category=\"abi-typegen|{contract}\")\n    static bool {method}(const FAbiTypegenBytes& Data, {result}& Result, FString& Error{topics});\n"));
    out.definitions.push_str(&format!("\nbool {class}::{method}(const FAbiTypegenBytes& Data, {result}& Result, FString& Error{topics})\n{{\n    Error.Reset();\n"));
    if event {
        out.definitions.push_str("    TArray<uint8> TopicBytes; for (const auto& Topic : Topics) { atg_word Word{}; if (!Topic.TryToWord(Word, Error)) return false; TopicBytes.Append(Word.bytes, 32); }\n");
    }
    out.definitions.push_str(&format!("    FAbiTypegenResult Native({native});\n    if (!Native.IsOk()) {{ Error = Native.Error(); return false; }}\n    {result} Decoded{{}};\n"));
    for (i, (n, _)) in fields.iter().enumerate() {
        out.definitions.push_str(&format!("    if (!Read{prefix}{n}(atg_value_at(Native.Value(), {i}), Decoded.{n}, Error)) return false;\n"));
    }
    out.definitions
        .push_str("    Result = MoveTemp(Decoded); return true;\n}\n");
}
