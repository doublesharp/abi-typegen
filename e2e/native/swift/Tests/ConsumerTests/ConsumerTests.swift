import Consumer
import BigInt
import Foundation
import Generated
import Testing
import Web3Core
import web3swift

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
    #expect(event.topic == Token.transferEventTopic)
}

@Test func tuplesAreNamedTypes() {
    let account = TupleCases.TupleAccountPosition(
        account: EthereumAddress("0x0000000000000000000000000000000000000001")!
    )
    let params = TupleCases.Deposit0Params(position: account)
    #expect(params.position == account)
}

@Test func offlineCallEncodingAndTypedResults() throws {
    let recipient = try #require(EthereumAddress("0x0000000000000000000000000000000000000003"))
    let calldata = try Token.encodeTransfer(.init(to: recipient, amount: BigUInt(42)))
    let contract = try EthereumContract(Token.abi)
    #expect(calldata == contract.method(Token.transferSignature, parameters: [recipient, BigUInt(42)], extraData: nil))
    #expect(calldata.prefix(4) == Token.transferSelector)
    let result = try Token.decodeBalanceOfResult(Data(repeating: 0, count: 31) + Data([42]))
    #expect(result.uint256 == BigUInt(42))
    #expect(throws: Error.self) { try Token.decodeBalanceOfResult(Data([1])) }
    #expect(throws: Error.self) { try Token.decodeBalanceOfResult(Data(repeating: 0, count: 32) + Data([1])) }
    #expect(throws: Error.self) { try Token.encodeTransfer(.init(to: recipient, amount: BigUInt(1) << 256)) }
}

@Test func overloadedAndTupleCallsRoundTripThroughSdk() throws {
    let address = try #require(EthereumAddress("0x0000000000000000000000000000000000000004"))
    let first = try Vault.encodeDeposit0(.init(amount: BigUInt(7), recipient: address))
    let second = try Vault.encodeDeposit1(.init(amount: BigUInt(7)))
    #expect(first.prefix(4) == Vault.deposit0Selector)
    #expect(second.prefix(4) == Vault.deposit1Selector)
    let position = Vault.Position(shares: 7, depositedAt: 9, token: address)
    let encoded = ABIEncoder.encode(types: [
        .tuple(types: [.uint(bits: 256), .uint(bits: 64), .address]),
    ], values: [[BigUInt(7), BigUInt(9), address]])
    let decoded = try Vault.decodeGetPositionResult(try #require(encoded))
    #expect(decoded.position == position)
}

@Test func eventFiltersAndCustomErrorsDecode() throws {
    let owner = try #require(EthereumAddress("0x0000000000000000000000000000000000000005"))
    let topics = try Token.filterTransfer(filter0: owner)
    #expect(topics.count == 3)
    let errorData = Token.invalidRecipientErrorSelector
    guard case .invalidRecipient = try Token.decodeCustomError(errorData) else {
        Issue.record("expected InvalidRecipient")
        return
    }
    if case nil = try Token.decodeCustomError(Data([0, 0, 0, 0])) {} else {
        Issue.record("unknown error selector decoded")
    }

    let recipient = try #require(EthereumAddress("0x0000000000000000000000000000000000000006"))
    func hex(_ data: Data) -> String { "0x" + data.map { String(format: "%02x", $0) }.joined() }
    let topicAddress = { (value: EthereumAddress) in
        "0x" + String(repeating: "0", count: 24) + String(value.address.dropFirst(2))
    }
    let logJSON: [String: Any] = [
        "address": owner.address,
        "blockHash": hex(Data(repeating: 0, count: 32)),
        "blockNumber": "0x1",
        "data": hex(Data(repeating: 0, count: 31) + Data([42])),
        "logIndex": "0x0",
        "removed": "0x0",
        "topics": [hex(Token.transferEventTopic), topicAddress(owner), topicAddress(recipient)],
        "transactionHash": hex(Data(repeating: 0, count: 32)),
        "transactionIndex": "0x0",
    ]
    let log = try JSONDecoder().decode(EventLog.self, from: JSONSerialization.data(withJSONObject: logJSON))
    let transfer = try Token.decodeTransferEvent(log)
    #expect(transfer.from == owner)
    #expect(transfer.to == recipient)
    #expect(transfer.amount == 42)
}

@Test func nestedArraysAndBytesRespectAbiShapes() throws {
    let matrix = [[BigUInt(1), BigUInt(2)], [BigUInt(3)]]
    let encoded = try #require(ABIEncoder.encode(types: [.array(type: .array(type: .uint(bits: 256), length: 0), length: 0)], values: [matrix]))
    #expect(try EdgeCases.decodeNestedArrayResult(encoded).uint256ArrayArray == matrix)
    #expect(throws: Error.self) {
        try TupleCases.encodeSetRows(.init(rows: [[true], [false, true]]))
    }
    #expect(throws: Error.self) {
        try Exchange.encodeCancelOrder(.init(orderHash: Data(repeating: 1, count: 31)))
    }
    let a = Data([0x12])
    let b = Data(repeating: 0x34, count: 16)
    let c = Data(repeating: 0x56, count: 32)
    let bytes = try #require(ABIEncoder.encode(types: [.bytes(length: 1), .bytes(length: 16), .bytes(length: 32)], values: [a, b, c]))
    let decoded = try EdgeCases.decodeFixedBytesResult(bytes)
    #expect(decoded.a == a && decoded.b == b && decoded.c == c)
}

@Test func payableWriteCarriesTransactionOptions() throws {
    let address = try #require(EthereumAddress("0x0000000000000000000000000000000000000007"))
    let sender = try #require(EthereumAddress("0x0000000000000000000000000000000000000008"))
    let provider = Web3HttpProvider(url: URL(string: "http://127.0.0.1:1")!, network: .Mainnet)
    let client = try Vault.Client(web3: Web3(provider: provider), at: address)
    var options = CodableTransaction(to: sender, data: Data())
    options.from = sender
    options.gasLimit = 123_456
    let prepared = try client.prepareDeposit1(.init(amount: 7), transaction: options, value: 11)
    #expect(prepared.transaction.to == address)
    #expect(prepared.transaction.from == sender)
    #expect(prepared.transaction.gasLimit == 123_456)
    #expect(prepared.transaction.value == 11)
    #expect(prepared.transaction.data.prefix(4) == Vault.deposit1Selector)
    var nonpayableOptions = options
    nonpayableOptions.value = 1
    let tokenClient = try Token.Client(web3: Web3(provider: provider), at: address)
    #expect(throws: Error.self) {
        try tokenClient.prepareTransfer(.init(to: sender, amount: 1), transaction: nonpayableOptions)
    }
}

@Test func indexedReferenceTopicsRemainHashes() throws {
    let label = Data(repeating: 0x11, count: 32)
    let payload = Data(repeating: 0x22, count: 32)
    let values = Data(repeating: 0x33, count: 32)
    let topics = try NativeCases.filterIndexedReferences(filter0: label, filter1: payload, filter2: values)
    #expect(topics.count == 4)
    #expect(throws: Error.self) { try NativeCases.filterIndexedReferences(filter0: Data([1])) }
    func hex(_ data: Data) -> String { "0x" + data.map { String(format: "%02x", $0) }.joined() }
    let address = try #require(EthereumAddress("0x0000000000000000000000000000000000000009"))
    let logJSON: [String: Any] = [
        "address": address.address,
        "blockHash": hex(Data(repeating: 0, count: 32)),
        "blockNumber": "0x1",
        "data": "0x",
        "logIndex": "0x0",
        "removed": "0x0",
        "topics": [hex(NativeCases.indexedReferencesEventTopic), hex(label), hex(payload), hex(values)],
        "transactionHash": hex(Data(repeating: 0, count: 32)),
        "transactionIndex": "0x0",
    ]
    let log = try JSONDecoder().decode(EventLog.self, from: JSONSerialization.data(withJSONObject: logJSON))
    let decoded = try NativeCases.decodeIndexedReferencesEvent(log)
    #expect(decoded.label == label && decoded.payload == payload && decoded.values == values)
}

@Test func signedRpcWriteReadAndEvent() async throws {
    let environment = ProcessInfo.processInfo.environment
    guard let rpc = environment["ATG_RPC_URL"],
          let addressText = environment["ATG_TOKEN_ADDRESS"],
          let privateKey = environment["ATG_PRIVATE_KEY"],
          let chainText = environment["ATG_CHAIN_ID"] else { return }
    let url = try #require(URL(string: rpc))
    let address = try #require(EthereumAddress(addressText))
    let chainID = try #require(BigUInt(chainText))
    let wallet = try #require(PlainKeystore(privateKey: privateKey))
    let sender = try #require(wallet.addresses?.first)
    let provider = try await Web3HttpProvider(url: url, network: .Custom(networkID: chainID), keystoreManager: KeystoreManager([wallet]))
    let web3 = Web3(provider: provider)
    let client = try Token.Client(web3: web3, at: address)
    var options = CodableTransaction(to: address, data: Data())
    options.from = sender
    let operation = try client.prepareMint(.init(to: sender, amount: 42), transaction: options)
    let fees = Policies(gasLimitPolicy: .manual(200_000), gasPricePolicy: .manual(2_000_000_000), maxFeePerGasPolicy: .manual(2_000_000_000), maxPriorityFeePerGasPolicy: .manual(1_000_000_000))
    let submission = try await operation.writeToChain(password: "", policies: fees, sendRaw: true)
    let hash = try #require(Data.fromHex(submission.hash))
    var mined: TransactionReceipt?
    for _ in 0..<30 {
        if let receipt = try? await web3.eth.transactionReceipt(hash), receipt.status == .ok {
            mined = receipt
            break
        }
        try await Task.sleep(for: .milliseconds(100))
    }
    let receipt = try #require(mined)
    let balance = try await client.readBalanceOf(.init(address: sender))
    #expect(balance.uint256 == 42)
    let log = try #require(receipt.logs.first { $0.topics.first == Token.transferEventTopic })
    let transfer = try Token.decodeTransferEvent(log)
    #expect(transfer.to == sender && transfer.amount == 42)
}
