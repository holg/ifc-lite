//! Bridge between Yew UI and Bevy renderer
//!
//! Handles data transfer via localStorage and JavaScript FFI.
//! Uses binary format for geometry data to reduce memory usage and improve performance.

use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::prelude::*;

/// Global debug mode flag (set from URL parameter ?debug=1)
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

/// Check if debug mode is enabled
pub fn is_debug() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

/// Initialize debug mode from URL parameters
/// Call this once at startup
pub fn init_debug_from_url() {
    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            if search.contains("debug=1") || search.contains("debug=true") {
                DEBUG_MODE.store(true, Ordering::Relaxed);
                // Always log this one
                web_sys::console::log_1(&"[IFC-Lite] Debug mode enabled via URL".into());
            }
        }
    }
}

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

    /// Set geometry data via JS bridge (binary format)
    #[wasm_bindgen(js_name = setIfcGeometryBinary)]
    pub fn set_ifc_geometry_binary(data: &Uint8Array);

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

/// Binary format header magic number
const BINARY_MAGIC: u32 = 0x49464342; // "IFCB" in ASCII

/// Serialize geometry data to compact binary format
/// Format:
/// - u32: magic (0x49464342 = "IFCB")
/// - u32: version (1)
/// - u32: mesh_count
/// - For each mesh:
///   - u64: entity_id
///   - u32: positions_len (number of f32s)
///   - f32[]: positions
///   - u32: normals_len
///   - f32[]: normals
///   - u32: indices_len
///   - u32[]: indices
///   - f32[4]: color
///   - f32[16]: transform
///   - u8: entity_type_len
///   - utf8[]: entity_type
///   - u8: name_len (0 if None)
///   - utf8[]: name (if any)
fn serialize_geometry_binary(geometry: &[GeometryData]) -> Vec<u8> {
    // Estimate capacity: header + meshes
    let estimated_size: usize = 12
        + geometry
            .iter()
            .map(|g| {
                8 + 4
                    + g.positions.len() * 4
                    + 4
                    + g.normals.len() * 4
                    + 4
                    + g.indices.len() * 4
                    + 16
                    + 64
                    + 1
                    + g.entity_type.len()
                    + 1
                    + g.name.as_ref().map(|n| n.len()).unwrap_or(0)
            })
            .sum::<usize>();

    let mut buf = Vec::with_capacity(estimated_size);

    // Header
    buf.extend_from_slice(&BINARY_MAGIC.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes()); // version
    buf.extend_from_slice(&(geometry.len() as u32).to_le_bytes());

    for mesh in geometry {
        // entity_id
        buf.extend_from_slice(&mesh.entity_id.to_le_bytes());

        // positions
        buf.extend_from_slice(&(mesh.positions.len() as u32).to_le_bytes());
        for &p in &mesh.positions {
            buf.extend_from_slice(&p.to_le_bytes());
        }

        // normals
        buf.extend_from_slice(&(mesh.normals.len() as u32).to_le_bytes());
        for &n in &mesh.normals {
            buf.extend_from_slice(&n.to_le_bytes());
        }

        // indices
        buf.extend_from_slice(&(mesh.indices.len() as u32).to_le_bytes());
        for &i in &mesh.indices {
            buf.extend_from_slice(&i.to_le_bytes());
        }

        // color
        for &c in &mesh.color {
            buf.extend_from_slice(&c.to_le_bytes());
        }

        // transform
        for &t in &mesh.transform {
            buf.extend_from_slice(&t.to_le_bytes());
        }

        // entity_type
        let type_bytes = mesh.entity_type.as_bytes();
        buf.push(type_bytes.len() as u8);
        buf.extend_from_slice(type_bytes);

        // name
        if let Some(ref name) = mesh.name {
            let name_bytes = name.as_bytes();
            buf.push(name_bytes.len().min(255) as u8);
            buf.extend_from_slice(&name_bytes[..name_bytes.len().min(255)]);
        } else {
            buf.push(0);
        }
    }

    buf
}

/// Save geometry data for Bevy (uses binary format via JS bridge)
pub fn save_geometry(geometry: &[GeometryData]) {
    let binary = serialize_geometry_binary(geometry);
    log(&format!(
        "[Yew] Geometry binary size: {} bytes ({} meshes)",
        binary.len(),
        geometry.len()
    ));

    // Create Uint8Array and copy data
    let array = Uint8Array::new_with_length(binary.len() as u32);
    array.copy_from(&binary);

    set_ifc_geometry_binary(&array);
    log("[Yew] Geometry sent via JS bridge (binary)");
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

/// Log to browser console (only in debug mode)
pub fn log(msg: &str) {
    if is_debug() {
        web_sys::console::log_1(&msg.into());
    }
}

/// Log error to browser console (always shown)
pub fn log_error(msg: &str) {
    web_sys::console::error_1(&msg.into());
}

/// Log warning to browser console (always shown)
pub fn log_warn(msg: &str) {
    web_sys::console::warn_1(&msg.into());
}

/// Log info that should always be shown (e.g., load complete)
pub fn log_info(msg: &str) {
    web_sys::console::info_1(&msg.into());
}
