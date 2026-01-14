//! localStorage bridge for Bevy-Yew communication
//!
//! This module handles data transfer between Yew UI and Bevy renderer
//! using localStorage as an intermediary (proven pattern from gldf-rs).

use crate::{EntityInfo, IfcMesh};
use serde::{Deserialize, Serialize};

/// Storage keys for localStorage
pub const GEOMETRY_KEY: &str = "ifc_lite_geometry";
pub const ENTITIES_KEY: &str = "ifc_lite_entities";
pub const SELECTION_KEY: &str = "ifc_lite_selection";
pub const VISIBILITY_KEY: &str = "ifc_lite_visibility";
pub const CAMERA_KEY: &str = "ifc_lite_camera";
pub const TIMESTAMP_KEY: &str = "ifc_lite_timestamp";
pub const SECTION_KEY: &str = "ifc_lite_section";
pub const FOCUS_KEY: &str = "ifc_lite_focus";
pub const CAMERA_CMD_KEY: &str = "ifc_lite_camera_cmd";

/// Selection state for storage
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SelectionStorage {
    pub selected_ids: Vec<u64>,
    pub hovered_id: Option<u64>,
}

/// Visibility state for storage
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VisibilityStorage {
    pub hidden: Vec<u64>,
    pub isolated: Option<Vec<u64>>,
}

/// Camera state for storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraStorage {
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
    pub target: [f32; 3],
}

impl Default for CameraStorage {
    fn default() -> Self {
        Self {
            azimuth: 0.785, // 45 degrees
            elevation: 0.615, // ~35 degrees (isometric)
            distance: 10.0,
            target: [0.0, 0.0, 0.0],
        }
    }
}

/// Section plane state for storage
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SectionStorage {
    pub enabled: bool,
    pub axis: String, // "x", "y", or "z"
    pub position: f32, // 0.0 to 1.0
    pub flipped: bool,
}

/// Focus command for zooming to entity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FocusStorage {
    pub entity_id: u64,
}

/// Camera command from UI
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraCommandStorage {
    pub cmd: String,
    pub mode: Option<String>,
}

// ============================================================================
// WASM Storage Functions
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm_storage {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = getIfcGeometry)]
        fn get_ifc_geometry() -> Option<String>;

        #[wasm_bindgen(js_name = getIfcEntities)]
        fn get_ifc_entities() -> Option<String>;

        #[wasm_bindgen(js_name = getIfcTimestamp)]
        fn get_ifc_timestamp() -> String;
    }

    fn get_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn get_timestamp() -> Option<String> {
        let ts = get_ifc_timestamp();
        if ts.is_empty() { None } else { Some(ts) }
    }

    pub fn load_geometry() -> Option<Vec<IfcMesh>> {
        let json = match get_ifc_geometry() {
            Some(j) if !j.is_empty() => j,
            _ => {
                crate::log("[Bevy] No geometry in JS bridge");
                return None;
            }
        };
        crate::log(&format!("[Bevy] Geometry JSON size: {} bytes", json.len()));
        match serde_json::from_str(&json) {
            Ok(meshes) => Some(meshes),
            Err(e) => {
                crate::log(&format!("[Bevy] Error parsing geometry JSON: {:?}", e));
                None
            }
        }
    }

    pub fn load_entities() -> Option<Vec<EntityInfo>> {
        let json = get_ifc_entities()?;
        serde_json::from_str(&json).ok()
    }

    pub fn load_selection() -> Option<SelectionStorage> {
        let storage = get_storage()?;
        let json = storage.get_item(SELECTION_KEY).ok()??;
        serde_json::from_str(&json).ok()
    }

    pub fn save_selection(selection: &SelectionStorage) {
        if let Some(storage) = get_storage() {
            if let Ok(json) = serde_json::to_string(selection) {
                let _ = storage.set_item(SELECTION_KEY, &json);
                update_timestamp();
            }
        }
    }

    pub fn load_visibility() -> Option<VisibilityStorage> {
        let storage = get_storage()?;
        let json = storage.get_item(VISIBILITY_KEY).ok()??;
        serde_json::from_str(&json).ok()
    }

    pub fn load_camera() -> Option<CameraStorage> {
        let storage = get_storage()?;
        let json = storage.get_item(CAMERA_KEY).ok()??;
        serde_json::from_str(&json).ok()
    }

    pub fn save_camera(camera: &CameraStorage) {
        if let Some(storage) = get_storage() {
            if let Ok(json) = serde_json::to_string(camera) {
                let _ = storage.set_item(CAMERA_KEY, &json);
                // Don't update timestamp for camera - too frequent
            }
        }
    }

    pub fn load_section() -> Option<SectionStorage> {
        let storage = get_storage()?;
        let json = storage.get_item(SECTION_KEY).ok()??;
        serde_json::from_str(&json).ok()
    }

    pub fn load_focus() -> Option<FocusStorage> {
        let storage = get_storage()?;
        let json = storage.get_item(FOCUS_KEY).ok()??;
        serde_json::from_str(&json).ok()
    }

    pub fn clear_focus() {
        if let Some(storage) = get_storage() {
            let _ = storage.remove_item(FOCUS_KEY);
        }
    }

    pub fn load_camera_cmd() -> Option<CameraCommandStorage> {
        let storage = get_storage()?;
        let json = storage.get_item(CAMERA_CMD_KEY).ok()??;
        serde_json::from_str(&json).ok()
    }

    pub fn clear_camera_cmd() {
        if let Some(storage) = get_storage() {
            let _ = storage.remove_item(CAMERA_CMD_KEY);
        }
    }

    fn update_timestamp() {
        if let Some(storage) = get_storage() {
            let ts = js_sys::Date::now().to_string();
            let _ = storage.set_item(TIMESTAMP_KEY, &ts);
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_storage::*;

// ============================================================================
// Native (no-op) Storage Functions
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod native_storage {
    use super::*;

    pub fn get_timestamp() -> Option<String> {
        None
    }

    pub fn load_geometry() -> Option<Vec<IfcMesh>> {
        None
    }

    pub fn load_entities() -> Option<Vec<EntityInfo>> {
        None
    }

    pub fn load_selection() -> Option<SelectionStorage> {
        None
    }

    pub fn save_selection(_selection: &SelectionStorage) {}

    pub fn load_visibility() -> Option<VisibilityStorage> {
        None
    }

    pub fn load_camera() -> Option<CameraStorage> {
        None
    }

    pub fn save_camera(_camera: &CameraStorage) {}

    pub fn load_section() -> Option<SectionStorage> {
        None
    }

    pub fn load_focus() -> Option<FocusStorage> {
        None
    }

    pub fn clear_focus() {}

    pub fn load_camera_cmd() -> Option<CameraCommandStorage> {
        None
    }

    pub fn clear_camera_cmd() {}
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_storage::*;
