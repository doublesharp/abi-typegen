use abi_typegen_codegen::{ethers, ethers5, viem, wagmi, web3js, zod};
use abi_typegen_core::parser::parse_artifact;

#[test]
fn colliding_typescript_parameter_names_remain_distinct() {
    let ir = parse_artifact("Cases", r#"{"abi":[{"type":"function","name":"run","stateMutability":"nonpayable","inputs":[{"name":"class","type":"uint256"},{"name":"_class","type":"bool"}],"outputs":[] }]}"#).unwrap();
    for rendered in [
        ethers::render_ethers_file(&ir),
        ethers5::render_ethers5_file(&ir),
    ] {
        assert!(rendered.contains("_class2: boolean"), "{rendered}");
    }
    assert!(viem::render_viem_file(&ir).contains("_class2: boolean"));
    assert!(wagmi::render_wagmi_file(&ir).contains("args._class2"));
    assert!(web3js::render_web3js_file(&ir).contains("_class2: boolean"));
    assert!(zod::render_zod_file(&ir).contains("_class2: z.boolean()"));
}

#[test]
fn normalized_export_names_do_not_collide() {
    let ir = parse_artifact("Cases", r#"{"abi":[{"type":"function","name":"foo_bar","stateMutability":"nonpayable","inputs":[{"name":"amount","type":"uint256"}],"outputs":[]},{"type":"function","name":"fooBar","stateMutability":"nonpayable","inputs":[{"name":"enabled","type":"bool"}],"outputs":[] }]}"#).unwrap();
    assert!(viem::render_viem_file(&ir).contains("CasesFooBar2Params"));
    assert!(zod::render_zod_file(&ir).contains("CasesFooBar2ParamsSchema"));
}

#[test]
fn tuple_input_properties_keep_abi_names() {
    let ir = parse_artifact("Cases", r#"{"abi":[{"type":"function","name":"run","stateMutability":"nonpayable","inputs":[{"name":"item","type":"tuple","components":[{"name":"class","type":"uint256"},{"name":"_class","type":"bool"}]}],"outputs":[] }]}"#).unwrap();
    for rendered in [
        ethers::render_ethers_file(&ir),
        ethers5::render_ethers5_file(&ir),
    ] {
        assert!(
            rendered.contains("{ class: BigNumberish; _class: boolean"),
            "{rendered}"
        );
    }
    assert!(viem::render_viem_file(&ir).contains("{ class: bigint; _class: boolean"));
    assert!(zod::render_zod_file(&ir).contains("{ class: z.bigint()"));
}

#[test]
fn tuple_output_properties_keep_abi_names() {
    let ir = parse_artifact("Cases", r#"{"abi":[{"type":"function","name":"read","stateMutability":"view","inputs":[],"outputs":[{"name":"item","type":"tuple","components":[{"name":"class","type":"uint256"},{"name":"_class","type":"bool"}]}]}]}"#).unwrap();
    assert!(ethers::render_ethers_file(&ir).contains("{ class: bigint; _class: boolean }"));
    assert!(ethers5::render_ethers5_file(&ir).contains("{ class: BigNumber; _class: boolean }"));
}
