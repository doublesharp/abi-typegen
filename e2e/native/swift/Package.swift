// swift-tools-version:6.0
// Compile check for generated Swift bindings.
import PackageDescription

let package = Package(
    name: "AbiTypegenE2E",
    platforms: [.macOS(.v12), .iOS(.v15)],
    products: [.library(name: "Consumer", targets: ["Consumer"])],
    dependencies: [
        .package(url: "https://github.com/web3swift-team/web3swift.git", from: "3.3.0"),
    ],
    targets: [
        // Generated sources live in their own module, so the consumer
        // exercises `public` access across a module boundary.
        .target(
            name: "Generated",
            dependencies: [.product(name: "web3swift", package: "web3swift")]
        ),
        .target(name: "Consumer", dependencies: ["Generated"]),
        .testTarget(name: "ConsumerTests", dependencies: ["Consumer", "Generated"]),
    ]
)
