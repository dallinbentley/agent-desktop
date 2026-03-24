// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "agent-computer",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "spike-ax-perf", targets: ["SpikeAXPerf"]),
        .executable(name: "spike-cgevent", targets: ["SpikeCGEvent"]),
        .executable(name: "spike-screenshot", targets: ["SpikeScreenshot"]),
        .executable(name: "spike-daemon", targets: ["SpikeDaemon"]),
    ],
    dependencies: [],
    targets: [
        .executableTarget(name: "SpikeAXPerf", path: "Sources/Spikes/AXPerf"),
        .executableTarget(name: "SpikeCGEvent", path: "Sources/Spikes/CGEvent"),
        .executableTarget(name: "SpikeScreenshot", path: "Sources/Spikes/Screenshot"),
        .executableTarget(name: "SpikeDaemon", path: "Sources/Spikes/Daemon"),
    ]
)
