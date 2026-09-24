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
    dependencies: [
        .package(url: "https://github.com/sparkle-project/Sparkle", exact: "2.10.0")
    ],
    targets: [
        .target(name: "HeadroomKit"),
        .target(
            name: "HeadroomUI",
            dependencies: ["HeadroomKit"],
            resources: [.copy("Resources/ProviderIcons")]
        ),
        .target(name: "HeadroomSettings", dependencies: ["HeadroomKit"]),
        .executableTarget(
            name: "Headroom",
            dependencies: [
                "HeadroomKit", "HeadroomUI", "HeadroomSettings",
                .product(name: "Sparkle", package: "Sparkle", condition: .when(platforms: [.macOS])),
            ]
        ),
        .testTarget(
            name: "HeadroomKitTests",
            dependencies: ["HeadroomKit"],
            resources: [.copy("Fixtures")]
        ),
    ]
)
