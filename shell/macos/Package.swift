// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "HeadroomMac",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "HeadroomKit", targets: ["HeadroomKit"]),
        .library(name: "HeadroomUI", targets: ["HeadroomUI"]),
        .executable(name: "Headroom", targets: ["Headroom"]),
    ],
    targets: [
        .target(name: "HeadroomKit"),
        .target(name: "HeadroomUI", dependencies: ["HeadroomKit"]),
        .executableTarget(name: "Headroom", dependencies: ["HeadroomKit", "HeadroomUI"]),
        .testTarget(
            name: "HeadroomKitTests",
            dependencies: ["HeadroomKit"],
            resources: [.copy("Fixtures")]
        ),
    ]
)
