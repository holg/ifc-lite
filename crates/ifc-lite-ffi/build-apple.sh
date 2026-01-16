#!/bin/bash
# Build script for iOS and macOS XCFramework
# Creates universal binaries for all Apple platforms

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CRATE_NAME="ifc-lite-ffi"
LIB_NAME="libifc_lite_ffi"
OUTPUT_DIR="$SCRIPT_DIR/output"
XCFRAMEWORK_NAME="IfcLiteFFI"

echo "🔧 Building IFC-Lite FFI for Apple platforms..."
echo "   Project root: $PROJECT_ROOT"
echo "   Output dir: $OUTPUT_DIR"

# Ensure we're in the project root
cd "$PROJECT_ROOT"

# Clean previous build
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/headers"
mkdir -p "$OUTPUT_DIR/swift"

# Generate UniFFI bindings (Swift + C header)
echo "🔄 Generating UniFFI bindings..."
cargo build -p "$CRATE_NAME" --release 2>&1 | grep -E "(Compiling|Finished|error)" || true
cargo run -p "$CRATE_NAME" --features cli --bin uniffi-bindgen -- \
    generate --library "target/release/$LIB_NAME.dylib" \
    --language swift \
    --out-dir "$SCRIPT_DIR/bindings" 2>&1 | grep -v "Warning:" || true

# Copy headers and Swift bindings
echo "📋 Copying headers and Swift bindings..."
cp "$SCRIPT_DIR/bindings/ifc_lite_ffiFFI.h" "$OUTPUT_DIR/headers/"
cp "$SCRIPT_DIR/bindings/ifc_lite_ffiFFI.modulemap" "$OUTPUT_DIR/headers/module.modulemap"
cp "$SCRIPT_DIR/bindings/ifc_lite_ffi.swift" "$OUTPUT_DIR/swift/"

# Check for required Rust targets
echo "🔍 Checking Rust targets..."
TARGETS=(
    "aarch64-apple-ios"           # iOS device (arm64)
    "aarch64-apple-ios-sim"       # iOS Simulator (Apple Silicon)
    "x86_64-apple-ios"            # iOS Simulator (Intel)
    "aarch64-apple-darwin"        # macOS (Apple Silicon)
    "x86_64-apple-darwin"         # macOS (Intel)
)

for target in "${TARGETS[@]}"; do
    if ! rustup target list --installed | grep -q "$target"; then
        echo "   Installing target: $target"
        rustup target add "$target"
    fi
done

# Build for each target
echo "🏗️  Building for all targets..."

build_target() {
    local target=$1
    echo "   Building for $target..."
    cargo build -p "$CRATE_NAME" --release --target "$target" 2>&1 | grep -E "(Compiling|Finished|error)" || true
}

# Build all targets in parallel
build_target "aarch64-apple-ios" &
build_target "aarch64-apple-ios-sim" &
build_target "x86_64-apple-ios" &
build_target "aarch64-apple-darwin" &
build_target "x86_64-apple-darwin" &
wait

echo "✅ All targets built"

# Create output directories for each platform
mkdir -p "$OUTPUT_DIR/ios-device"
mkdir -p "$OUTPUT_DIR/ios-simulator"
mkdir -p "$OUTPUT_DIR/macos"

# Copy libraries
echo "📦 Copying libraries..."

# iOS Device (arm64 only)
cp "target/aarch64-apple-ios/release/$LIB_NAME.a" "$OUTPUT_DIR/ios-device/"

# iOS Simulator (universal: arm64 + x86_64)
echo "   Creating iOS Simulator universal binary..."
lipo -create \
    "target/aarch64-apple-ios-sim/release/$LIB_NAME.a" \
    "target/x86_64-apple-ios/release/$LIB_NAME.a" \
    -output "$OUTPUT_DIR/ios-simulator/$LIB_NAME.a"

# macOS (universal: arm64 + x86_64)
echo "   Creating macOS universal binary..."
lipo -create \
    "target/aarch64-apple-darwin/release/$LIB_NAME.a" \
    "target/x86_64-apple-darwin/release/$LIB_NAME.a" \
    -output "$OUTPUT_DIR/macos/$LIB_NAME.a"

# Verify universal binaries
echo "🔍 Verifying universal binaries..."
echo "   iOS Simulator:"
lipo -info "$OUTPUT_DIR/ios-simulator/$LIB_NAME.a"
echo "   macOS:"
lipo -info "$OUTPUT_DIR/macos/$LIB_NAME.a"

# Create XCFramework
echo "📱 Creating XCFramework..."
rm -rf "$OUTPUT_DIR/$XCFRAMEWORK_NAME.xcframework"

xcodebuild -create-xcframework \
    -library "$OUTPUT_DIR/ios-device/$LIB_NAME.a" \
    -headers "$OUTPUT_DIR/headers" \
    -library "$OUTPUT_DIR/ios-simulator/$LIB_NAME.a" \
    -headers "$OUTPUT_DIR/headers" \
    -library "$OUTPUT_DIR/macos/$LIB_NAME.a" \
    -headers "$OUTPUT_DIR/headers" \
    -output "$OUTPUT_DIR/$XCFRAMEWORK_NAME.xcframework"

echo "✅ XCFramework created: $OUTPUT_DIR/$XCFRAMEWORK_NAME.xcframework"

# Create a Swift package for easy integration
echo "📦 Creating Swift Package..."
PACKAGE_DIR="$OUTPUT_DIR/IfcLite"
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR/Sources/IfcLite"

# Copy Swift bindings
cp "$OUTPUT_DIR/swift/ifc_lite_ffi.swift" "$PACKAGE_DIR/Sources/IfcLite/"

# Copy XCFramework
cp -R "$OUTPUT_DIR/$XCFRAMEWORK_NAME.xcframework" "$PACKAGE_DIR/"

# Create Package.swift
cat > "$PACKAGE_DIR/Package.swift" << 'EOF'
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "IfcLite",
    platforms: [
        .iOS(.v15),
        .macOS(.v12)
    ],
    products: [
        .library(
            name: "IfcLite",
            targets: ["IfcLite", "IfcLiteFFI"]
        ),
    ],
    targets: [
        .target(
            name: "IfcLite",
            dependencies: ["IfcLiteFFI"],
            path: "Sources/IfcLite"
        ),
        .binaryTarget(
            name: "IfcLiteFFI",
            path: "IfcLiteFFI.xcframework"
        ),
    ]
)
EOF

echo "✅ Swift Package created: $PACKAGE_DIR"

# Print summary
echo ""
echo "=========================================="
echo "🎉 Build complete!"
echo "=========================================="
echo ""
echo "Output files:"
echo "  📱 XCFramework: $OUTPUT_DIR/$XCFRAMEWORK_NAME.xcframework"
echo "  📦 Swift Package: $PACKAGE_DIR"
echo "  📄 Swift bindings: $OUTPUT_DIR/swift/ifc_lite_ffi.swift"
echo ""
echo "To use in your iOS/macOS project:"
echo ""
echo "Option 1: Swift Package Manager"
echo "  1. Copy the 'IfcLite' folder to your project"
echo "  2. Add it as a local package dependency"
echo ""
echo "Option 2: Manual integration"
echo "  1. Add IfcLiteFFI.xcframework to your project"
echo "  2. Add ifc_lite_ffi.swift to your target"
echo ""
echo "Example usage in Swift:"
echo "  import IfcLite"
echo ""
echo "  let scene = IfcScene()"
echo "  let result = try scene.loadFile(path: \"/path/to/model.ifc\")"
echo "  print(\"Loaded \\(result.entities.count) entities\")"
echo ""
