// Uses generated types from another module, which requires them to be public.
import BigInt
import Foundation
import Generated
import Web3Core

/// Builds a tuple value through the generated public initializer.
public func samplePosition() -> Vault.Position {
    Vault.Position(
        shares: BigUInt(7),
        depositedAt: BigUInt(9),
        token: EthereumAddress("0x0000000000000000000000000000000000000002")!
    )
}

/// Generated values cross concurrency domains because they are `Sendable`.
public func sendAcross(_ value: Vault.Position) async -> Vault.Position {
    await Task.detached { value }.value
}
