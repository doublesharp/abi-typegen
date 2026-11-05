open Atg_overloads
let () =
  let address = String.make 20 '\001' in
  let node = String.make 32 '\002' in
  let cases = [
    atg_safe_transfer_from_signature, "safeTransferFrom(address,address,uint256)", encode_atg_safe_transfer_from { atg_tuple_0_atg_from=address; atg_tuple_0_atg_to=address; atg_tuple_0_atg_token_id=Z.of_int 7 }, "42842e0e";
    atg_safe_transfer_from2_signature, "safeTransferFrom(address,address,uint256,bytes)", encode_atg_safe_transfer_from2 { atg_tuple_4_atg_from=address; atg_tuple_4_atg_to=address; atg_tuple_4_atg_token_id=Z.of_int 7; atg_tuple_4_atg_data="\001" }, "b88d4fde";
    atg_attestations_signature, "attestations(bytes32,bytes32)", encode_atg_attestations { atg_tuple_6_atg_node=node; atg_tuple_6_atg_attestation_type=node }, "92abd17b";
    atg_attestations2_signature, "attestations(bytes32)", encode_atg_attestations2 { atg_tuple_10_atg_attestation_id=node }, "940992a3";
    atg_set_addr_signature, "setAddr(bytes32,uint256,bytes)", encode_atg_set_addr { atg_tuple_15_atg_node=node; atg_tuple_15_atg_coin_type=Z.of_int 60; atg_tuple_15_atg_value="\002" }, "8b95dd71";
    atg_set_addr2_signature, "setAddr(bytes32,address)", encode_atg_set_addr2 { atg_tuple_16_atg_node=node; atg_tuple_16_atg_value=address }, "d5fa2b00"
  ] in
  List.iter (fun (signature, expected, data, selector) ->
    assert (signature = expected);
    let actual = String.concat "" (List.init 4 (fun i -> Printf.sprintf "%02x" (Char.code data.[i]))) in
    assert (actual = selector)) cases;
  print_endline "OCaml overload alias selectors passed"
