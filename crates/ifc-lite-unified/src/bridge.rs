//! Direct bridge between Yew and Bevy
//!
//! In unified mode, Yew and Bevy share the same WASM memory space.
//! Data is passed directly via Rust structures - no JSON serialization needed.

use crate::SHARED_STATE;
use ifc_lite_bevy::{EntityInfo, IfcMesh, IfcSceneData};

/// Pass geometry data directly from Yew to shared state
/// This is called after IFC parsing completes
pub fn set_geometry_direct(meshes: Vec<IfcMesh>, entities: Vec<EntityInfo>) {
    web_sys::console::log_1(
        &format!(
            "[Bridge] Setting geometry: {} meshes, {} entities",
            meshes.len(),
            entities.len()
        )
        .into(),
    );

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

/// Get scene data from shared state (called by Bevy)
pub fn take_scene_data() -> Option<IfcSceneData> {
    let mut state = SHARED_STATE.lock().unwrap();
    if state.data_ready {
        state.data_ready = false;
        state.scene_data.take()
    } else {
        None
    }
}

/// Update selection (from either Yew or Bevy)
pub fn set_selection(selected_ids: Vec<u64>, hovered_id: Option<u64>) {
    let mut state = SHARED_STATE.lock().unwrap();
    state.selected_ids = selected_ids;
    state.hovered_id = hovered_id;
}

/// Get current selection
pub fn get_selection() -> (Vec<u64>, Option<u64>) {
    let state = SHARED_STATE.lock().unwrap();
    (state.selected_ids.clone(), state.hovered_id)
}

/// Update visibility
pub fn set_visibility(hidden_ids: Vec<u64>, isolated_ids: Option<Vec<u64>>) {
    let mut state = SHARED_STATE.lock().unwrap();
    state.hidden_ids = hidden_ids;
    state.isolated_ids = isolated_ids;
}

/// Send camera command
pub fn send_camera_cmd(cmd: &str) {
    let mut state = SHARED_STATE.lock().unwrap();
    state.camera_cmd = Some(cmd.to_string());
}

/// Take camera command (consumed by Bevy)
pub fn take_camera_cmd() -> Option<String> {
    let mut state = SHARED_STATE.lock().unwrap();
    state.camera_cmd.take()
}
