//! localStorage bridge for Bevy-Yew communication
//!
//! This module handles data transfer between Yew UI and Bevy renderer
//! using localStorage as an intermediary (proven pattern from gldf-rs).
//! Geometry data uses binary format for efficiency.

use crate::{EntityInfo, IfcMesh};
use serde::{Deserialize, Serialize};

/// Binary format header magic number
#[allow(dead_code)]
const BINARY_MAGIC: u32 = 0x49464342; // "IFCB" in ASCII

/// Storage keys for localStorage
pub const GEOMETRY_KEY: &str = "ifc_lite_geometry";
pub const ENTITIES_KEY: &str = "ifc_lite_entities";
pub const SELECTION_KEY: &str = "ifc_lite_selection";
pub const SELECTION_SOURCE_KEY: &str = "ifc_lite_selection_source";
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
            azimuth: 0.785,   // 45 degrees
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
    pub axis: String,  // "x", "y", or "z"
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
    use js_sys::Uint8Array;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = getIfcGeometryBinary)]
        fn get_ifc_geometry_binary() -> Option<Uint8Array>;

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
        if ts.is_empty() {
            None
        } else {
            Some(ts)
        }
    }

    /// Deserialize geometry from binary format
    fn deserialize_geometry_binary(data: &[u8]) -> Option<Vec<IfcMesh>> {
        let mut cursor = 0;

        // Helper to read bytes
        macro_rules! read_bytes {
            ($n:expr) => {{
                if cursor + $n > data.len() {
                    crate::log("[Bevy] Binary data truncated");
                    return None;
                }
                let slice = &data[cursor..cursor + $n];
                cursor += $n;
                slice
            }};
        }

        // Read header
        let magic = u32::from_le_bytes(read_bytes!(4).try_into().ok()?);
        if magic != BINARY_MAGIC {
            crate::log(&format!("[Bevy] Invalid magic: {:08x}", magic));
            return None;
        }

        let version = u32::from_le_bytes(read_bytes!(4).try_into().ok()?);
        if version != 1 {
            crate::log(&format!("[Bevy] Unsupported version: {}", version));
            return None;
        }

        let mesh_count = u32::from_le_bytes(read_bytes!(4).try_into().ok()?) as usize;
        crate::log(&format!("[Bevy] Parsing {} meshes from binary", mesh_count));

        let mut meshes = Vec::with_capacity(mesh_count);

        for _ in 0..mesh_count {
            // entity_id
            let entity_id = u64::from_le_bytes(read_bytes!(8).try_into().ok()?);

            // positions
            let positions_len = u32::from_le_bytes(read_bytes!(4).try_into().ok()?) as usize;
            let mut positions = Vec::with_capacity(positions_len);
            for _ in 0..positions_len {
                positions.push(f32::from_le_bytes(read_bytes!(4).try_into().ok()?));
            }

            // normals
            let normals_len = u32::from_le_bytes(read_bytes!(4).try_into().ok()?) as usize;
            let mut normals = Vec::with_capacity(normals_len);
            for _ in 0..normals_len {
                normals.push(f32::from_le_bytes(read_bytes!(4).try_into().ok()?));
            }

            // indices
            let indices_len = u32::from_le_bytes(read_bytes!(4).try_into().ok()?) as usize;
            let mut indices = Vec::with_capacity(indices_len);
            for _ in 0..indices_len {
                indices.push(u32::from_le_bytes(read_bytes!(4).try_into().ok()?));
            }

            // color
            let mut color = [0.0f32; 4];
            for c in &mut color {
                *c = f32::from_le_bytes(read_bytes!(4).try_into().ok()?);
            }

            // transform
            let mut transform = [0.0f32; 16];
            for t in &mut transform {
                *t = f32::from_le_bytes(read_bytes!(4).try_into().ok()?);
            }

            // entity_type
            let type_len = read_bytes!(1)[0] as usize;
            let entity_type = String::from_utf8_lossy(read_bytes!(type_len)).to_string();

            // name
            let name_len = read_bytes!(1)[0] as usize;
            let name = if name_len > 0 {
                Some(String::from_utf8_lossy(read_bytes!(name_len)).to_string())
            } else {
                None
            };

            meshes.push(IfcMesh {
                entity_id,
                positions,
                normals,
                indices,
                color,
                transform,
                entity_type,
                name,
            });
        }

        Some(meshes)
    }

    pub fn load_geometry() -> Option<Vec<IfcMesh>> {
        let array = match get_ifc_geometry_binary() {
            Some(a) if a.length() > 0 => a,
            _ => {
                crate::log("[Bevy] No geometry in JS bridge");
                return None;
            }
        };

        crate::log(&format!(
            "[Bevy] Geometry binary size: {} bytes",
            array.length()
        ));

        // Copy to Vec<u8>
        let data = array.to_vec();
        deserialize_geometry_binary(&data)
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
                let _ = storage.set_item(SELECTION_SOURCE_KEY, "bevy");
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
