// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "CodeBridge",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "CodeBridge", targets: ["CodeBridge"])
    ],
    dependencies: [],
    targets: [
        .executableTarget(
            name: "CodeBridge",
            dependencies: [],
            path: "Sources/CodeBridge"
        )
    ]
)
