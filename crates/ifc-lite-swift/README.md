# IFC-Lite Swift

SwiftUI-based IFC viewer for iOS and macOS, powered by the Rust `ifc-lite` library.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      SwiftUI App                         │
├──────────────┬────────────────────────┬─────────────────┤
│ HierarchyPanel │      ViewportView      │ PropertiesPanel │
│  (native UI)   │    (3D rendering)      │   (native UI)   │
└───────┬────────┴───────────┬───────────┴────────┬────────┘
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
                    ┌────────▼────────┐
                    │  ViewerViewModel │
                    │   (Swift state)  │
                    └────────┬────────┘
                             │ FFI calls
                    ┌────────▼────────┐
                    │   IfcLite FFI    │
                    │  (Rust library)  │
                    └─────────────────┘
```

## Components

- **IfcLiteViewerApp.swift** - App entry point
- **ViewerViewModel.swift** - Main state management (connects to Rust FFI)
- **ContentView.swift** - Main layout with NavigationSplitView
- **HierarchyPanel.swift** - Tree view of model entities (grouped by type)
- **PropertiesPanel.swift** - Selected entity properties and actions
- **ToolbarView.swift** - File operations, view controls, visibility
- **ViewportView.swift** - 3D viewport (placeholder, needs Metal/SceneKit integration)

## Building

### Prerequisites

1. Build the FFI library first:
   ```bash
   cd crates/ifc-lite-ffi
   ./build-apple.sh
   ```

2. This creates the XCFramework and Swift package in `output/IfcLite/`

### Running

#### Option A: Swift Package Manager (command line)
```bash
cd crates/ifc-lite-swift/IfcLiteViewer
swift build
swift run
```

#### Option B: Xcode
1. Open `IfcLiteViewer/Package.swift` in Xcode
2. Wait for package resolution
3. Build and run (⌘R)

## Features

- [x] IFC file loading (via file dialog or drag & drop)
- [x] Hierarchy panel with spatial structure (Project > Site > Building > Storey)
- [x] Search/filter entities
- [x] Entity selection (click in hierarchy or 3D view)
- [x] Properties panel with IFC property sets
- [x] Visibility control (hide/show/isolate)
- [x] 3D viewport with SceneKit rendering
- [x] Camera controls (orbit, pan, zoom)
- [x] Zoom to entity
- [x] Entity picking in 3D view
- [ ] Section planes
- [ ] Measurements
