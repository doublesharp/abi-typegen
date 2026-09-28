open Atg_sample
external fail_next_copy : unit -> unit = "test_fail_next_copy"
external release_count : unit -> int = "test_release_count"
let collect () = Gc.full_major (); Gc.full_major ()
let check_failure action =
  collect ();
  let before = release_count () in
  fail_next_copy ();
  let raised = try ignore (action ()); false with Out_of_memory -> true in
  assert raised;
  collect ();
  assert (release_count () = before + 1)
let () =
  let payload = {atg_payload_atg_amount=Z.one; atg_payload_atg_rows=[||]; atg_payload_atg_label="text"; atg_payload_atg_flags=[|true;false|]} in
  let args = {atg_tuple_0_atg_payload=payload} in
  let encoded = encode_atg_echo args in
  let output = String.sub encoded 4 (String.length encoded - 4) in
  check_failure (fun () -> encode_atg_echo args);
  check_failure (fun () -> decode_atg_echo output);
  check_failure (fun () -> decode_atg_echo "invalid");
  collect ();
  let before = release_count () in
  ignore (decode_atg_echo output);
  assert (release_count () = before + 1);
  collect ();
  assert (release_count () = before + 1);
  print_endline "OCaml result ownership survives encode/decode/error-copy exceptions without double frees"
