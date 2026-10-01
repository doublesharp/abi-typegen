import Foundation
import Generated
import Testing

@Test func wrapperSdkNamesRemainUsableAsContractAndTupleNames() throws {
    let call = try Web32.encodeEcho(.init(ok: true))
    #expect(call.prefix(4) == Web32.echoSelector)
    #expect(try Web32.decodeEchoResult(Data(repeating: 0, count: 31) + Data([1])).ok)

    let params = SwiftWrapperNames.EchoParams(
        client: .init(ok: true), wrapperError: .init(ok: false), error: .init(ok: true),
        decoder: .init(ok: false), abiValue: .init(ok: true), contract: .init(ok: false)
    )
    let encoded = try SwiftWrapperNames.encodeEcho(params)
    let decoded = try SwiftWrapperNames.decodeEchoResult(Data(encoded.dropFirst(4)))
    #expect(decoded.client == params.client)
    #expect(decoded.wrapperError == params.wrapperError)
    #expect(decoded.error == params.error)
    #expect(decoded.decoder == params.decoder)
    #expect(decoded.abiValue == params.abiValue)
    #expect(decoded.contract == params.contract)
}
