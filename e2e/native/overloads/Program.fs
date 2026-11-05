open Contracts.Overloads
open Nethereum.Web3

let contract = bind (Web3()) "0x0000000000000000000000000000000000000001"
let node = Array.create 32 2uy
let address = "0x0000000000000000000000000000000000000001"
let cases = [
    encodeSafeTransferFrom contract { From = address; To = address; TokenId = 7I }, "0x42842e0e"
    encodeSafeTransferFrom2 contract { From = address; To = address; TokenId = 7I; Data = [|1uy|] }, "0xb88d4fde"
    encodeAttestations contract { Node = node; AttestationType = node }, "0x92abd17b"
    encodeAttestations2 contract { AttestationId = node }, "0x940992a3"
    encodeSetAddr contract { Node = node; CoinType = 60I; Value = [|2uy|] }, "0x8b95dd71"
    encodeSetAddr2 contract { Node = node; Value = address }, "0xd5fa2b00"
]
for data, selector in cases do
    if not (data.StartsWith(selector, System.StringComparison.Ordinal)) then
        failwithf "Wrong overload selector: %s, expected %s" data selector
printfn "F# overload alias selectors passed"
