#!/bin/bash
# Organize trunk build output into subdirectories
# Run this after `trunk build --release`

set -e

DIST_DIR="dist"

if [[ ! -d "$DIST_DIR" ]]; then
    echo "Error: dist directory not found"
    exit 1
fi

cd "$DIST_DIR"

# Create subdirectories
mkdir -p wasm js styles

# Move files to appropriate directories
mv *.wasm wasm/ 2>/dev/null || true
mv *.js js/ 2>/dev/null || true
mv *.css styles/ 2>/dev/null || true

# Update index.html references
if [[ $(uname) == "Darwin" ]]; then
    # macOS sed
    sed -i '' \
        -e 's|href="/styles-|href="/styles/styles-|g' \
        -e 's|href="/ifc-lite-viewer-\([^"]*\)\.js"|href="/js/ifc-lite-viewer-\1.js"|g' \
        -e 's|href="/ifc-lite-viewer-\([^"]*\)_bg\.wasm"|href="/wasm/ifc-lite-viewer-\1_bg.wasm"|g' \
        -e "s|from '/ifc-lite-viewer-|from '/js/ifc-lite-viewer-|g" \
        -e "s|module_or_path: '/ifc-lite-viewer-|module_or_path: '/wasm/ifc-lite-viewer-|g" \
        index.html
else
    # Linux sed
    sed -i \
        -e 's|href="/styles-|href="/styles/styles-|g' \
        -e 's|href="/ifc-lite-viewer-\([^"]*\)\.js"|href="/js/ifc-lite-viewer-\1.js"|g' \
        -e 's|href="/ifc-lite-viewer-\([^"]*\)_bg\.wasm"|href="/wasm/ifc-lite-viewer-\1_bg.wasm"|g' \
        -e "s|from '/ifc-lite-viewer-|from '/js/ifc-lite-viewer-|g" \
        -e "s|module_or_path: '/ifc-lite-viewer-|module_or_path: '/wasm/ifc-lite-viewer-|g" \
        index.html
fi

# Also update the JS file to reference wasm in wasm/ subdirectory
JS_FILE=$(ls js/ifc-lite-viewer-*.js 2>/dev/null | head -1)
if [[ -n "$JS_FILE" ]]; then
    WASM_FILE=$(ls wasm/ifc-lite-viewer-*_bg.wasm 2>/dev/null | head -1 | xargs basename)
    if [[ -n "$WASM_FILE" ]]; then
        if [[ $(uname) == "Darwin" ]]; then
            sed -i '' "s|$WASM_FILE|../wasm/$WASM_FILE|g" "$JS_FILE"
        else
            sed -i "s|$WASM_FILE|../wasm/$WASM_FILE|g" "$JS_FILE"
        fi
    fi
fi

echo "Dist organized:"
echo "  wasm/  - $(ls wasm/ 2>/dev/null | wc -l | tr -d ' ') files"
echo "  js/    - $(ls js/ 2>/dev/null | wc -l | tr -d ' ') files"
echo "  styles/- $(ls styles/ 2>/dev/null | wc -l | tr -d ' ') files"
