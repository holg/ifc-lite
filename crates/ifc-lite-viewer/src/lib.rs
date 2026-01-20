//! IFC-Lite Unified Viewer
//!
//! Single WASM containing both Yew (UI) and Bevy (3D renderer).
//! Data flows directly through Rust memory - no JS bridge serialization needed!

use wasm_bindgen::prelude::*;
use yew::prelude::*;
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Shared geometry data channel for direct Yew->Bevy transfer
/// This avoids serialization/deserialization overhead of the JS bridge
pub mod shared {
    use super::*;
    use ifc_lite_bevy::{IfcMesh, EntityInfo, MeshGeometry};
    use ifc_lite_yew::bridge::{GeometryData, EntityData};
    use std::sync::Arc;

    /// Pending scene data to be picked up by Bevy
    pub struct PendingSceneData {
        pub meshes: Vec<IfcMesh>,
        pub entities: Vec<EntityInfo>,
        pub timestamp: String,
    }

    /// Global channel for geometry data
    static PENDING_DATA: Lazy<Mutex<Option<PendingSceneData>>> = Lazy::new(|| Mutex::new(None));

    /// Convert GeometryData (Yew) to IfcMesh (Bevy) - zero-copy for the geometry!
    fn convert_geometry(g: GeometryData) -> IfcMesh {
        IfcMesh {
            entity_id: g.entity_id,
            geometry: Arc::new(MeshGeometry::new(g.positions, g.normals, g.indices)),
            color: g.color,
            transform: g.transform,
            entity_type: g.entity_type,
            name: g.name,
        }
    }

    /// Convert EntityData (Yew) to EntityInfo (Bevy)
    fn convert_entity(e: EntityData) -> EntityInfo {
        EntityInfo {
            id: e.id,
            entity_type: e.entity_type,
            name: e.name,
            storey: e.storey,
            storey_elevation: e.storey_elevation,
        }
    }

    /// Set geometry data from Yew using Yew's types (called after parsing)
    /// This performs direct memory transfer - no serialization!
    pub fn set_scene_data_from_yew(geometry: Vec<GeometryData>, entities: Vec<EntityData>) {
        let timestamp = js_sys::Date::now().to_string();
        web_sys::console::log_1(&format!(
            "[Unified] Direct transfer: {} meshes, {} entities (no serialization!)",
            geometry.len(), entities.len()
        ).into());

        // Convert directly - moves ownership, no serialization
        let meshes: Vec<IfcMesh> = geometry.into_iter().map(convert_geometry).collect();
        let entity_infos: Vec<EntityInfo> = entities.into_iter().map(convert_entity).collect();

        let mut guard = PENDING_DATA.lock().unwrap();
        *guard = Some(PendingSceneData {
            meshes,
            entities: entity_infos,
            timestamp
        });
    }

    /// Take geometry data for Bevy (consumes the data)
    pub fn take_scene_data() -> Option<PendingSceneData> {
        let mut guard = PENDING_DATA.lock().unwrap();
        guard.take()
    }

    /// Check if new data is available
    pub fn has_pending_data() -> bool {
        let guard = PENDING_DATA.lock().unwrap();
        guard.is_some()
    }

    /// Get the current timestamp (for Bevy polling)
    pub fn get_timestamp() -> Option<String> {
        let guard = PENDING_DATA.lock().unwrap();
        guard.as_ref().map(|d| d.timestamp.clone())
    }
}

/// Main application component
#[function_component]
fn App() -> Html {
    html! {
        <ifc_lite_yew::ViewerLayout />
    }
}

/// WASM entry point - starts Yew UI
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"[Unified] Starting IFC-Lite Viewer".into());

    // Register the startBevyUnified function on window so Yew can call it
    register_bevy_starter();

    // Initialize debug mode
    ifc_lite_yew::bridge::init_debug_from_url();

    // Start Yew
    yew::Renderer::<App>::new().render();
}

/// Register the Bevy starter function on window.startBevyUnified
fn register_bevy_starter() {
    use wasm_bindgen::closure::Closure;

    let closure = Closure::wrap(Box::new(|canvas: String| {
        web_sys::console::log_1(&format!("[Unified] startBevyUnified called with: {}", canvas).into());
        ifc_lite_bevy::run_on_canvas(&canvas);
    }) as Box<dyn Fn(String)>);

    let window = web_sys::window().expect("no window");
    js_sys::Reflect::set(
        &window,
        &"startBevyUnified".into(),
        closure.as_ref(),
    ).expect("failed to set startBevyUnified");

    // Leak the closure so it lives forever
    closure.forget();

    web_sys::console::log_1(&"[Unified] Registered window.startBevyUnified".into());
}

/// Start Bevy renderer on canvas - called from JS after geometry is ready
#[wasm_bindgen(js_name = startBevyUnified)]
pub fn start_bevy_unified(canvas_selector: &str) {
    web_sys::console::log_1(&format!("[Unified] Starting Bevy on {}", canvas_selector).into());
    ifc_lite_bevy::run_on_canvas(canvas_selector);
}

/// Check if we're in unified mode - always true for this build
#[wasm_bindgen(js_name = isUnifiedMode)]
pub fn is_unified_mode() -> bool {
    true
}
