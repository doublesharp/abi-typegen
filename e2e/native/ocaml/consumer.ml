open Atg_sample
let rejects f = let rejected = try ignore (f ()); false with Invalid_argument _ | Failure _ -> true in assert rejected
let () =
  let maximum = Z.pred (Z.shift_left Z.one 256) in
  let payload = { atg_payload_atg_amount=maximum; atg_payload_atg_rows=[|[|Z.minus_one; Z.zero|];[||]|]; atg_payload_atg_label="hello 🌍\000"; atg_payload_atg_flags=[|true;false|] } in
  let args = { atg_tuple_0_atg_payload=payload } in
  let encoded = encode_atg_echo args in
  let returned = String.sub encoded 4 (String.length encoded - 4) in
  assert (decode_atg_echo returned = args);
  let transport ~write ~address ~data ~value = assert (not write && String.length address = 20 && data=encoded && Z.equal value Z.zero); returned in
  assert (call_atg_echo transport ~address:(String.make 20 '\001') args = args);
  rejects (fun () -> encode_atg_echo {atg_tuple_0_atg_payload={payload with atg_payload_atg_amount=Z.succ maximum}});
  rejects (fun () -> encode_atg_echo {atg_tuple_0_atg_payload={payload with atg_payload_atg_flags=[||]}});
  rejects (fun () -> call_atg_echo transport ~address:"bad" args);
  rejects (fun () -> call_atg_echo transport ~address:(String.make 20 '\001') ~value:Z.one args);
  rejects (fun () -> decode_atg_echo "bad");
  let denied=decode_atg_denied_error ("\126\070\218\182" ^ String.make 31 '\000' ^ "\007") in
  assert (Z.equal denied.atg_tuple_9_atg_code (Z.of_int 7));
  let write_transport ~write ~address:_ ~data:_ ~value = assert (write && Z.equal value Z.one); "transaction" in
  assert (call_atg_set_payload write_transport ~address:(String.make 20 '\001') ~value:Z.one args = "transaction");
  assert (encode_constructor ~bytecode:"\096\000" args = "\096\000" ^ returned);
  rejects (fun () -> encode_atg_echo {atg_tuple_0_atg_payload={payload with atg_payload_atg_amount=Z.minus_one}});
  rejects (fun () -> decode_atg_denied_error (String.make 36 '\000'));
  let hash=String.make 32 '\123' in
  let event=decode_atg_message_event ~topics:[hash] (String.make 31 '\000' ^ "\007") in
  assert (event.atg_tuple_10_atg_text=hash && Z.equal event.atg_tuple_10_atg_code (Z.of_int 7));
  rejects (fun () -> decode_atg_message_event ~topics:[] (String.make 32 '\000'));
  (* Repeated collections exercise C-root lifetimes of nested arrays. *)
  for _ = 1 to 1000 do Gc.minor (); assert (decode_atg_echo returned = args) done;
  print_endline "OCaml ABI runtime checks passed"
