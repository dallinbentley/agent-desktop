// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "agent-computer",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "agent-computer", targets: ["AgentComputerCLI"]),
        .executable(name: "agent-computer-daemon", targets: ["AgentComputerDaemon"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.3.0"),
    ],
    targets: [
        // Shared protocol types and utilities
        .target(
            name: "AgentComputerShared",
            path: "Sources/Shared"
        ),
        // CLI thin client
        .executableTarget(
            name: "AgentComputerCLI",
            dependencies: [
                "AgentComputerShared",
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ],
            path: "Sources/CLI"
        ),
        // Persistent daemon
        .executableTarget(
            name: "AgentComputerDaemon",
            dependencies: [
                "AgentComputerShared",
            ],
            path: "Sources/Daemon"
        ),
        // Spike targets (kept for reference)
        .executableTarget(name: "SpikeAXPerf", path: "Sources/Spikes/AXPerf"),
        .executableTarget(name: "SpikeCGEvent", path: "Sources/Spikes/CGEvent"),
        .executableTarget(name: "SpikeScreenshot", path: "Sources/Spikes/Screenshot"),
        .executableTarget(name: "SpikeDaemon", path: "Sources/Spikes/Daemon"),
        .executableTarget(name: "SpikeAXActions", path: "Sources/Spikes/AXActions"),
    ]
)
