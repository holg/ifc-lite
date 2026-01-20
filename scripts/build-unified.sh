#!/bin/bash
# Build the unified IFC-Lite viewer (single WASM, no Yew)
#
# This builds a single WASM module that includes:
# - IFC parsing
# - Geometry processing
# - Bevy 3D rendering
# - Pure Bevy UI (toolbar, panels, etc.)
#
# Memory efficiency: No JS bridge, no serialization overhead

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BEVY_CRATE="$ROOT_DIR/crates/ifc-lite-bevy"
OUT_DIR="$BEVY_CRATE/web/dist"

echo "=== Building IFC-Lite Unified Viewer ==="
echo "Output: $OUT_DIR"
echo ""

# Create output directory
mkdir -p "$OUT_DIR"

# Build with wasm-pack
echo "Building WASM with wasm-pack..."
cd "$BEVY_CRATE"

# Use --release for production, --dev for debugging
BUILD_MODE="${1:---release}"

wasm-pack build \
    --target web \
    $BUILD_MODE \
    --out-dir "$OUT_DIR/pkg" \
    --out-name ifc_lite_bevy \
    -- --features bevy-ui

# Copy HTML to dist
echo "Copying web assets..."
cp "$BEVY_CRATE/web/index.html" "$OUT_DIR/"

# Update import path in HTML (wasm-pack outputs to pkg/)
sed -i.bak "s|'./ifc_lite_bevy.js'|'./pkg/ifc_lite_bevy.js'|g" "$OUT_DIR/index.html"
rm -f "$OUT_DIR/index.html.bak"

echo ""
echo "=== Build Complete ==="
echo ""
echo "To serve locally:"
echo "  cd $OUT_DIR"
echo "  python3 -m http.server 8080"
echo "  # Then open http://localhost:8080"
echo ""
echo "Files:"
ls -la "$OUT_DIR"
ls -la "$OUT_DIR/pkg/"*.wasm 2>/dev/null || true
