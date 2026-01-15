#!/bin/bash
set -e

# Build ifc-lite-bevy for Apple platforms (iOS and macOS)
# Creates an XCFramework for use in Swift projects

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/output"
FRAMEWORK_NAME="IfcLiteBevy"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building ifc-lite-bevy for Apple platforms${NC}"
echo "Output directory: $OUTPUT_DIR"

# Parse arguments
BUILD_TYPE="release"
SKIP_IOS=false
SKIP_MACOS=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --debug)
            BUILD_TYPE="debug"
            shift
            ;;
        --release)
            BUILD_TYPE="release"
            shift
            ;;
        --skip-ios)
            SKIP_IOS=true
            shift
            ;;
        --skip-macos)
            SKIP_MACOS=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Set cargo build flags
if [ "$BUILD_TYPE" = "release" ]; then
    CARGO_FLAGS="--release"
    TARGET_DIR="release"
else
    CARGO_FLAGS=""
    TARGET_DIR="debug"
fi

# Clean output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/libs"
mkdir -p "$OUTPUT_DIR/include"

# Copy header file
cp "$SCRIPT_DIR/include/libifc_lite_bevy.h" "$OUTPUT_DIR/include/"

# Function to build for a target
build_target() {
    local target=$1
    local description=$2

    echo -e "${YELLOW}Building for $description ($target)...${NC}"

    # Check if target is installed
    if ! rustup target list --installed | grep -q "$target"; then
        echo "Installing target $target..."
        rustup target add "$target"
    fi

    # Build
    cd "$PROJECT_ROOT"
    cargo build -p ifc-lite-bevy $CARGO_FLAGS --target "$target"

    # Copy the library
    local lib_path="$PROJECT_ROOT/target/$target/$TARGET_DIR/libifc_lite_bevy.a"
    if [ -f "$lib_path" ]; then
        cp "$lib_path" "$OUTPUT_DIR/libs/libifc_lite_bevy-$target.a"
        echo -e "${GREEN}  Built: libifc_lite_bevy-$target.a${NC}"
    else
        echo -e "${RED}  Error: Library not found at $lib_path${NC}"
        return 1
    fi
}

# Build for macOS
if [ "$SKIP_MACOS" = false ]; then
    echo -e "\n${GREEN}=== Building for macOS ===${NC}"

    # macOS ARM64
    build_target "aarch64-apple-darwin" "macOS ARM64"

    # macOS x86_64
    build_target "x86_64-apple-darwin" "macOS x86_64"

    # Create universal macOS binary
    echo -e "${YELLOW}Creating universal macOS binary...${NC}"
    lipo -create \
        "$OUTPUT_DIR/libs/libifc_lite_bevy-aarch64-apple-darwin.a" \
        "$OUTPUT_DIR/libs/libifc_lite_bevy-x86_64-apple-darwin.a" \
        -output "$OUTPUT_DIR/libs/libifc_lite_bevy-macos-universal.a"
    echo -e "${GREEN}  Created: libifc_lite_bevy-macos-universal.a${NC}"
fi

# Build for iOS
if [ "$SKIP_IOS" = false ]; then
    echo -e "\n${GREEN}=== Building for iOS ===${NC}"

    # iOS Device (ARM64)
    build_target "aarch64-apple-ios" "iOS Device ARM64"

    # iOS Simulator ARM64
    build_target "aarch64-apple-ios-sim" "iOS Simulator ARM64"

    # iOS Simulator x86_64 (for Intel Macs)
    build_target "x86_64-apple-ios" "iOS Simulator x86_64"

    # Create universal iOS simulator binary
    echo -e "${YELLOW}Creating universal iOS simulator binary...${NC}"
    lipo -create \
        "$OUTPUT_DIR/libs/libifc_lite_bevy-aarch64-apple-ios-sim.a" \
        "$OUTPUT_DIR/libs/libifc_lite_bevy-x86_64-apple-ios.a" \
        -output "$OUTPUT_DIR/libs/libifc_lite_bevy-ios-simulator-universal.a"
    echo -e "${GREEN}  Created: libifc_lite_bevy-ios-simulator-universal.a${NC}"
fi

# Create XCFramework
echo -e "\n${GREEN}=== Creating XCFramework ===${NC}"

XCFRAMEWORK_PATH="$OUTPUT_DIR/$FRAMEWORK_NAME.xcframework"
rm -rf "$XCFRAMEWORK_PATH"

# Build the xcodebuild command based on what was built
XCFRAMEWORK_ARGS=()

if [ "$SKIP_MACOS" = false ]; then
    # Create macOS framework structure
    MACOS_FRAMEWORK="$OUTPUT_DIR/frameworks/macos/$FRAMEWORK_NAME.framework"
    mkdir -p "$MACOS_FRAMEWORK/Headers"
    cp "$OUTPUT_DIR/include/libifc_lite_bevy.h" "$MACOS_FRAMEWORK/Headers/"
    cp "$OUTPUT_DIR/libs/libifc_lite_bevy-macos-universal.a" "$MACOS_FRAMEWORK/$FRAMEWORK_NAME"

    # Create Info.plist for macOS
    cat > "$MACOS_FRAMEWORK/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$FRAMEWORK_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.ifc-lite.bevy</string>
    <key>CFBundleName</key>
    <string>$FRAMEWORK_NAME</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>MinimumOSVersion</key>
    <string>13.0</string>
</dict>
</plist>
EOF

    XCFRAMEWORK_ARGS+=(-framework "$MACOS_FRAMEWORK")
fi

if [ "$SKIP_IOS" = false ]; then
    # Create iOS device framework structure
    IOS_FRAMEWORK="$OUTPUT_DIR/frameworks/ios-device/$FRAMEWORK_NAME.framework"
    mkdir -p "$IOS_FRAMEWORK/Headers"
    cp "$OUTPUT_DIR/include/libifc_lite_bevy.h" "$IOS_FRAMEWORK/Headers/"
    cp "$OUTPUT_DIR/libs/libifc_lite_bevy-aarch64-apple-ios.a" "$IOS_FRAMEWORK/$FRAMEWORK_NAME"

    cat > "$IOS_FRAMEWORK/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$FRAMEWORK_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.ifc-lite.bevy</string>
    <key>CFBundleName</key>
    <string>$FRAMEWORK_NAME</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>MinimumOSVersion</key>
    <string>15.0</string>
</dict>
</plist>
EOF

    XCFRAMEWORK_ARGS+=(-framework "$IOS_FRAMEWORK")

    # Create iOS simulator framework structure
    IOS_SIM_FRAMEWORK="$OUTPUT_DIR/frameworks/ios-simulator/$FRAMEWORK_NAME.framework"
    mkdir -p "$IOS_SIM_FRAMEWORK/Headers"
    cp "$OUTPUT_DIR/include/libifc_lite_bevy.h" "$IOS_SIM_FRAMEWORK/Headers/"
    cp "$OUTPUT_DIR/libs/libifc_lite_bevy-ios-simulator-universal.a" "$IOS_SIM_FRAMEWORK/$FRAMEWORK_NAME"

    cat > "$IOS_SIM_FRAMEWORK/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$FRAMEWORK_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.ifc-lite.bevy</string>
    <key>CFBundleName</key>
    <string>$FRAMEWORK_NAME</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>MinimumOSVersion</key>
    <string>15.0</string>
</dict>
</plist>
EOF

    XCFRAMEWORK_ARGS+=(-framework "$IOS_SIM_FRAMEWORK")
fi

# Create the XCFramework
xcodebuild -create-xcframework \
    "${XCFRAMEWORK_ARGS[@]}" \
    -output "$XCFRAMEWORK_PATH"

echo -e "${GREEN}Created: $XCFRAMEWORK_PATH${NC}"

# Create Swift Package
echo -e "\n${GREEN}=== Creating Swift Package ===${NC}"

SWIFT_PACKAGE_DIR="$OUTPUT_DIR/$FRAMEWORK_NAME"
mkdir -p "$SWIFT_PACKAGE_DIR/Sources/$FRAMEWORK_NAME"

# Copy XCFramework
cp -r "$XCFRAMEWORK_PATH" "$SWIFT_PACKAGE_DIR/"

# Create Package.swift
cat > "$SWIFT_PACKAGE_DIR/Package.swift" << 'EOF'
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "IfcLiteBevy",
    platforms: [
        .macOS(.v13),
        .iOS(.v15)
    ],
    products: [
        .library(
            name: "IfcLiteBevy",
            targets: ["IfcLiteBevy", "IfcLiteBevyFramework"]
        ),
    ],
    targets: [
        .target(
            name: "IfcLiteBevy",
            dependencies: ["IfcLiteBevyFramework"],
            path: "Sources/IfcLiteBevy"
        ),
        .binaryTarget(
            name: "IfcLiteBevyFramework",
            path: "IfcLiteBevy.xcframework"
        ),
    ]
)
EOF

# Create Swift wrapper module
cat > "$SWIFT_PACKAGE_DIR/Sources/$FRAMEWORK_NAME/IfcLiteBevy.swift" << 'EOF'
import Foundation
import IfcLiteBevyFramework

/// Swift wrapper for the IFC-Lite Bevy viewer
public class BevyViewer {
    private var app: OpaquePointer?

    public init() {}

    /// Create and attach the Bevy app to a native view
    /// - Parameters:
    ///   - view: The native view (UIView or NSView) with CAMetalLayer backing
    ///   - maxFps: Maximum frames per second
    ///   - scaleFactor: Display scale factor
    public func attach(to view: AnyObject, maxFps: Int32 = 60, scaleFactor: Float) {
        let viewPtr = Unmanaged.passUnretained(view).toOpaque()
        app = create_bevy_app(viewPtr, maxFps, scaleFactor)
    }

    /// Process a single frame. Call this from your display link.
    public func update() {
        guard let app = app else { return }
        enter_frame(app)
    }

    /// Release all resources
    public func release() {
        guard let app = app else { return }
        release_bevy_app(app)
        self.app = nil
    }

    deinit {
        release()
    }

    // MARK: - Data Loading

    /// Load geometry from JSON
    public func loadGeometry(json: String) -> Bool {
        guard let app = app else { return false }
        return load_geometry(app, json)
    }

    /// Load entity metadata from JSON
    public func loadEntities(json: String) -> Bool {
        guard let app = app else { return false }
        return load_entities(app, json)
    }

    // MARK: - Selection

    /// Select an entity by ID
    public func select(entityId: UInt64) {
        guard let app = app else { return }
        select_entity(app, entityId)
    }

    /// Clear the current selection
    public func clearSelection() {
        guard let app = app else { return }
        clear_selection(app)
    }

    // MARK: - Visibility

    /// Hide an entity
    public func hide(entityId: UInt64) {
        guard let app = app else { return }
        hide_entity(app, entityId)
    }

    /// Show a hidden entity
    public func show(entityId: UInt64) {
        guard let app = app else { return }
        show_entity(app, entityId)
    }

    /// Show all hidden entities
    public func showAll() {
        guard let app = app else { return }
        show_all(app)
    }

    /// Isolate entities (hide all others)
    public func isolate(entityIds: [UInt64]) {
        guard let app = app else { return }
        entityIds.withUnsafeBufferPointer { buffer in
            isolate_entities(app, buffer.baseAddress, buffer.count)
        }
    }

    // MARK: - Camera

    /// Set camera to home view
    public func cameraHome() {
        guard let app = app else { return }
        camera_home(app)
    }

    /// Fit camera to show all geometry
    public func cameraFitAll() {
        guard let app = app else { return }
        camera_fit_all(app)
    }

    /// Focus camera on a specific entity
    public func cameraFocus(entityId: UInt64) {
        guard let app = app else { return }
        camera_focus_entity(app, entityId)
    }

    // MARK: - Touch Input

    /// Handle touch started
    public func touchStarted(x: Float, y: Float) {
        guard let app = app else { return }
        touch_started(app, x, y)
    }

    /// Handle touch moved
    public func touchMoved(x: Float, y: Float) {
        guard let app = app else { return }
        touch_moved(app, x, y)
    }

    /// Handle touch ended
    public func touchEnded(x: Float, y: Float) {
        guard let app = app else { return }
        touch_ended(app, x, y)
    }

    /// Handle touch cancelled
    public func touchCancelled(x: Float, y: Float) {
        guard let app = app else { return }
        touch_cancelled(app, x, y)
    }

    // MARK: - Theme

    /// Set the viewer theme
    public func setTheme(dark: Bool) {
        guard let app = app else { return }
        set_theme(app, dark)
    }
}
EOF

echo -e "${GREEN}Created Swift Package at: $SWIFT_PACKAGE_DIR${NC}"

# Summary
echo -e "\n${GREEN}=== Build Complete ===${NC}"
echo "Output directory: $OUTPUT_DIR"
echo ""
echo "Contents:"
ls -la "$OUTPUT_DIR"
echo ""
echo "To use in your Swift project:"
echo "  1. Add the Swift Package from: $SWIFT_PACKAGE_DIR"
echo "  2. Or add the XCFramework directly: $XCFRAMEWORK_PATH"
