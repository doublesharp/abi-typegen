open Atg_token

(* Argument arrays avoid shell parsing, including RPC URLs and generated calldata. *)
let run program arguments =
  let channel = Unix.open_process_args_in program (Array.of_list (program :: arguments)) in
  let output = Buffer.create 256 in
  (try while true do Buffer.add_string output (input_line channel); Buffer.add_char output '\n' done with End_of_file -> ());
  match Unix.close_process_in channel with
  | Unix.WEXITED 0 -> String.trim (Buffer.contents output)
  | _ -> failwith (program ^ " failed")

let hex bytes =
  "0x" ^ String.concat "" (List.init (String.length bytes) (fun i -> Printf.sprintf "%02x" (Char.code bytes.[i])))
let unhex text =
  if String.length text < 2 || String.sub text 0 2 <> "0x" || String.length text mod 2 <> 0 then failwith "invalid hex response";
  String.init ((String.length text - 2) / 2) (fun i -> Char.chr (int_of_string ("0x" ^ String.sub text (2+i*2) 2)))
let url = Sys.getenv "ATG_RPC_URL"
let contract = unhex (Sys.getenv "ATG_TOKEN_ADDRESS")
let owner = unhex (run "cast" ["wallet"; "address"; "--private-key"; Sys.getenv "ATG_PRIVATE_KEY"])
let receipt_log = ref None
let transport ~write ~address ~data ~value =
  assert (Z.equal value Z.zero);
  if write then (
    let hash = run "cast" ["send"; "--rpc-url"; url; "--chain"; Sys.getenv "ATG_CHAIN_ID";
      "--private-key"; Sys.getenv "ATG_PRIVATE_KEY"; hex address; "--data"; hex data; "--async"] in
    let receipt = run "python3" ["rpc_data.py"; "receipt"; hash] in
    (match String.split_on_char '\n' receipt with
    | data :: topics -> receipt_log := Some (List.map unhex topics, unhex data)
    | [] -> failwith "missing receipt event");
    checked_bytes 32 (unhex hash)
  ) else unhex (run "cast" ["call"; "--rpc-url"; url; hex address; "--data"; hex data])
let last_event () = match !receipt_log with Some event -> event | None -> failwith "missing receipt"
let balance account = (call_atg_balance_of transport ~address:contract {atg_tuple_7_atg_=account}).atg_tuple_2_atg_
let () =
  let large = Z.add (Z.shift_left Z.one 200) (Z.of_int 12345) in
  let before = balance owner in
  ignore (call_atg_mint transport ~address:contract {atg_tuple_11_atg_to=owner;atg_tuple_11_atg_amount=large});
  assert (Z.equal (balance owner) (Z.add before large));
  let topics, data = last_event () in
  let minted = decode_atg_transfer_event ~topics data in
  assert (minted.atg_tuple_17_atg_from=String.make 20 '\000' && minted.atg_tuple_17_atg_to=owner && Z.equal minted.atg_tuple_17_atg_amount large);
  let spender = String.make 19 '\000' ^ "\042" in
  ignore (call_atg_approve transport ~address:contract {atg_tuple_4_atg_spender=spender;atg_tuple_4_atg_amount=large});
  let allowance = call_atg_allowance transport ~address:contract {atg_tuple_0_atg_=owner;atg_tuple_0_atg_2=spender} in
  assert (Z.equal allowance.atg_tuple_2_atg_ large);
  let topics, data = last_event () in
  let approval = decode_atg_approval_event ~topics data in
  assert (approval.atg_tuple_16_atg_owner=owner && approval.atg_tuple_16_atg_spender=spender && Z.equal approval.atg_tuple_16_atg_amount large);
  let amount = Z.of_int 7 in
  let previous = balance spender in
  ignore (call_atg_transfer transport ~address:contract {atg_tuple_11_atg_to=spender;atg_tuple_11_atg_amount=amount});
  assert (Z.equal (balance spender) (Z.add previous amount));
  assert (Z.equal (balance owner) (Z.sub (Z.add before large) amount));
  let topics, data = last_event () in
  let transferred = decode_atg_transfer_event ~topics data in
  assert (transferred.atg_tuple_17_atg_from=owner && transferred.atg_tuple_17_atg_to=spender && Z.equal transferred.atg_tuple_17_atg_amount amount);
  let required = Z.shift_left Z.one 250 in
  let calldata = encode_atg_transfer {atg_tuple_11_atg_to=spender;atg_tuple_11_atg_amount=required} in
  let revert = run "python3" ["rpc_data.py"; "revert"; hex owner; hex calldata] |> unhex |> decode_atg_insufficient_balance_error in
  assert (revert.atg_tuple_15_atg_account=owner && Z.equal revert.atg_tuple_15_atg_required required && Z.equal revert.atg_tuple_15_atg_available (balance owner));
  print_endline "OCaml Anvil: exact uint256 mint/read, signed approve/transfer, successful receipts, events and custom revert passed"
