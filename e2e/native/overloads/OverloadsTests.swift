import BigInt
import Foundation
import Generated
import Testing
import Web3Core

@Test func aliasesEncodeOriginalSelectors() throws {
    let address = try #require(EthereumAddress("0x0000000000000000000000000000000000000001"))
    let node = Data(repeating: 2, count: 32)
    let cases: [(String, String, Data, String)] = [
        (Overloads.safeTransferFrom0Signature, "safeTransferFrom(address,address,uint256)", try Overloads.encodeSafeTransferFrom0(.init(from: address, to: address, tokenId: 7)), "42842e0e"),
        (Overloads.safeTransferFrom1Signature, "safeTransferFrom(address,address,uint256,bytes)", try Overloads.encodeSafeTransferFrom1(.init(from: address, to: address, tokenId: 7, data: Data([1]))), "b88d4fde"),
        (Overloads.attestations0Signature, "attestations(bytes32,bytes32)", try Overloads.encodeAttestations0(.init(node: node, attestationType: node)), "92abd17b"),
        (Overloads.attestations1Signature, "attestations(bytes32)", try Overloads.encodeAttestations1(.init(attestationId: node)), "940992a3"),
        (Overloads.setAddr0Signature, "setAddr(bytes32,uint256,bytes)", try Overloads.encodeSetAddr0(.init(node: node, coinType: 60, value: Data([2]))), "8b95dd71"),
        (Overloads.setAddr1Signature, "setAddr(bytes32,address)", try Overloads.encodeSetAddr1(.init(node: node, value: address)), "d5fa2b00"),
    ]
    for (signature, expectedSignature, data, selector) in cases {
        #expect(signature == expectedSignature)
        #expect(data.prefix(4).map { String(format: "%02x", $0) }.joined() == selector)
    }
}
