// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "IfcLiteViewer",
    platforms: [
        .iOS(.v16),
        .macOS(.v13)
    ],
    products: [
        .executable(
            name: "IfcLiteViewer",
            targets: ["IfcLiteViewer"]
        ),
    ],
    dependencies: [
        .package(path: "../../ifc-lite-ffi/output/IfcLite"),
    ],
    targets: [
        .executableTarget(
            name: "IfcLiteViewer",
            dependencies: ["IfcLite"],
            path: "IfcLiteViewer"
        ),
    ]
)
