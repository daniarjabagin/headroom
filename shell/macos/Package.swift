// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "HeadroomMac",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "HeadroomKit", targets: ["HeadroomKit"]),
        .library(name: "HeadroomUI", targets: ["HeadroomUI"]),
        .library(name: "HeadroomSettings", targets: ["HeadroomSettings"]),
        .executable(name: "Headroom", targets: ["Headroom"]),
    ],
    targets: [
        .target(name: "HeadroomKit"),
        .target(name: "HeadroomUI", dependencies: ["HeadroomKit"]),
        .target(name: "HeadroomSettings", dependencies: ["HeadroomKit"]),
        .executableTarget(name: "Headroom", dependencies: ["HeadroomKit", "HeadroomUI", "HeadroomSettings"]),
        .testTarget(
            name: "HeadroomKitTests",
            dependencies: ["HeadroomKit"],
            resources: [.copy("Fixtures")]
        ),
    ]
)
