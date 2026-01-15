# IFC-Lite: Migration Plan from TypeScript/React to Bevy/Yew

## Executive Summary

This document outlines the migration of the ifc-lite viewer from the current React/TypeScript/WebGPU stack to a unified Rust-based Bevy/Yew architecture. This aligns with the gldf-rs ecosystem and enables code reuse for IFC functionality.

---

## Current Architecture (TypeScript/React)

### Stack
- **UI Framework:** React 18 + Radix UI + TailwindCSS
- **State Management:** Zustand (~1,100 LOC store)
- **Rendering:** Custom WebGPU pipeline with PBR shading
- **Build:** Vite + pnpm workspace
- **Packages:** 11 TypeScript packages + 3 Rust/WASM crates

### Key Metrics
| Component | Lines of Code |
|-----------|---------------|
| Viewer App (React) | ~3,500 LOC |
| Renderer Package | ~1,200 LOC |
| Zustand Store | ~1,100 LOC |
| Total TypeScript | ~8,000+ LOC |
| Rust Core (WASM) | ~7,600 LOC |

### Features to Migrate
- [x] 3-panel responsive layout (hierarchy, viewport, properties)
- [x] Orbit/Pan/Zoom/Walk camera modes
- [x] Single & multi-selection (click, box select)
- [x] Entity visibility (hide/isolate/show all)
- [x] Storey filtering
- [x] Measurement tool
- [x] Section plane tool
- [x] Property/quantity display
- [x] ViewCube widget
- [x] Context menus
- [x] Keyboard shortcuts
- [x] Export (GLB, CSV, screenshot)
- [x] Dark/light theme
- [x] Mobile responsive

---

## Target Architecture (Bevy/Yew)

### Stack
- **UI Framework:** Yew 0.22 (component-based, CSR)
- **State Management:** Yew `use_reducer` + context
- **Rendering:** Bevy 0.15 with WebGL2/WebGPU
- **Build:** Trunk (WASM bundler)
- **Integration:** localStorage bridge pattern (proven in gldf-rs)

### Advantages
1. **Unified Rust Codebase** - No TypeScript/JavaScript maintenance
2. **gldf-rs Compatibility** - Share IFC parsing, geometry, and rendering code
3. **Single Language** - Rust across parser, geometry, renderer, and UI
4. **Plugin Ecosystem** - Bevy plugins for photometry (eulumdat-bevy), L3D
5. **Future-Proof** - Bevy 0.15 has strong WASM + GPU support

---

## Migration Phases

### Phase 1: Foundation (Week 1-2)

#### 1.1 Project Structure
```
ifc-lite/
├── crates/
│   ├── ifc-lite-core/        # (existing) STEP parser
│   ├── ifc-lite-geometry/    # (existing) Geometry processing
│   ├── ifc-lite-bevy/        # (new) Bevy 3D renderer
│   ├── ifc-lite-yew/         # (new) Yew UI components
│   └── ifc-lite-viewer/      # (new) Main viewer app
├── apps/
│   └── viewer/               # (migrate) Trunk build target
└── shared/
    └── types/                # Shared Rust types
```

#### 1.2 Core Dependencies
```toml
[workspace.dependencies]
# Bevy (3D engine)
bevy = { version = "0.15", default-features = false, features = [
    "bevy_asset", "bevy_render", "bevy_pbr", "bevy_core_pipeline",
    "bevy_winit", "webgl2", "png"
]}

# Yew (UI)
yew = { version = "0.22", features = ["csr"] }
yew-hooks = "0.3"

# WASM
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [
    "Window", "Document", "HtmlCanvasElement", "Storage",
    "MouseEvent", "KeyboardEvent", "WheelEvent", "TouchEvent"
]}
js-sys = "0.3"
gloo = { version = "0.11", features = ["file", "storage", "timers"] }
wasm-bindgen-futures = "0.4"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.22"
```

#### 1.3 Bevy-Yew Bridge (localStorage pattern)
```rust
// Storage keys (match gldf-rs pattern)
pub const IFC_GEOMETRY_KEY: &str = "ifc_lite_geometry";
pub const IFC_ENTITIES_KEY: &str = "ifc_lite_entities";
pub const IFC_TIMESTAMP_KEY: &str = "ifc_lite_timestamp";
pub const IFC_SELECTION_KEY: &str = "ifc_lite_selection";
pub const IFC_VISIBILITY_KEY: &str = "ifc_lite_visibility";

// Bridge functions (JavaScript FFI)
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadBevyViewer)]
    fn load_bevy_viewer() -> js_sys::Promise;

    #[wasm_bindgen(js_name = saveIfcGeometry)]
    fn save_ifc_geometry(json: &str);

    #[wasm_bindgen(js_name = updateSelection)]
    fn update_selection(ids: &str);
}
```

---

### Phase 2: Bevy Renderer (Week 2-4)

#### 2.1 ifc-lite-bevy Crate
```rust
// src/lib.rs
pub struct IfcViewerPlugin;

impl Plugin for IfcViewerPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<IfcSceneData>()
            .init_resource::<ViewerState>()
            .add_plugins(IfcMeshPlugin)
            .add_plugins(IfcCameraPlugin)
            .add_plugins(IfcPickingPlugin)
            .add_plugins(IfcSectionPlanePlugin)
            .add_systems(Update, poll_scene_changes);
    }
}
```

#### 2.2 Mesh System
```rust
#[derive(Resource, Default)]
pub struct IfcSceneData {
    pub meshes: Vec<IfcMesh>,
    pub bounds: Option<Aabb>,
    pub timestamp: u64,
}

#[derive(Component)]
pub struct IfcEntity {
    pub id: u64,
    pub entity_type: String,
    pub name: Option<String>,
}

fn spawn_ifc_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    scene_data: &IfcSceneData,
) {
    for ifc_mesh in &scene_data.meshes {
        let mesh = create_bevy_mesh(&ifc_mesh.geometry);
        let material = StandardMaterial {
            base_color: ifc_mesh.color.into(),
            metallic: 0.1,
            perceptual_roughness: 0.8,
            ..default()
        };

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::from_matrix(ifc_mesh.transform),
            IfcEntity {
                id: ifc_mesh.entity_id,
                entity_type: ifc_mesh.entity_type.clone(),
                name: ifc_mesh.name.clone(),
            },
        ));
    }
}
```

#### 2.3 Camera System (Orbit/Pan/Zoom)
```rust
#[derive(Resource)]
pub struct CameraController {
    pub mode: CameraMode,
    pub target: Vec3,
    pub distance: f32,
    pub azimuth: f32,
    pub elevation: f32,
    pub damping: f32,
    pub velocity: Vec3,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CameraMode {
    Orbit,
    Pan,
    Walk,
}

fn camera_orbit_system(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    mut controller: ResMut<CameraController>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    if mouse.pressed(MouseButton::Left) && controller.mode == CameraMode::Orbit {
        for ev in motion.read() {
            controller.azimuth -= ev.delta.x * 0.005;
            controller.elevation -= ev.delta.y * 0.005;
            controller.elevation = controller.elevation.clamp(-1.5, 1.5);
        }
    }

    // Apply with damping
    let mut transform = camera.single_mut();
    let position = controller.target + spherical_to_cartesian(
        controller.distance,
        controller.azimuth,
        controller.elevation,
    );
    transform.translation = transform.translation.lerp(position, 1.0 - controller.damping);
    transform.look_at(controller.target, Vec3::Y);
}
```

#### 2.4 Selection & Picking
```rust
#[derive(Resource, Default)]
pub struct SelectionState {
    pub selected: HashSet<u64>,
    pub hovered: Option<u64>,
}

fn picking_system(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    rapier_context: Res<RapierContext>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut selection: ResMut<SelectionState>,
    entities: Query<&IfcEntity>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        let window = windows.single();
        if let Some(cursor) = window.cursor_position() {
            let (camera, transform) = cameras.single();
            if let Some(ray) = camera.viewport_to_world(transform, cursor) {
                if let Some((entity, _)) = rapier_context.cast_ray(
                    ray.origin, ray.direction.into(), 1000.0, true, default()
                ) {
                    if let Ok(ifc_entity) = entities.get(entity) {
                        selection.selected.clear();
                        selection.selected.insert(ifc_entity.id);
                        // Notify Yew via localStorage
                        notify_selection_change(&selection.selected);
                    }
                }
            }
        }
    }
}
```

#### 2.5 Section Plane
```rust
#[derive(Resource)]
pub struct SectionPlane {
    pub enabled: bool,
    pub axis: Axis,
    pub position: f32,  // 0.0 - 1.0
    pub flipped: bool,
}

// Custom shader for section plane clipping
const SECTION_SHADER: &str = r#"
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let plane_normal = section_plane.xyz;
    let plane_dist = section_plane.w;

    if dot(in.world_position.xyz, plane_normal) > plane_dist {
        discard;
    }

    // Standard PBR lighting...
    return pbr_result;
}
"#;
```

---

### Phase 3: Yew UI Components (Week 4-6)

#### 3.1 Component Structure
```
ifc-lite-yew/src/
├── lib.rs
├── app.rs                    # Main App component
├── state.rs                  # Global state (Reducer)
├── bridge.rs                 # Bevy-Yew bridge
├── components/
│   ├── mod.rs
│   ├── viewer_layout.rs      # 3-panel layout
│   ├── toolbar.rs            # Tool buttons
│   ├── hierarchy_panel.rs    # Entity tree
│   ├── properties_panel.rs   # Properties/quantities
│   ├── viewport.rs           # Canvas + Bevy embed
│   ├── view_cube.rs          # 3D orientation widget
│   ├── status_bar.rs         # Stats display
│   ├── context_menu.rs       # Right-click menu
│   └── overlays/
│       ├── measure_tool.rs
│       └── section_tool.rs
└── hooks/
    ├── use_ifc.rs            # IFC loading
    └── use_keyboard.rs       # Shortcuts
```

#### 3.2 State Management
```rust
// state.rs
#[derive(Clone, PartialEq)]
pub struct ViewerState {
    pub loading: bool,
    pub progress: Option<Progress>,
    pub error: Option<String>,
    pub selected_ids: HashSet<u64>,
    pub hidden_ids: HashSet<u64>,
    pub isolated_ids: Option<HashSet<u64>>,
    pub active_tool: Tool,
    pub theme: Theme,
    pub left_panel_collapsed: bool,
    pub right_panel_collapsed: bool,
    pub section_plane: SectionPlaneState,
    pub measurements: Vec<Measurement>,
}

pub enum ViewerAction {
    // Loading
    SetLoading(bool),
    SetProgress(Progress),
    SetError(String),
    ClearError,

    // Selection
    SetSelection(HashSet<u64>),
    AddToSelection(u64),
    RemoveFromSelection(u64),
    ClearSelection,

    // Visibility
    HideEntity(u64),
    ShowEntity(u64),
    IsolateEntities(HashSet<u64>),
    ShowAll,

    // Tools
    SetActiveTool(Tool),

    // UI
    ToggleTheme,
    SetLeftPanelCollapsed(bool),
    SetRightPanelCollapsed(bool),

    // Section
    SetSectionAxis(Axis),
    SetSectionPosition(f32),
    ToggleSectionPlane,

    // Measurement
    AddMeasurement(Measurement),
    RemoveMeasurement(usize),
    ClearMeasurements,
}

impl Reducible for ViewerState {
    type Action = ViewerAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut next = (*self).clone();
        match action {
            ViewerAction::SetSelection(ids) => {
                next.selected_ids = ids;
                notify_bevy_selection(&next.selected_ids);
            }
            // ... other actions
        }
        Rc::new(next)
    }
}
```

#### 3.3 Main Layout Component
```rust
// viewer_layout.rs
#[function_component]
pub fn ViewerLayout() -> Html {
    let state = use_reducer(ViewerState::default);

    html! {
        <ContextProvider<UseReducerHandle<ViewerState>> context={state.clone()}>
            <div class="viewer-layout">
                // Left Panel (Hierarchy)
                if !state.left_panel_collapsed {
                    <div class="panel panel-left">
                        <HierarchyPanel />
                    </div>
                }

                // Center (Viewport)
                <div class="viewport-container">
                    <Toolbar />
                    <Viewport />
                    <ViewportOverlays />
                    <StatusBar />
                </div>

                // Right Panel (Properties)
                if !state.right_panel_collapsed {
                    <div class="panel panel-right">
                        <PropertiesPanel />
                    </div>
                }
            </div>
        </ContextProvider<UseReducerHandle<ViewerState>>>
    }
}
```

#### 3.4 Viewport Component (Bevy Embed)
```rust
// viewport.rs
#[function_component]
pub fn Viewport() -> Html {
    let bevy_loaded = use_state(|| false);
    let canvas_ref = use_node_ref();

    // Load Bevy on mount
    {
        let bevy_loaded = bevy_loaded.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match load_bevy_viewer().await {
                    Ok(_) => bevy_loaded.set(true),
                    Err(e) => log::error!("Failed to load Bevy: {:?}", e),
                }
            });
            || ()
        });
    }

    html! {
        <div class="viewport">
            <canvas
                ref={canvas_ref}
                id="bevy-canvas"
                class="viewport-canvas"
            />
            if !*bevy_loaded {
                <div class="loading-overlay">
                    <span>{"Loading 3D viewer..."}</span>
                </div>
            }
        </div>
    }
}
```

#### 3.5 Hierarchy Panel with Virtual Scrolling
```rust
// hierarchy_panel.rs
#[function_component]
pub fn HierarchyPanel() -> Html {
    let state = use_context::<UseReducerHandle<ViewerState>>().unwrap();
    let entities = use_state(Vec::<EntityNode>::new);
    let search_query = use_state(String::new);
    let scroll_ref = use_node_ref();

    // Virtual scrolling state
    let virtual_items = use_virtual_scroll(
        entities.len(),
        36,  // item height
        scroll_ref.clone(),
    );

    let filtered_entities: Vec<_> = entities.iter()
        .filter(|e| {
            search_query.is_empty() ||
            e.name.to_lowercase().contains(&search_query.to_lowercase())
        })
        .collect();

    html! {
        <div class="hierarchy-panel">
            <div class="search-bar">
                <input
                    type="text"
                    placeholder="Search entities..."
                    value={(*search_query).clone()}
                    oninput={Callback::from(move |e: InputEvent| {
                        let input: HtmlInputElement = e.target_unchecked_into();
                        search_query.set(input.value());
                    })}
                />
            </div>

            <div ref={scroll_ref} class="hierarchy-list">
                <div style={format!("height: {}px", virtual_items.total_height)}>
                    { for virtual_items.visible_items.iter().map(|idx| {
                        let entity = &filtered_entities[*idx];
                        html! {
                            <EntityRow
                                key={entity.id}
                                entity={entity.clone()}
                                selected={state.selected_ids.contains(&entity.id)}
                                style={format!("top: {}px", idx * 36)}
                            />
                        }
                    })}
                </div>
            </div>
        </div>
    }
}
```

---

### Phase 4: Feature Parity (Week 6-8)

#### 4.1 Measurement Tool
```rust
// overlays/measure_tool.rs
#[function_component]
pub fn MeasureTool() -> Html {
    let state = use_context::<UseReducerHandle<ViewerState>>().unwrap();

    html! {
        <div class="measure-panel">
            <h3>{"Measurements"}</h3>

            { for state.measurements.iter().enumerate().map(|(i, m)| {
                let distance = m.start.distance(m.end);
                html! {
                    <div class="measurement-item">
                        <span>{format!("{:.2}m", distance)}</span>
                        <button onclick={
                            let state = state.clone();
                            Callback::from(move |_| {
                                state.dispatch(ViewerAction::RemoveMeasurement(i));
                            })
                        }>{"×"}</button>
                    </div>
                }
            })}

            if !state.measurements.is_empty() {
                <div class="measurement-total">
                    <span>{"Total: "}</span>
                    <span>{format!("{:.2}m", state.measurements.iter()
                        .map(|m| m.start.distance(m.end))
                        .sum::<f32>()
                    )}</span>
                </div>
                <button onclick={
                    let state = state.clone();
                    Callback::from(move |_| {
                        state.dispatch(ViewerAction::ClearMeasurements);
                    })
                }>{"Clear All"}</button>
            }
        </div>
    }
}
```

#### 4.2 ViewCube Widget
```rust
// view_cube.rs
#[function_component]
pub fn ViewCube() -> Html {
    let rotation = use_state(|| (0.0f32, 0.0f32));

    // Poll camera rotation from Bevy via localStorage
    {
        let rotation = rotation.clone();
        use_interval(move || {
            if let Some(rot) = get_camera_rotation() {
                rotation.set(rot);
            }
        }, 100);
    }

    let faces = [
        ("top", "Top", (0.0, -90.0)),
        ("bottom", "Bottom", (0.0, 90.0)),
        ("front", "Front", (0.0, 0.0)),
        ("back", "Back", (180.0, 0.0)),
        ("left", "Left", (-90.0, 0.0)),
        ("right", "Right", (90.0, 0.0)),
    ];

    html! {
        <div
            class="view-cube"
            style={format!(
                "transform: rotateX({}deg) rotateY({}deg)",
                rotation.1.to_degrees(),
                rotation.0.to_degrees()
            )}
        >
            { for faces.iter().map(|(class, label, target)| {
                html! {
                    <div
                        class={format!("view-cube-face {}", class)}
                        onclick={
                            let target = *target;
                            Callback::from(move |_| {
                                set_camera_preset(target.0, target.1);
                            })
                        }
                    >
                        {label}
                    </div>
                }
            })}
        </div>
    }
}
```

#### 4.3 Keyboard Shortcuts
```rust
// hooks/use_keyboard.rs
#[hook]
pub fn use_keyboard_shortcuts() {
    let state = use_context::<UseReducerHandle<ViewerState>>().unwrap();

    use_effect_with(state.clone(), |state| {
        let state = state.clone();
        let listener = EventListener::new(&document(), "keydown", move |event| {
            let event: KeyboardEvent = event.clone().unchecked_into();

            match event.key().as_str() {
                "v" | "V" => state.dispatch(ViewerAction::SetActiveTool(Tool::Select)),
                "p" | "P" => state.dispatch(ViewerAction::SetActiveTool(Tool::Pan)),
                "o" | "O" => state.dispatch(ViewerAction::SetActiveTool(Tool::Orbit)),
                "c" | "C" => state.dispatch(ViewerAction::SetActiveTool(Tool::Walk)),
                "m" | "M" => state.dispatch(ViewerAction::SetActiveTool(Tool::Measure)),
                "x" | "X" => state.dispatch(ViewerAction::SetActiveTool(Tool::Section)),
                "b" | "B" => state.dispatch(ViewerAction::SetActiveTool(Tool::BoxSelect)),
                "i" | "I" => isolate_selection(&state),
                "Delete" => hide_selection(&state),
                "a" | "A" => state.dispatch(ViewerAction::ShowAll),
                "h" | "H" => set_camera_preset(45.0, 35.264),  // Isometric
                "z" | "Z" => fit_all(),
                "f" | "F" => frame_selection(&state),
                "t" | "T" => state.dispatch(ViewerAction::ToggleTheme),
                "1" => set_camera_preset(0.0, 0.0),    // Front
                "2" => set_camera_preset(180.0, 0.0),  // Back
                "3" => set_camera_preset(-90.0, 0.0),  // Left
                "4" => set_camera_preset(90.0, 0.0),   // Right
                "5" => set_camera_preset(0.0, -90.0),  // Top
                "6" => set_camera_preset(0.0, 90.0),   // Bottom
                _ => {}
            }
        });

        move || drop(listener)
    });
}
```

---

### Phase 5: Export & Polish (Week 8-10)

#### 5.1 Export Functions
```rust
// export.rs
pub fn export_glb(scene_data: &IfcSceneData) -> Result<Vec<u8>, ExportError> {
    // Use gltf crate to build GLB
    let mut root = gltf_json::Root::default();

    for mesh in &scene_data.meshes {
        // Add mesh data to GLB
    }

    gltf::binary::write(&root)
}

pub fn export_csv_entities(entities: &[EntityInfo]) -> String {
    let mut csv = String::from("ID,Type,Name,Storey\n");
    for entity in entities {
        csv.push_str(&format!(
            "{},{},{},{}\n",
            entity.id,
            entity.entity_type,
            entity.name.as_deref().unwrap_or(""),
            entity.storey.as_deref().unwrap_or("")
        ));
    }
    csv
}

pub fn capture_screenshot() -> Result<Vec<u8>, ScreenshotError> {
    // Read pixels from Bevy's render target
    // Encode as PNG
}
```

#### 5.2 Styling (CSS)
```css
/* styles.css - Dark theme (matching gldf-rs) */
:root {
    --bg-primary: #1e1e1e;
    --bg-secondary: #252526;
    --bg-card: #2d2d2d;
    --bg-card-hover: #363636;
    --bg-input: #3c3c3c;
    --accent-blue: #0a84ff;
    --text-primary: #ffffff;
    --text-secondary: #8e8e93;
    --border-color: #3d3d3d;
}

.viewer-layout {
    display: grid;
    grid-template-columns: auto 1fr auto;
    height: 100vh;
    background: var(--bg-primary);
}

.panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    overflow: hidden;
}

.panel-left { width: 280px; }
.panel-right { width: 320px; }

.viewport-container {
    display: flex;
    flex-direction: column;
    position: relative;
}

.viewport-canvas {
    flex: 1;
    width: 100%;
    height: 100%;
}

/* Mobile responsive */
@media (max-width: 768px) {
    .viewer-layout {
        grid-template-columns: 1fr;
        grid-template-rows: 1fr auto;
    }

    .panel-left, .panel-right {
        position: fixed;
        bottom: 0;
        left: 0;
        right: 0;
        height: 50vh;
        width: 100%;
        z-index: 100;
    }
}
```

---

## Migration Checklist

### Core Infrastructure
- [ ] Create crate structure (ifc-lite-bevy, ifc-lite-yew, ifc-lite-viewer)
- [ ] Set up Trunk build configuration
- [ ] Implement localStorage bridge (Bevy ↔ Yew)
- [ ] Create bevy-loader.js for lazy loading
- [ ] Set up coordinate system conversion (IFC Z-up → Bevy Y-up)

### Bevy Renderer
- [ ] Mesh spawning from IFC geometry
- [ ] PBR materials with entity colors
- [ ] Orbit camera controller with damping
- [ ] Pan camera mode
- [ ] Walk/first-person camera mode
- [ ] Zoom (wheel + pinch)
- [ ] GPU-based picking (raycasting)
- [ ] Selection highlighting
- [ ] Section plane clipping shader
- [ ] Animated camera transitions
- [ ] Frustum culling
- [ ] Progressive mesh loading

### Yew UI
- [ ] ViewerLayout (3-panel responsive)
- [ ] Toolbar with tool buttons
- [ ] HierarchyPanel with virtual scrolling
- [ ] PropertiesPanel with tabs
- [ ] ViewCube widget
- [ ] StatusBar (FPS, memory, counts)
- [ ] ContextMenu (right-click)
- [ ] MeasureTool overlay
- [ ] SectionTool overlay
- [ ] BoxSelection overlay
- [ ] HoverTooltip
- [ ] KeyboardShortcutsDialog
- [ ] Theme toggle (dark/light)
- [ ] File input handling

### Features
- [ ] Single-click selection
- [ ] Multi-selection (Ctrl+click)
- [ ] Box selection
- [ ] Hide/Isolate/Show all
- [ ] Storey filtering
- [ ] Measurement tool
- [ ] Section plane tool
- [ ] Keyboard shortcuts
- [ ] GLB export
- [ ] CSV export
- [ ] Screenshot capture
- [ ] Mobile responsive layout

### Integration
- [ ] IFC file loading via existing WASM parser
- [ ] Geometry streaming with progress
- [ ] Property/quantity extraction
- [ ] Spatial hierarchy building
- [ ] Hot reload support (timestamp polling)

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Bevy WASM performance | High | Profile early, use LOD, frustum culling |
| WebGPU vs WebGL2 compatibility | Medium | Default to WebGL2, feature-detect WebGPU |
| localStorage size limits (~5MB) | Medium | Use IndexedDB for large files |
| Touch input complexity | Medium | Use proven patterns from gldf-rs |
| Virtual scrolling in Yew | Low | Port TanStack Virtual logic |

---

## Success Criteria

1. **Feature Parity**: All current viewer features working
2. **Performance**: First geometry visible in <500ms for typical models
3. **Bundle Size**: <300KB total WASM (parser + geometry + renderer + UI)
4. **Compatibility**: Works on Chrome, Firefox, Safari (WebGL2)
5. **Code Reuse**: Share ≥50% of IFC code with gldf-rs

---

## Next Steps

1. **Approve this plan** and allocate resources
2. **Phase 1**: Set up project structure and bridge pattern
3. **Prototype**: Get basic mesh rendering working in Bevy
4. **Iterate**: Add features incrementally, testing each phase
5. **Integration**: Connect to gldf-rs for shared IFC functionality
