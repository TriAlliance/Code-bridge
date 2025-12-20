// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "CodeBridge",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .executable(name: "CodeBridge", targets: ["CodeBridge"])
    ],
    dependencies: [
        // Swift dependencies would go here
    ],
    targets: [
        .executableTarget(
            name: "CodeBridge",
            dependencies: [],
            path: "Sources/CodeBridge",
            resources: [
                .process("Resources")
            ]
        )
    ]
)
