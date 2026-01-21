//! Storage types and localStorage bridge
//!
//! In unified mode (bevy-ui): No external JS bridge needed, files loaded directly
//! In external-ui mode: Uses localStorage/JS bridge to communicate with Yew

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// JavaScript FFI to get geometry from JS bridge (set by Yew)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    /// Get timestamp from JS bridge
    #[wasm_bindgen(js_name = getIfcTimestamp)]
    fn js_get_ifc_timestamp() -> Option<String>;

    /// Get geometry binary from JS bridge
    #[wasm_bindgen(js_name = getIfcGeometryBinary)]
    fn js_get_ifc_geometry_binary() -> Option<js_sys::Uint8Array>;

    /// Get entities JSON from JS bridge
    #[wasm_bindgen(js_name = getIfcEntities)]
    fn js_get_ifc_entities() -> Option<String>;

    /// Clear geometry from JS bridge to free memory
    #[wasm_bindgen(js_name = clearIfcGeometryBridge)]
    fn js_clear_ifc_geometry_bridge();
}

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
// JS Bridge functions - used in unified Yew+Bevy mode
// ============================================================================

/// Get timestamp from JS bridge
#[cfg(target_arch = "wasm32")]
pub fn get_timestamp() -> Option<String> {
    js_get_ifc_timestamp()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_timestamp() -> Option<String> {
    None
}

/// Binary format magic number (must match bridge.rs)
const BINARY_MAGIC: u32 = 0x49464342; // "IFCB"

/// Read f32 values from unaligned byte slice
#[cfg(target_arch = "wasm32")]
fn read_f32_vec(data: &[u8], offset: &mut usize, count: usize) -> Option<Vec<f32>> {
    let bytes_needed = count * 4;
    if *offset + bytes_needed > data.len() {
        return None;
    }
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let bytes: [u8; 4] = data[*offset..*offset + 4].try_into().ok()?;
        result.push(f32::from_le_bytes(bytes));
        *offset += 4;
    }
    Some(result)
}

/// Read u32 values from unaligned byte slice
#[cfg(target_arch = "wasm32")]
fn read_u32_vec(data: &[u8], offset: &mut usize, count: usize) -> Option<Vec<u32>> {
    let bytes_needed = count * 4;
    if *offset + bytes_needed > data.len() {
        return None;
    }
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let bytes: [u8; 4] = data[*offset..*offset + 4].try_into().ok()?;
        result.push(u32::from_le_bytes(bytes));
        *offset += 4;
    }
    Some(result)
}

/// Deserialize geometry from binary format
#[cfg(target_arch = "wasm32")]
fn deserialize_geometry_binary(data: &[u8]) -> Option<Vec<crate::IfcMesh>> {
    use crate::mesh::MeshGeometry;
    use std::sync::Arc;

    if data.len() < 12 {
        return None;
    }

    let mut offset = 0;

    // Read header
    let magic = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?);
    offset += 4;
    if magic != BINARY_MAGIC {
        web_sys::console::error_1(&format!("[Bevy] Invalid geometry magic: {:08x}", magic).into());
        return None;
    }

    let _version = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?);
    offset += 4;

    let mesh_count = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
    offset += 4;

    let mut meshes = Vec::with_capacity(mesh_count);

    for _ in 0..mesh_count {
        if offset + 8 > data.len() {
            break;
        }

        // entity_id
        let entity_id = u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?);
        offset += 8;

        // positions
        let positions_len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;
        if offset + positions_len * 4 > data.len() {
            break;
        }
        let positions = read_f32_vec(data, &mut offset, positions_len)?;

        // normals
        let normals_len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;
        if offset + normals_len * 4 > data.len() {
            break;
        }
        let normals = read_f32_vec(data, &mut offset, normals_len)?;

        // indices
        let indices_len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;
        if offset + indices_len * 4 > data.len() {
            break;
        }
        let indices = read_u32_vec(data, &mut offset, indices_len)?;

        // color (4 floats)
        if offset + 16 > data.len() {
            break;
        }
        let color_vec = read_f32_vec(data, &mut offset, 4)?;
        let color: [f32; 4] = [color_vec[0], color_vec[1], color_vec[2], color_vec[3]];

        // transform (16 floats)
        if offset + 64 > data.len() {
            break;
        }
        let transform_vec = read_f32_vec(data, &mut offset, 16)?;
        let transform: [f32; 16] = transform_vec.try_into().ok()?;

        // entity_type
        if offset >= data.len() {
            break;
        }
        let type_len = data[offset] as usize;
        offset += 1;
        if offset + type_len > data.len() {
            break;
        }
        let entity_type = String::from_utf8_lossy(&data[offset..offset + type_len]).to_string();
        offset += type_len;

        // name
        if offset >= data.len() {
            break;
        }
        let name_len = data[offset] as usize;
        offset += 1;
        let name = if name_len > 0 && offset + name_len <= data.len() {
            let n = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string();
            offset += name_len;
            Some(n)
        } else {
            None
        };

        meshes.push(crate::IfcMesh {
            entity_id,
            geometry: Arc::new(MeshGeometry {
                positions,
                normals,
                indices,
            }),
            color,
            transform,
            entity_type,
            name,
        });
    }

    Some(meshes)
}

/// Load geometry from JS bridge
#[cfg(target_arch = "wasm32")]
pub fn load_geometry() -> Option<Vec<crate::IfcMesh>> {
    let uint8_array = js_get_ifc_geometry_binary()?;
    let data = uint8_array.to_vec();
    web_sys::console::log_1(&format!("[Bevy] Loading geometry from JS bridge: {} bytes", data.len()).into());
    let meshes = deserialize_geometry_binary(&data)?;
    web_sys::console::log_1(&format!("[Bevy] Deserialized {} meshes", meshes.len()).into());
    // Clear the JS bridge to free memory
    js_clear_ifc_geometry_bridge();
    Some(meshes)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_geometry() -> Option<Vec<crate::IfcMesh>> {
    None
}

/// Load entities from JS bridge
#[cfg(target_arch = "wasm32")]
pub fn load_entities() -> Option<Vec<crate::EntityInfo>> {
    let json = js_get_ifc_entities()?;
    serde_json::from_str(&json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_entities() -> Option<Vec<crate::EntityInfo>> {
    None
}

/// Load selection from localStorage
#[cfg(target_arch = "wasm32")]
pub fn load_selection() -> Option<SelectionStorage> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item("ifc_lite_selection").ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_selection() -> Option<SelectionStorage> {
    None
}

/// Save selection to localStorage
#[cfg(target_arch = "wasm32")]
pub fn save_selection(selection: &SelectionStorage) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(selection) {
                let _ = storage.set_item("ifc_lite_selection", &json);
                // Mark source as "bevy" so Yew knows to pick up this change
                let _ = storage.set_item("ifc_lite_selection_source", "bevy");
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_selection(_selection: &SelectionStorage) {}

/// Get selection source from localStorage
#[cfg(target_arch = "wasm32")]
pub fn get_selection_source() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage.get_item("ifc_lite_selection_source").ok()?
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_selection_source() -> Option<String> {
    None
}

/// Load visibility from localStorage
#[cfg(target_arch = "wasm32")]
pub fn load_visibility() -> Option<VisibilityStorage> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item("ifc_lite_visibility").ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_visibility() -> Option<VisibilityStorage> {
    None
}

/// Load camera from localStorage
#[cfg(target_arch = "wasm32")]
pub fn load_camera() -> Option<CameraStorage> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item("ifc_lite_camera").ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_camera() -> Option<CameraStorage> {
    None
}

/// Save camera to localStorage
#[cfg(target_arch = "wasm32")]
pub fn save_camera(camera: &CameraStorage) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(camera) {
                let _ = storage.set_item("ifc_lite_camera", &json);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_camera(_camera: &CameraStorage) {}

/// Load section plane from localStorage
#[cfg(target_arch = "wasm32")]
pub fn load_section() -> Option<SectionStorage> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item("ifc_lite_section").ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_section() -> Option<SectionStorage> {
    None
}

/// Load focus command from localStorage
#[cfg(target_arch = "wasm32")]
pub fn load_focus() -> Option<FocusStorage> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item("ifc_lite_focus").ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_focus() -> Option<FocusStorage> {
    None
}

/// Clear focus command
#[cfg(target_arch = "wasm32")]
pub fn clear_focus() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item("ifc_lite_focus");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_focus() {}

/// Load camera command from localStorage
#[cfg(target_arch = "wasm32")]
pub fn load_camera_cmd() -> Option<CameraCommandStorage> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item("ifc_lite_camera_cmd").ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_camera_cmd() -> Option<CameraCommandStorage> {
    None
}

/// Clear camera command
#[cfg(target_arch = "wasm32")]
pub fn clear_camera_cmd() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item("ifc_lite_camera_cmd");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_camera_cmd() {}

/// Load palette from localStorage
#[cfg(target_arch = "wasm32")]
pub fn load_palette() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage.get_item("ifc_lite_palette").ok()?
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_palette() -> Option<String> {
    None
}

/// Clear palette
#[cfg(target_arch = "wasm32")]
pub fn clear_palette() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item("ifc_lite_palette");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_palette() {}
