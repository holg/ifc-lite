//! Bridge between Leptos UI and Bevy renderer
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
pub const SELECTION_SOURCE_KEY: &str = "ifc_lite_selection_source";
pub const SECTION_KEY: &str = "ifc_lite_section";
pub const FOCUS_KEY: &str = "ifc_lite_focus";
pub const CAMERA_CMD_KEY: &str = "ifc_lite_camera_cmd";
pub const PALETTE_KEY: &str = "ifc_lite_palette";

// JavaScript FFI functions (used in split mode when Bevy is separate WASM)
#[wasm_bindgen]
extern "C" {
    /// Load the Bevy viewer module
    #[wasm_bindgen(js_name = loadBevyViewer, catch)]
    async fn js_load_bevy_viewer() -> Result<(), JsValue>;

    /// Check if Bevy is loaded
    #[wasm_bindgen(js_name = isBevyLoaded)]
    fn js_is_bevy_loaded() -> bool;

    /// Check if Bevy is currently loading
    #[wasm_bindgen(js_name = isBevyLoading)]
    fn js_is_bevy_loading() -> bool;

    /// Set geometry data via JS bridge (binary format)
    #[wasm_bindgen(js_name = setIfcGeometryBinary)]
    fn js_set_ifc_geometry_binary(data: &Uint8Array);

    /// Append geometry data to existing (for streaming)
    #[wasm_bindgen(js_name = appendIfcGeometryBinary)]
    fn js_append_ifc_geometry_binary(data: &Uint8Array);

    /// Signal that geometry streaming is complete
    #[wasm_bindgen(js_name = finalizeIfcGeometry)]
    fn js_finalize_ifc_geometry();

    /// Set entity data via JS bridge
    #[wasm_bindgen(js_name = setIfcEntities)]
    fn js_set_ifc_entities(json: &str);

    /// Check if unified mode (Bevy in same WASM) - defined in index.html
    #[wasm_bindgen(js_name = isUnifiedMode)]
    fn js_is_unified_mode() -> bool;

    /// Start Bevy in unified mode
    #[wasm_bindgen(js_name = startBevyUnified)]
    fn js_start_bevy_unified(canvas: &str);
}

/// Check if running in unified mode (Bevy in same WASM)
pub fn is_unified_mode() -> bool {
    // Check if the JS function exists - if not, we're in split mode
    js_sys::Reflect::get(&js_sys::global(), &"isUnifiedMode".into())
        .map(|v| v.is_function())
        .unwrap_or(false)
        && js_is_unified_mode()
}

/// Load Bevy viewer - works in both unified and split mode
pub async fn load_bevy_viewer() -> Result<(), JsValue> {
    if is_unified_mode() {
        // In unified mode, Bevy is already loaded - just start it
        log_info("[Leptos] Unified mode - starting Bevy on #bevy-canvas");
        js_start_bevy_unified("#bevy-canvas");
        log_info("[Leptos] Bevy start call returned");
        Ok(())
    } else {
        // Split mode - load separate WASM
        js_load_bevy_viewer().await
    }
}

/// Check if Bevy is loaded
pub fn is_bevy_loaded() -> bool {
    // Always use the JS function - it tracks whether Bevy has actually started
    js_is_bevy_loaded()
}

/// Check if Bevy is loading
pub fn is_bevy_loading() -> bool {
    if is_unified_mode() {
        false
    } else {
        js_is_bevy_loading()
    }
}

/// Set geometry binary
pub fn set_ifc_geometry_binary(data: &Uint8Array) {
    js_set_ifc_geometry_binary(data);
}

/// Append geometry binary
pub fn append_ifc_geometry_binary(data: &Uint8Array) {
    js_append_ifc_geometry_binary(data);
}

/// Finalize geometry
pub fn finalize_ifc_geometry() {
    js_finalize_ifc_geometry();
}

/// Set entities JSON
pub fn set_ifc_entities(json: &str) {
    js_set_ifc_entities(json);
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
    /// Description attribute (index 3) - often more human-readable than Name
    pub description: Option<String>,
    pub global_id: Option<String>,
    pub storey: Option<String>,
    pub storey_elevation: Option<f32>,
}

impl EntityData {
    /// Get display label for tree view: prefer description, then name, then type#id
    pub fn display_label(&self) -> String {
        if let Some(ref desc) = self.description {
            if !desc.is_empty() && desc != "$" {
                return desc.clone();
            }
        }
        if let Some(ref name) = self.name {
            if !name.is_empty() && name != "$" {
                return name.clone();
            }
        }
        format!("#{}", self.id)
    }
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

/// Color palette for IFC visualization (local definition to avoid circular deps)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum ColorPalette {
    #[default]
    Vibrant,
    Realistic,
    HighContrast,
    Monochrome,
}

impl ColorPalette {
    pub fn name(&self) -> &'static str {
        match self {
            ColorPalette::Vibrant => "Vibrant",
            ColorPalette::Realistic => "Realistic",
            ColorPalette::HighContrast => "High Contrast",
            ColorPalette::Monochrome => "Monochrome",
        }
    }

    pub fn next(&self) -> ColorPalette {
        match self {
            ColorPalette::Vibrant => ColorPalette::Realistic,
            ColorPalette::Realistic => ColorPalette::HighContrast,
            ColorPalette::HighContrast => ColorPalette::Monochrome,
            ColorPalette::Monochrome => ColorPalette::Vibrant,
        }
    }
}

/// Binary format header magic number
#[allow(dead_code)]
const BINARY_MAGIC: u32 = 0x49464342; // "IFCB" in ASCII

/// Serialize geometry data to compact binary format
#[allow(dead_code)]
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

        // positions - bulk copy using bytemuck
        buf.extend_from_slice(&(mesh.positions.len() as u32).to_le_bytes());
        buf.extend_from_slice(bytemuck::cast_slice(&mesh.positions));

        // normals - bulk copy
        buf.extend_from_slice(&(mesh.normals.len() as u32).to_le_bytes());
        buf.extend_from_slice(bytemuck::cast_slice(&mesh.normals));

        // indices - bulk copy
        buf.extend_from_slice(&(mesh.indices.len() as u32).to_le_bytes());
        buf.extend_from_slice(bytemuck::cast_slice(&mesh.indices));

        // color - bulk copy
        buf.extend_from_slice(bytemuck::cast_slice(&mesh.color));

        // transform - bulk copy
        buf.extend_from_slice(bytemuck::cast_slice(&mesh.transform));

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

/// Save geometry data for Bevy
/// In unified mode: direct memory transfer (no serialization!)
/// In split mode: binary serialization via JS bridge
pub fn save_geometry(geometry: Vec<GeometryData>) {
    let start = js_sys::Date::now();
    let mesh_count = geometry.len();

    #[cfg(feature = "unified")]
    {
        // UNIFIED MODE: Direct memory transfer - no serialization!
        // Convert GeometryData -> IfcMesh and pass directly
        use ifc_lite_bevy::{IfcMesh, MeshGeometry};
        use std::sync::Arc;

        let meshes: Vec<IfcMesh> = geometry
            .into_iter()
            .map(|g| IfcMesh {
                entity_id: g.entity_id,
                geometry: Arc::new(MeshGeometry::new(g.positions, g.normals, g.indices)),
                color: g.color,
                transform: g.transform,
                entity_type: g.entity_type,
                name: g.name,
            })
            .collect();

        // Store in global for Bevy to pick up
        ifc_lite_bevy::set_pending_meshes(meshes);

        let total_time = js_sys::Date::now() - start;
        log_info(&format!(
            "[Leptos] Geometry direct transfer: {} meshes in {:.0}ms (no serialization!)",
            mesh_count, total_time
        ));
    }

    #[cfg(not(feature = "unified"))]
    {
        // SPLIT MODE: Binary serialization via JS bridge
        let serialize_start = js_sys::Date::now();
        let binary = serialize_geometry_binary(&geometry);
        let serialize_time = js_sys::Date::now() - serialize_start;
        let size = binary.len();

        log_info(&format!(
            "[Leptos] Geometry serialized: {} bytes ({} meshes) in {:.0}ms",
            size, mesh_count, serialize_time
        ));

        // Create Uint8Array and copy data
        let copy_start = js_sys::Date::now();
        let array = Uint8Array::new_with_length(size as u32);
        array.copy_from(&binary);
        let copy_time = js_sys::Date::now() - copy_start;

        // Send to JS bridge
        let bridge_start = js_sys::Date::now();
        set_ifc_geometry_binary(&array);
        let bridge_time = js_sys::Date::now() - bridge_start;

        let total_time = js_sys::Date::now() - start;
        log_info(&format!(
            "[Leptos] Geometry bridge: {:.0}ms total (serialize: {:.0}ms, copy: {:.0}ms, bridge: {:.0}ms) | {:.1} MB",
            total_time, serialize_time, copy_time, bridge_time,
            size as f64 / (1024.0 * 1024.0)
        ));
    }
}

/// Save entity data for Bevy (uses JS bridge)
pub fn save_entities(entities: &[EntityData]) {
    let start = js_sys::Date::now();
    if let Ok(json) = serde_json::to_string(entities) {
        let serialize_time = js_sys::Date::now() - start;
        set_ifc_entities(&json);
        let total_time = js_sys::Date::now() - start;
        log_info(&format!(
            "[Leptos] Entities bridge: {:.0}ms total (serialize: {:.0}ms) | {} entities, {:.1} KB JSON",
            total_time, serialize_time, entities.len(), json.len() as f64 / 1024.0
        ));
    }
}

/// Save selection state for Bevy (marks source as "leptos")
pub fn save_selection(selection: &SelectionData) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(selection) {
            let _ = storage.set_item(SELECTION_KEY, &json);
            let _ = storage.set_item(SELECTION_SOURCE_KEY, "leptos");
            update_timestamp();
        }
    }
}

/// Get the source of the last selection change ("leptos" or "bevy")
pub fn get_selection_source() -> Option<String> {
    let storage = get_storage()?;
    storage.get_item(SELECTION_SOURCE_KEY).ok()?
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

/// Save palette command for Bevy (to recolor meshes)
pub fn save_palette(palette: ColorPalette) {
    if let Some(storage) = get_storage() {
        // Send palette name as string for Bevy to interpret
        let palette_str = match palette {
            ColorPalette::Vibrant => "vibrant",
            ColorPalette::Realistic => "realistic",
            ColorPalette::HighContrast => "high_contrast",
            ColorPalette::Monochrome => "monochrome",
        };
        let _ = storage.set_item(PALETTE_KEY, palette_str);
        update_timestamp();
    }
}

// ============================================================================
// Model Caching - localStorage-based cache for parsed IFC data
// ============================================================================

const CACHE_PREFIX: &str = "ifc_cache_";
const CACHE_INDEX_KEY: &str = "ifc_cache_index";
const MAX_CACHE_ENTRIES: usize = 5; // Keep last 5 models cached

/// Cache entry metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_hash: String,
    pub file_name: String,
    pub entity_count: usize,
    pub geometry_count: usize,
    pub timestamp: f64,
}

/// Cache index (list of cached models)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheIndex {
    pub entries: Vec<CacheEntry>,
}

/// Cached model data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedModel {
    pub geometry: Vec<GeometryData>,
    pub entities: Vec<EntityData>,
    pub spatial_tree_json: Option<String>,
    pub storeys_json: Option<String>,
}

/// Compute a simple hash of file content for cache key
pub fn compute_file_hash(content: &str) -> String {
    // Simple hash: use length + first/last chars + sample chars
    let len = content.len();
    let first = content.chars().take(100).collect::<String>();
    let last = content.chars().rev().take(100).collect::<String>();

    // Sample some chars from middle
    let mid_start = len / 2;
    let middle: String = content.chars().skip(mid_start).take(100).collect();

    // Create a hash string
    format!("{:x}_{:x}_{:x}",
        first.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64)),
        middle.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64)),
        last.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
    )
}

/// Get cache index
fn get_cache_index() -> Option<CacheIndex> {
    let storage = get_storage()?;
    let json = storage.get_item(CACHE_INDEX_KEY).ok()??;
    serde_json::from_str(&json).ok()
}

/// Save cache index
fn save_cache_index(index: &CacheIndex) {
    if let Some(storage) = get_storage() {
        if let Ok(json) = serde_json::to_string(index) {
            let _ = storage.set_item(CACHE_INDEX_KEY, &json);
        }
    }
}

/// Check if a model is cached
pub fn is_model_cached(file_hash: &str) -> bool {
    get_cache_index()
        .map(|idx| idx.entries.iter().any(|e| e.file_hash == file_hash))
        .unwrap_or(false)
}

/// Load cached model
pub fn load_cached_model(file_hash: &str) -> Option<CachedModel> {
    let storage = get_storage()?;
    let key = format!("{}{}", CACHE_PREFIX, file_hash);
    let json = storage.get_item(&key).ok()??;

    // Update access timestamp in index
    if let Some(mut index) = get_cache_index() {
        if let Some(entry) = index.entries.iter_mut().find(|e| e.file_hash == file_hash) {
            entry.timestamp = js_sys::Date::now();
            save_cache_index(&index);
        }
    }

    log_info(&format!("[Cache] Loading cached model: {}", file_hash));
    serde_json::from_str(&json).ok()
}

/// Save model to cache
pub fn save_model_to_cache(
    file_hash: &str,
    file_name: &str,
    model: &CachedModel,
) {
    let storage = match get_storage() {
        Some(s) => s,
        None => return,
    };

    // Serialize model
    let json = match serde_json::to_string(model) {
        Ok(j) => j,
        Err(e) => {
            log_warn(&format!("[Cache] Failed to serialize model: {}", e));
            return;
        }
    };

    // Check size - localStorage has ~5MB limit per key, be conservative
    let size_mb = json.len() as f64 / (1024.0 * 1024.0);
    if size_mb > 4.0 {
        log_warn(&format!("[Cache] Model too large to cache: {:.2}MB", size_mb));
        return;
    }

    // Save model data
    let key = format!("{}{}", CACHE_PREFIX, file_hash);
    if storage.set_item(&key, &json).is_err() {
        log_warn("[Cache] Failed to save model - storage may be full");
        // Try to clear old cache entries and retry
        clear_oldest_cache_entry();
        if storage.set_item(&key, &json).is_err() {
            return;
        }
    }

    // Update cache index
    let mut index = get_cache_index().unwrap_or_default();

    // Remove existing entry for this hash if present
    index.entries.retain(|e| e.file_hash != file_hash);

    // Add new entry
    index.entries.push(CacheEntry {
        file_hash: file_hash.to_string(),
        file_name: file_name.to_string(),
        entity_count: model.entities.len(),
        geometry_count: model.geometry.len(),
        timestamp: js_sys::Date::now(),
    });

    // Enforce max cache size
    while index.entries.len() > MAX_CACHE_ENTRIES {
        // Remove oldest entry
        if let Some(oldest) = index.entries.iter().min_by(|a, b| {
            a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal)
        }).cloned() {
            let old_key = format!("{}{}", CACHE_PREFIX, oldest.file_hash);
            let _ = storage.remove_item(&old_key);
            index.entries.retain(|e| e.file_hash != oldest.file_hash);
        }
    }

    save_cache_index(&index);
    log_info(&format!("[Cache] Model cached: {} ({:.2}MB)", file_name, size_mb));
}

/// Clear oldest cache entry to make room
fn clear_oldest_cache_entry() {
    if let Some(storage) = get_storage() {
        if let Some(mut index) = get_cache_index() {
            if let Some(oldest) = index.entries.iter().min_by(|a, b| {
                a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal)
            }).cloned() {
                let key = format!("{}{}", CACHE_PREFIX, oldest.file_hash);
                let _ = storage.remove_item(&key);
                index.entries.retain(|e| e.file_hash != oldest.file_hash);
                save_cache_index(&index);
                log_info(&format!("[Cache] Cleared old entry: {}", oldest.file_name));
            }
        }
    }
}

/// Clear all cached models
pub fn clear_model_cache() {
    if let Some(storage) = get_storage() {
        if let Some(index) = get_cache_index() {
            for entry in &index.entries {
                let key = format!("{}{}", CACHE_PREFIX, entry.file_hash);
                let _ = storage.remove_item(&key);
            }
        }
        let _ = storage.remove_item(CACHE_INDEX_KEY);
        log_info("[Cache] All cached models cleared");
    }
}
