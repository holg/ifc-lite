//! Unified IFC-Lite Viewer - Yew UI + Bevy 3D in single WASM
//!
//! This crate combines Yew (UI) and Bevy (3D renderer) into a single WASM module.
//! Communication happens directly via shared Rust structures - no JS bridge needed.

use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

pub mod bridge;

use ifc_lite_bevy::{EntityInfo, IfcMesh, IfcMeshSerialized, IfcSceneData};

/// Shared state between Yew and Bevy - direct memory access
pub static SHARED_STATE: Lazy<Arc<Mutex<SharedState>>> =
    Lazy::new(|| Arc::new(Mutex::new(SharedState::default())));

/// Shared state structure - Yew writes, Bevy reads
#[derive(Default)]
pub struct SharedState {
    /// Scene data to render (set by Yew after parsing)
    pub scene_data: Option<IfcSceneData>,
    /// Flag indicating new data is available
    pub data_ready: bool,
    /// Selection state
    pub selected_ids: Vec<u64>,
    pub hovered_id: Option<u64>,
    /// Visibility
    pub hidden_ids: Vec<u64>,
    pub isolated_ids: Option<Vec<u64>>,
    /// Camera commands
    pub camera_cmd: Option<String>,
    /// Section plane
    pub section_enabled: bool,
    pub section_axis: String,
    pub section_position: f32,
    pub section_flipped: bool,
}

/// WASM entry point - starts both Yew UI and Bevy renderer
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"[Unified] Starting IFC-Lite Unified Viewer".into());

    // Start Yew UI
    yew::Renderer::<ifc_lite_yew::components::ViewerLayout>::new().render();

    // Note: Bevy will be started when user loads a file
    // This avoids blocking the UI while Bevy initializes
}

/// Start Bevy renderer on a canvas element
/// Called from Yew after geometry is ready
#[wasm_bindgen]
pub fn start_bevy_renderer(canvas_selector: &str) {
    web_sys::console::log_1(&format!("[Unified] Starting Bevy on {}", canvas_selector).into());

    // Take scene data from shared state (moves ownership, avoids clone)
    let scene_data = {
        let mut state = SHARED_STATE.lock().unwrap();
        state.scene_data.take().unwrap_or_default()
    };

    // Run Bevy with the scene data
    ifc_lite_bevy::run_with_data(canvas_selector, scene_data);
}

/// Set scene data from Yew (after parsing IFC file)
#[wasm_bindgen]
pub fn set_scene_data(meshes_json: &str, entities_json: &str) {
    web_sys::console::log_1(&"[Unified] Setting scene data from Yew".into());

    // Parse JSON (temporary - will optimize later)
    // Use IfcMeshSerialized for deserialization, then convert to IfcMesh
    let meshes: Vec<IfcMesh> = match serde_json::from_str::<Vec<IfcMeshSerialized>>(meshes_json) {
        Ok(serialized) => serialized.into_iter().map(IfcMesh::from).collect(),
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to parse meshes: {}", e).into());
            return;
        }
    };

    let entities: Vec<EntityInfo> = match serde_json::from_str(entities_json) {
        Ok(e) => e,
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to parse entities: {}", e).into());
            return;
        }
    };

    web_sys::console::log_1(
        &format!(
            "[Unified] Received {} meshes, {} entities",
            meshes.len(),
            entities.len()
        )
        .into(),
    );

    // Store in shared state
    let mut state = SHARED_STATE.lock().unwrap();
    state.scene_data = Some(IfcSceneData {
        meshes,
        entities,
        bounds: None,
        timestamp: 0,
        dirty: true,
    });
    state.data_ready = true;
}

/// Check if Bevy is ready to receive data
#[wasm_bindgen]
pub fn is_bevy_ready() -> bool {
    // For now, always ready - Bevy will poll shared state
    true
}
