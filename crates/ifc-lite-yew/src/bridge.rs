//! Bridge between Yew UI and Bevy renderer
//!
//! Handles data transfer via localStorage and JavaScript FFI.

use serde::{Deserialize, Serialize};
use serde_json;
use wasm_bindgen::prelude::*;

/// Storage keys (must match ifc-lite-bevy)
pub const GEOMETRY_KEY: &str = "ifc_lite_geometry";
pub const ENTITIES_KEY: &str = "ifc_lite_entities";
pub const SELECTION_KEY: &str = "ifc_lite_selection";
pub const VISIBILITY_KEY: &str = "ifc_lite_visibility";
pub const CAMERA_KEY: &str = "ifc_lite_camera";
pub const TIMESTAMP_KEY: &str = "ifc_lite_timestamp";
pub const SECTION_KEY: &str = "ifc_lite_section";
pub const FOCUS_KEY: &str = "ifc_lite_focus";
pub const CAMERA_CMD_KEY: &str = "ifc_lite_camera_cmd";

// JavaScript FFI functions
#[wasm_bindgen]
extern "C" {
    /// Load the Bevy viewer module
    #[wasm_bindgen(js_name = loadBevyViewer, catch)]
    pub async fn load_bevy_viewer() -> Result<(), JsValue>;

    /// Check if Bevy is loaded
    #[wasm_bindgen(js_name = isBevyLoaded)]
    pub fn is_bevy_loaded() -> bool;

    /// Check if Bevy is currently loading
    #[wasm_bindgen(js_name = isBevyLoading)]
    pub fn is_bevy_loading() -> bool;

    /// Set geometry data via JS bridge (avoids localStorage limit)
    #[wasm_bindgen(js_name = setIfcGeometry)]
    pub fn set_ifc_geometry(json: &str);

    /// Set entity data via JS bridge
    #[wasm_bindgen(js_name = setIfcEntities)]
    pub fn set_ifc_entities(json: &str);
}

/// Get localStorage
fn get_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// Update timestamp to trigger Bevy reload
pub fn update_timestamp() {
    if let Some(storage) = get_storage() {
        let ts = js_sys::Date::now().to_string();
        let _ = storage.set_item(TIMESTAMP_KEY, &ts);
    }
}

/// Geometry data for Bevy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeometryData {
    pub entity_id: u64,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
    pub transform: [f32; 16],
    pub entity_type: String,
    pub name: Option<String>,
}

/// Entity data for Bevy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityData {
    pub id: u64,
    pub entity_type: String,
    pub name: Option<String>,
    pub storey: Option<String>,
    pub storey_elevation: Option<f32>,
}

/// Selection state for storage
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SelectionData {
    pub selected_ids: Vec<u64>,
    pub hovered_id: Option<u64>,
}

/// Visibility state for storage
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VisibilityData {
    pub hidden: Vec<u64>,
    pub isolated: Option<Vec<u64>>,
}

/// Camera state for storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraData {
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
    pub target: [f32; 3],
}

/// Section plane state for storage
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SectionData {
    pub enabled: bool,
    pub axis: String,
    pub position: f32,
    pub flipped: bool,
}

/// Focus command for zooming to entity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FocusData {
    /// Entity ID to focus on (zoom to)
    pub entity_id: u64,
}

/// Camera command for view controls
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraCommand {
    /// Command type: "home", "fit_all", "set_mode"
    pub cmd: String,
    /// Optional mode for set_mode: "orbit", "pan", "walk"
    pub mode: Option<String>,
}

/// Save geometry data for Bevy (uses JS bridge to avoid localStorage limits)
pub fn save_geometry(geometry: &[GeometryData]) {
    match serde_json::to_string(geometry) {
        Ok(json) => {
            log(&format!("[Yew] Geometry JSON size: {} bytes", json.len()));
            // Use JS bridge instead of localStorage to handle large data
            set_ifc_geometry(&json);
            log("[Yew] Geometry sent via JS bridge");
        }
        Err(e) => {
            log_error(&format!("[Yew] Failed to serialize geometry: {:?}", e));
        }
    }
}

/// Save entity data for Bevy (uses JS bridge)
pub fn save_entities(entities: &[EntityData]) {
    if let Ok(json) = serde_json::to_string(entities) {
        set_ifc_entities(&json);
    }
}

/// Save selection state for Bevy
pub fn save_selection(selection: &SelectionData) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(selection) {
            let _ = storage.set_item(SELECTION_KEY, &json);
            update_timestamp();
        }
    }
}

/// Load selection state from Bevy
pub fn load_selection() -> Option<SelectionData> {
    let storage = get_storage()?;
    let json = storage.get_item(SELECTION_KEY).ok()??;
    serde_json::from_str(&json).ok()
}

/// Save visibility state for Bevy
pub fn save_visibility(visibility: &VisibilityData) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(visibility) {
            let _ = storage.set_item(VISIBILITY_KEY, &json);
            update_timestamp();
        }
    }
}

/// Load camera state from Bevy
pub fn load_camera() -> Option<CameraData> {
    let storage = get_storage()?;
    let json = storage.get_item(CAMERA_KEY).ok()??;
    serde_json::from_str(&json).ok()
}

/// Save section plane state for Bevy
pub fn save_section(section: &SectionData) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(section) {
            let _ = storage.set_item(SECTION_KEY, &json);
            update_timestamp();
        }
    }
}

/// Save focus command for Bevy (zoom to entity)
pub fn save_focus(focus: &FocusData) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(focus) {
            let _ = storage.set_item(FOCUS_KEY, &json);
            update_timestamp();
        }
    }
}

/// Save camera command for Bevy (home, fit_all, set_mode)
pub fn save_camera_cmd(cmd: &CameraCommand) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(cmd) {
            let _ = storage.set_item(CAMERA_CMD_KEY, &json);
            update_timestamp();
        }
    }
}

/// Clear all storage
pub fn clear_storage() {
    if let Some(storage) = get_storage() {
        let _ = storage.remove_item(GEOMETRY_KEY);
        let _ = storage.remove_item(ENTITIES_KEY);
        let _ = storage.remove_item(SELECTION_KEY);
        let _ = storage.remove_item(VISIBILITY_KEY);
        let _ = storage.remove_item(SECTION_KEY);
        let _ = storage.remove_item(FOCUS_KEY);
        update_timestamp();
    }
}

/// Log to browser console
pub fn log(msg: &str) {
    web_sys::console::log_1(&msg.into());
}

/// Log error to browser console
pub fn log_error(msg: &str) {
    web_sys::console::error_1(&msg.into());
}
