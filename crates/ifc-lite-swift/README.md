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
- [x] Hierarchy panel with type grouping
- [x] Search/filter entities
- [x] Entity selection
- [x] Properties panel
- [x] Visibility control (hide/show/isolate)
- [x] Dark/light mode
- [ ] 3D viewport (placeholder - needs Metal integration)
- [ ] Camera controls (orbit, pan, zoom)
- [ ] Entity picking in 3D view
- [ ] Section planes
- [ ] Measurements

## TODO: 3D Viewport Integration

The `ViewportView.swift` is currently a placeholder. To add real 3D rendering:

### Option 1: SceneKit (easiest)
```swift
import SceneKit

struct SceneKitView: NSViewRepresentable {
    let meshes: [MeshData]

    func makeNSView(context: Context) -> SCNView {
        let scnView = SCNView()
        scnView.scene = buildScene(from: meshes)
        scnView.allowsCameraControl = true
        return scnView
    }

    func buildScene(from meshes: [MeshData]) -> SCNScene {
        let scene = SCNScene()
        for mesh in meshes {
            let geometry = buildGeometry(from: mesh)
            let node = SCNNode(geometry: geometry)
            scene.rootNode.addChildNode(node)
        }
        return scene
    }
}
```

### Option 2: Metal (best performance)
Create a custom `MTKView` wrapper and use the mesh data directly.

### Option 3: Embedded Bevy (same renderer as web)
Compile `ifc-lite-bevy` for iOS/macOS and embed as a Metal view.
