import Consumer
import Foundation
import Generated
import Testing
import Web3Core

@Test func valuesCompareByContent() async {
    let position = samplePosition()
    #expect(position == samplePosition())
    #expect(Set([position, samplePosition()]).count == 1)
    #expect(await sendAcross(position) == position)
}

@Test func selectorsMatchWeb3swift() throws {
    let contract = try EthereumContract(Token.abi)
    let transfer = try #require(contract.methods["transfer"]?.first)
    #expect(transfer.signature == Token.transferSignature)
    #expect(transfer.selectorEncoded == Token.transferSelector)
    let event = try #require(contract.events["Transfer"])
    #expect(event.topic == Token.TransferEventTopic)
}

@Test func tuplesAreNamedTypes() {
    let account = TupleCases.TupleAccountPosition(
        account: EthereumAddress("0x0000000000000000000000000000000000000001")!
    )
    let params = TupleCases.Deposit0Params(position: account)
    #expect(params.position == account)
}
