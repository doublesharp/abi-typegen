type abi_value = Bool of bool | Word of string | Bytes of string | Seq of abi_value array
let malformed () = invalid_arg "malformed ABI value"
let checked_bytes n s = if String.length s <> n then invalid_arg "invalid byte length" else s
let word signed bits z =
  let bound = Z.shift_left Z.one (if signed then bits - 1 else bits) in
  let lower = if signed then Z.neg bound else Z.zero in
  if Z.compare z lower < 0 || Z.compare z bound >= 0 then invalid_arg "integer outside ABI range";
  let z = if Z.sign z < 0 then Z.add z (Z.shift_left Z.one 256) else z in
  Word (String.init 32 (fun i -> Char.chr (Z.to_int (Z.extract z ((31-i)*8) 8))))
let integer signed = function
 | Word s ->
   let s = checked_bytes 32 s in
   let z = ref Z.zero in
   String.iter (fun c -> z := Z.add (Z.shift_left !z 8) (Z.of_int (Char.code c))) s;
   if signed && Char.code s.[0] >= 128 then Z.sub !z (Z.shift_left Z.one 256) else !z
 | _ -> malformed ()
let sequence = function Seq xs -> xs | _ -> malformed ()
let bytes = function Bytes s -> s | _ -> malformed ()
let boolean = function Bool b -> b | _ -> malformed ()
let address = function Word s -> String.sub (checked_bytes 32 s) 12 20 | _ -> malformed ()
let fixed n xs = if Array.length xs <> n then invalid_arg "invalid array length" else xs
(** Transport receives write flag, destination (20 raw bytes), calldata, and optional value. *)
type transport = write:bool -> address:string -> data:string -> value:Z.t -> string
