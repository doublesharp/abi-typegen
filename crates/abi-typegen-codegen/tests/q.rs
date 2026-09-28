#[cfg(test)]
mod tests {
    use abi_typegen_codegen::q::*;
    use abi_typegen_core::types::ContractIr;
    fn contract() -> ContractIr {
        abi_typegen_core::parser::parse_artifact("Events", r#"{"abi":[{"type":"event","name":"select","anonymous":true,"inputs":[{"name":"chainId","type":"uint256","indexed":false},{"name":"x","type":"string","indexed":true},{"name":"x","type":"tuple[]","components":[{"name":"v","type":"address"}],"indexed":false}]}]}"#).expect("valid ABI")
    }
    #[test]
    fn schema_survives_without_wrappers() {
        let generated = render_q_artifacts(&contract(), false);
        assert!(generated.source.contains(".atgEvents.abi:"));
        assert!(generated.source.contains("e0selectTable:"));
        assert!(
            generated
                .source
                .contains("chainId:();contract:();blockNumber:()")
        );
        assert!(generated.source.contains("f0chainId:();f1x:();f2x:()"));
        assert!(!generated.source.contains("Decode:"));
        assert!(generated.bridge.is_empty());
    }
    #[test]
    fn decoder_uses_explicit_signature_and_shared_bridge() {
        let generated = render_q_artifacts(&contract(), true);
        assert!(
            generated
                .source
                .contains("select(uint256,string,(address)[])")
        );
        assert!(generated.source.contains("e0selectDecode:"));
        assert!(generated.source.contains("atg_q_decode"));
        assert!(generated.bridge.contains("atg_decode_event("));
        assert!(generated.source.contains("e0selectAnonymous:1b"));
    }
    #[test]
    fn nested_types_indexed_hashes_and_overloads_keep_distinct_shapes() {
        let ir = abi_typegen_core::parser::parse_artifact(
            "Events",
            r#"{"abi":[{"type":"event","name":"select","anonymous":true,"inputs":[{"name":"chainId","type":"uint256","indexed":false},{"name":"address","type":"address","indexed":false},{"name":"flags","type":"bool[2]","indexed":false},{"name":"tuple","type":"tuple","indexed":false,"components":[{"name":"n","type":"uint256"},{"name":"a","type":"address"}]},{"name":"hash","type":"string","indexed":true}]},{"type":"event","name":"select","anonymous":true,"inputs":[]}]}"#,
        )
        .expect("valid event fixture");
        let out = render_q_artifacts(&ir, true).source;
        assert!(out.contains("e0selectDecode:"));
        assert!(out.contains("e1selectDecode:"));
        assert!(out.contains("12_(v[1])"));
        assert!(out.contains("{x} each (v[2])"));
        assert!(out.contains("12_((v[3])[1])"));
        // Indexed strings stay byte-vector hashes rather than UTF-8 text.
        assert!(!out.contains("\"c\"$(v[4])"));
        assert!(out.contains("f0chainId"));
        assert!(out.contains("e1selectTable:([]chainId"));
    }
    #[test]
    fn contract_and_field_identifiers_do_not_inject_q_syntax() {
        let mut ir = contract();
        ir.name = "9_bad-name".into();
        ir.events[0].name = "select;bad".into();
        ir.events[0].inputs[0].name = "x`bad".into();
        let out = render_q_artifacts(&ir, false).source;
        assert!(out.contains(".atg9badname.e0selectbadTable:"));
        assert!(out.contains("f0xbad:()"));
        assert_eq!(namespace_name("a_b"), namespace_name("ab"));
    }
}
