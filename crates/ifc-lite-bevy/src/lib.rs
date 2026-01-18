//! IFC-Lite Bevy 3D Viewer
//!
//! Bevy-based 3D viewer for IFC models with WebGPU/WebGL2 rendering.
//! Supports orbit/pan/zoom camera controls, entity selection, and section planes.
//!
//! Features pure Bevy UI that works on both web (WASM) and native platforms.

// Allow unexpected_cfgs from objc crate's msg_send! macro used in native_view
#![allow(unexpected_cfgs)]

pub mod camera;
pub mod loader;
pub mod mesh;
pub mod picking;
pub mod section;
pub mod storage;

#[cfg(feature = "bevy-ui")]
pub mod ui;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod native_view;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod ffi;

use bevy::prelude::*;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Global debug mode flag (set from URL parameter ?debug=1)
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

/// Check if debug mode is enabled
pub fn is_debug() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

/// Initialize debug mode from URL parameters
#[cfg(target_arch = "wasm32")]
fn init_debug_from_url() {
    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            let search_str: &str = &search;
            if search_str.contains("debug=1") || search_str.contains("debug=true") {
                DEBUG_MODE.store(true, Ordering::Relaxed);
                web_sys::console::log_1(&"[Bevy] Debug mode enabled".into());
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn init_debug_from_url() {
    // Native: check env var
    if std::env::var("DEBUG").is_ok() {
        DEBUG_MODE.store(true, Ordering::Relaxed);
    }
}

// Re-exports
pub use camera::{CameraController, CameraMode, CameraPlugin};
pub use loader::{LoadIfcFileEvent, LoaderPlugin, OpenFileDialogRequest};
pub use mesh::{AutoFitState, IfcEntity, IfcMesh, IfcMeshSerialized, MeshGeometry, MeshPlugin};
pub use picking::{PickingPlugin, SelectionState};
pub use section::{SectionPlane, SectionPlanePlugin};
pub use storage::*;

#[cfg(feature = "bevy-ui")]
pub use ui::{IfcUiPlugin, UiState};

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub use native_view::{AppView, AppViewPlugin, AppViews};

/// Main IFC viewer plugin - combines all subsystems
pub struct IfcViewerPlugin;

impl Plugin for IfcViewerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IfcSceneData>()
            .init_resource::<ViewerSettings>()
            .init_resource::<IfcTimestamp>()
            .add_plugins((
                CameraPlugin,
                MeshPlugin,
                PickingPlugin,
                SectionPlanePlugin,
                LoaderPlugin,
            ))
            .add_systems(Update, poll_scene_changes);

        // Add Bevy UI when feature is enabled
        #[cfg(feature = "bevy-ui")]
        app.add_plugins(IfcUiPlugin);
    }
}

/// Resource containing all IFC scene data
#[derive(Resource, Default)]
pub struct IfcSceneData {
    /// All meshes in the scene
    pub meshes: Vec<IfcMesh>,
    /// Entity metadata (type, name, properties)
    pub entities: Vec<EntityInfo>,
    /// Scene bounds (AABB)
    pub bounds: Option<SceneBounds>,
    /// Data timestamp for change detection
    pub timestamp: u64,
    /// Whether scene needs rebuild
    pub dirty: bool,
}

/// Entity metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: u64,
    pub entity_type: String,
    pub name: Option<String>,
    pub storey: Option<String>,
    pub storey_elevation: Option<f32>,
}

/// Axis-aligned bounding box for scene
#[derive(Clone, Debug, Default)]
pub struct SceneBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl SceneBounds {
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn diagonal(&self) -> f32 {
        self.size().length()
    }
}

/// Viewer settings and state
#[derive(Resource)]
pub struct ViewerSettings {
    /// Current theme (affects background color)
    pub theme: Theme,
    /// Show grid
    pub show_grid: bool,
    /// Show axes helper
    pub show_axes: bool,
    /// Hidden entity IDs
    pub hidden_entities: FxHashSet<u64>,
    /// Isolated entity IDs (if Some, only show these)
    pub isolated_entities: Option<FxHashSet<u64>>,
    /// Active storey filter
    pub storey_filter: Option<String>,
}

impl Default for ViewerSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            show_grid: true,
            show_axes: true,
            hidden_entities: FxHashSet::default(),
            isolated_entities: None,
            storey_filter: None,
        }
    }
}

/// Theme variants
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

impl Theme {
    pub fn background_color(&self) -> Color {
        match self {
            Theme::Light => Color::srgb(0.95, 0.95, 0.95),
            Theme::Dark => Color::srgb(0.12, 0.12, 0.12),
        }
    }

    pub fn grid_color(&self) -> Color {
        match self {
            Theme::Light => Color::srgba(0.5, 0.5, 0.5, 0.3),
            Theme::Dark => Color::srgba(0.4, 0.4, 0.4, 0.3),
        }
    }
}

/// Timestamp for detecting localStorage changes (WASM)
#[derive(Resource, Default)]
pub struct IfcTimestamp(pub String);

/// System to poll localStorage for scene changes (WASM)
#[allow(unused_variables, unused_mut)]
pub fn poll_scene_changes(
    mut scene_data: ResMut<IfcSceneData>,
    mut settings: ResMut<ViewerSettings>,
    mut last_timestamp: ResMut<IfcTimestamp>,
    mut auto_fit: ResMut<mesh::AutoFitState>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(new_timestamp) = storage::get_timestamp() {
            if new_timestamp != last_timestamp.0 {
                log(&format!(
                    "[Bevy] Timestamp changed: {} -> {}",
                    last_timestamp.0, new_timestamp
                ));

                // Load geometry from storage
                if let Some(geometry) = storage::load_geometry() {
                    log(&format!("[Bevy] Loaded {} meshes", geometry.len()));
                    scene_data.meshes = geometry;
                    scene_data.dirty = true;
                    // Reset auto-fit state to trigger camera fit for new scene
                    auto_fit.has_fit = false;
                }

                // Load entities from storage
                if let Some(entities) = storage::load_entities() {
                    log(&format!("[Bevy] Loaded {} entities", entities.len()));
                    scene_data.entities = entities;
                }

                // Load selection state
                if let Some(selection) = storage::load_selection() {
                    // Selection is handled by PickingPlugin
                }

                // Load visibility state
                if let Some(visibility) = storage::load_visibility() {
                    settings.hidden_entities = visibility.hidden.into_iter().collect();
                    settings.isolated_entities =
                        visibility.isolated.map(|v| v.into_iter().collect());
                }

                last_timestamp.0 = new_timestamp;
            }
        }
    }
}

/// Log to browser console (WASM) or stdout (native) - only in debug mode
#[cfg(target_arch = "wasm32")]
pub fn log(msg: &str) {
    if is_debug() {
        web_sys::console::log_1(&msg.into());
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log(msg: &str) {
    if is_debug() {
        println!("{}", msg);
    }
}

/// Log info that should always be shown
#[cfg(target_arch = "wasm32")]
pub fn log_info(msg: &str) {
    web_sys::console::info_1(&msg.into());
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log_info(msg: &str) {
    println!("{}", msg);
}

/// Run the viewer on a canvas element (WASM)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_on_canvas(canvas_selector: &str) {
    console_error_panic_hook::set_once();
    init_debug_from_url();
    log(&format!("[Bevy] Starting on canvas: {}", canvas_selector));

    // Load initial data from localStorage
    let meshes = storage::load_geometry().unwrap_or_default();
    let entities = storage::load_entities().unwrap_or_default();

    log(&format!(
        "[Bevy] Initial load - {} meshes, {} entities",
        meshes.len(),
        entities.len()
    ));

    let scene_data = IfcSceneData {
        meshes,
        entities,
        bounds: None,
        timestamp: 0,
        dirty: true,
    };

    let mut app = App::new();

    // Insert resources before plugins
    app.insert_resource(scene_data);
    app.insert_resource(ViewerSettings::default());
    app.insert_resource(IfcTimestamp::default());

    // Add plugins
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "IFC-Lite Viewer".to_string(),
            canvas: Some(canvas_selector.to_string()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: false,
            ..default()
        }),
        ..default()
    }));

    app.add_plugins(IfcViewerPlugin);
    app.run();
}

/// Run the viewer in a native window (desktop)
#[cfg(not(target_arch = "wasm32"))]
pub fn run_on_canvas(_canvas_selector: &str) {
    run_native();
}

/// Run native desktop viewer
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "IFC-Lite Viewer".to_string(),
                resolution: (1280u32, 720u32).into(),
                ..default()
            }),
            ..default()
        }))
        // Dark gray background so we can see if rendering works
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.15)))
        .add_plugins(IfcViewerPlugin)
        .run();
}

#[cfg(target_arch = "wasm32")]
pub fn run_native() {
    run_on_canvas("#bevy-canvas");
}

/// WASM entry point
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn wasm_start() {
    log("[Bevy] wasm_start called");
    run_native();
}
