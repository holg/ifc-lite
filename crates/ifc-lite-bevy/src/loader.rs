//! IFC file loading - handles file dialog, drag-and-drop, and WASM file input

use crate::mesh::IfcMesh;
use crate::{EntityInfo, IfcSceneData};
use bevy::prelude::*;
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "ios"),
    not(target_os = "macos")
))]
use bevy::tasks::IoTaskPool;
use bevy::tasks::Task;
use ifc_lite_geometry_new::GeometryRouter;
use ifc_lite_model::{EntityId, IfcModel};
use ifc_lite_parser::{EntityScanner, ParsedModel};
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Plugin for file loading functionality
pub struct LoaderPlugin;

impl Plugin for LoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LoadIfcFileEvent>()
            .add_message::<LoadIfcContentEvent>()
            .add_message::<IfcFileLoadedEvent>()
            .add_message::<OpenFileDialogRequest>()
            .init_resource::<FileDialogState>()
            .add_systems(
                Update,
                (
                    handle_open_dialog_request,
                    poll_file_dialog,
                    poll_wasm_file_input,
                    handle_load_file_event,
                    handle_load_content_event,
                    handle_file_drop,
                ),
            );

        // On WASM, setup file input handling
        #[cfg(target_arch = "wasm32")]
        {
            setup_wasm_file_input();
        }
    }
}

/// System to poll WASM file input for pending files
fn poll_wasm_file_input(mut content_events: MessageWriter<LoadIfcContentEvent>) {
    if let Some((file_name, content)) = poll_pending_file() {
        content_events.write(LoadIfcContentEvent { file_name, content });
    }
}

/// Message to request opening a file dialog
#[derive(Message)]
pub struct OpenFileDialogRequest;

/// State for tracking async file dialog
#[derive(Resource, Default)]
pub struct FileDialogState {
    task: Option<Task<Option<PathBuf>>>,
}

/// Message to trigger file loading from path (native)
#[derive(Message)]
pub struct LoadIfcFileEvent {
    pub path: std::path::PathBuf,
}

/// Message to trigger file loading from content (WASM)
#[derive(Message)]
pub struct LoadIfcContentEvent {
    pub file_name: String,
    pub content: String,
}

/// Message emitted when file loading completes
#[derive(Message)]
pub struct IfcFileLoadedEvent {
    pub path: PathBuf,
    pub entity_count: usize,
    pub mesh_count: usize,
}

/// System to handle request to open file dialog (spawns async task)
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "ios"),
    not(target_os = "macos")
))]
fn handle_open_dialog_request(
    mut requests: MessageReader<OpenFileDialogRequest>,
    mut state: ResMut<FileDialogState>,
) {
    for _ in requests.read() {
        // Don't spawn another dialog if one is already pending
        if state.task.is_some() {
            crate::log("[Loader] File dialog already open");
            continue;
        }

        crate::log_info("[Loader] Opening file dialog...");

        let task_pool = IoTaskPool::get();
        let task = task_pool.spawn(async {
            use rfd::AsyncFileDialog;

            let file = AsyncFileDialog::new()
                .add_filter("IFC Files", &["ifc", "IFC"])
                .set_title("Open IFC File")
                .pick_file()
                .await;

            file.map(|f| f.path().to_path_buf())
        });

        state.task = Some(task);
    }
}

/// Handle file dialog request on WASM - trigger file input
#[cfg(target_arch = "wasm32")]
fn handle_open_dialog_request(
    mut requests: MessageReader<OpenFileDialogRequest>,
    _state: ResMut<FileDialogState>,
) {
    for _ in requests.read() {
        crate::log_info("[Loader] Opening file dialog (WASM)...");
        trigger_file_dialog();
    }
}

/// Stub for iOS/macOS - file dialog handled by native UI
#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "ios", target_os = "macos")
))]
fn handle_open_dialog_request(
    mut _requests: MessageReader<OpenFileDialogRequest>,
    mut _state: ResMut<FileDialogState>,
) {
    // File dialog handled by native UI on these platforms
}

/// System to poll async file dialog result
fn poll_file_dialog(
    mut state: ResMut<FileDialogState>,
    mut load_events: MessageWriter<LoadIfcFileEvent>,
) {
    if let Some(ref mut task) = state.task {
        if let Some(result) = bevy::tasks::block_on(bevy::tasks::poll_once(task)) {
            if let Some(path) = result {
                crate::log_info(&format!("[Loader] File selected: {:?}", path));
                load_events.write(LoadIfcFileEvent { path });
            } else {
                crate::log("[Loader] File dialog cancelled");
            }
            state.task = None;
        }
    }
}

/// System to handle file load events
fn handle_load_file_event(
    mut events: MessageReader<LoadIfcFileEvent>,
    mut scene_data: ResMut<IfcSceneData>,
    mut auto_fit: ResMut<crate::mesh::AutoFitState>,
    mut loaded_events: MessageWriter<IfcFileLoadedEvent>,
) {
    for event in events.read() {
        crate::log_info(&format!("[Loader] Loading file: {:?}", event.path));

        match load_ifc_file(&event.path) {
            Ok((meshes, entities)) => {
                let mesh_count = meshes.len();
                let entity_count = entities.len();

                crate::log_info(&format!(
                    "[Loader] Loaded {} meshes, {} entities",
                    mesh_count, entity_count
                ));

                // Update scene data
                scene_data.meshes = meshes;
                scene_data.entities = entities;
                scene_data.dirty = true;
                scene_data.bounds = None;

                // Reset auto-fit to trigger camera adjustment
                auto_fit.has_fit = false;

                loaded_events.write(IfcFileLoadedEvent {
                    path: event.path.clone(),
                    entity_count,
                    mesh_count,
                });
            }
            Err(e) => {
                crate::log_info(&format!("[Loader] Error loading file: {}", e));
            }
        }
    }
}

/// System to handle drag-and-drop files
fn handle_file_drop(
    mut file_drag_drop_events: MessageReader<bevy::window::FileDragAndDrop>,
    mut load_events: MessageWriter<LoadIfcFileEvent>,
) {
    for event in file_drag_drop_events.read() {
        if let bevy::window::FileDragAndDrop::DroppedFile { path_buf, .. } = event {
            // Check if it's an IFC file
            if let Some(ext) = path_buf.extension() {
                if ext.eq_ignore_ascii_case("ifc") {
                    crate::log_info(&format!("[Loader] File dropped: {:?}", path_buf));
                    load_events.write(LoadIfcFileEvent {
                        path: path_buf.clone(),
                    });
                }
            }
        }
    }
}

/// System to handle content load events (WASM - content comes from JS file input)
fn handle_load_content_event(
    mut events: MessageReader<LoadIfcContentEvent>,
    mut scene_data: ResMut<IfcSceneData>,
    mut auto_fit: ResMut<crate::mesh::AutoFitState>,
    mut loaded_events: MessageWriter<IfcFileLoadedEvent>,
) {
    for event in events.read() {
        crate::log_info(&format!(
            "[Loader] Loading content: {} ({:.2} MB)",
            event.file_name,
            event.content.len() as f64 / (1024.0 * 1024.0)
        ));

        match load_ifc_content(&event.content) {
            Ok((meshes, entities)) => {
                let mesh_count = meshes.len();
                let entity_count = entities.len();

                crate::log_info(&format!(
                    "[Loader] Loaded {} meshes, {} entities",
                    mesh_count, entity_count
                ));

                // Update scene data
                scene_data.meshes = meshes;
                scene_data.entities = entities;
                scene_data.dirty = true;
                scene_data.bounds = None;

                // Reset auto-fit to trigger camera adjustment
                auto_fit.has_fit = false;

                loaded_events.write(IfcFileLoadedEvent {
                    path: PathBuf::from(&event.file_name),
                    entity_count,
                    mesh_count,
                });
            }
            Err(e) => {
                crate::log_info(&format!("[Loader] Error loading content: {}", e));
            }
        }
    }
}

// ============================================================================
// WASM File Input Support
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod wasm_file_input {
    use super::*;
    use std::sync::Mutex;
    use wasm_bindgen::closure::Closure;

    // Global storage for pending file content (set by JS callback, read by Bevy system)
    static PENDING_FILE: Mutex<Option<(String, String)>> = Mutex::new(None);

    /// Setup WASM file input - creates hidden input element and exposes JS API
    pub fn setup_wasm_file_input() {
        // Create file input element
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let document = match window.document() {
            Some(d) => d,
            None => return,
        };

        // Create hidden file input
        let input: web_sys::HtmlInputElement = match document.create_element("input") {
            Ok(el) => match el.dyn_into() {
                Ok(i) => i,
                Err(_) => return,
            },
            Err(_) => return,
        };

        input.set_type("file");
        input.set_accept(".ifc,.IFC");
        input.set_id("bevy-file-input");
        input.style().set_property("display", "none").ok();

        // Add to document
        if let Some(body) = document.body() {
            let _ = body.append_child(&input);
        }

        // Set up change handler
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let input: web_sys::HtmlInputElement = match event.target() {
                Some(t) => match t.dyn_into() {
                    Ok(i) => i,
                    Err(_) => return,
                },
                None => return,
            };

            let files = match input.files() {
                Some(f) => f,
                None => return,
            };

            let file = match files.get(0) {
                Some(f) => f,
                None => return,
            };

            let file_name = file.name();
            crate::log_info(&format!("[WASM] File selected: {}", file_name));

            // Read file using FileReader
            let reader = match web_sys::FileReader::new() {
                Ok(r) => r,
                Err(_) => return,
            };

            let reader_clone = reader.clone();
            let file_name_clone = file_name.clone();

            let onload = Closure::wrap(Box::new(move |_: web_sys::Event| {
                let result = match reader_clone.result() {
                    Ok(r) => r,
                    Err(_) => return,
                };

                let content = match result.as_string() {
                    Some(s) => s,
                    None => return,
                };

                crate::log_info(&format!(
                    "[WASM] File read: {} bytes",
                    content.len()
                ));

                // Store in global for Bevy to pick up
                if let Ok(mut pending) = PENDING_FILE.lock() {
                    *pending = Some((file_name_clone.clone(), content));
                }
            }) as Box<dyn FnMut(_)>);

            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            onload.forget(); // Leak closure to keep it alive

            let _ = reader.read_as_text(&file);

            // Clear input so same file can be selected again
            input.set_value("");
        }) as Box<dyn FnMut(_)>);

        input.set_onchange(Some(closure.as_ref().unchecked_ref()));
        closure.forget(); // Leak closure to keep it alive

        crate::log("[WASM] File input element created");
    }

    /// Check for pending file content and emit load event
    pub fn poll_pending_file() -> Option<(String, String)> {
        if let Ok(mut pending) = PENDING_FILE.lock() {
            pending.take()
        } else {
            None
        }
    }

    /// Trigger file input dialog from JS
    pub fn trigger_file_dialog() {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let document = match window.document() {
            Some(d) => d,
            None => return,
        };

        if let Some(input) = document.get_element_by_id("bevy-file-input") {
            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                input.click();
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_file_input::*;

#[cfg(not(target_arch = "wasm32"))]
fn setup_wasm_file_input() {
    // No-op on native
}

#[cfg(not(target_arch = "wasm32"))]
pub fn poll_pending_file() -> Option<(String, String)> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn trigger_file_dialog() {
    // No-op on native - use rfd instead
}

/// Load an IFC file and convert to viewer format
fn load_ifc_file(
    path: &std::path::Path,
) -> Result<(Vec<IfcMesh>, Vec<EntityInfo>), Box<dyn std::error::Error>> {
    // Read file content
    let content = std::fs::read_to_string(path)?;

    // Parse using trait-based parser
    // Arguments: content, build_spatial (false for now), extract_properties (false)
    let model = Arc::new(ParsedModel::parse(&content, false, false)?);

    // Create geometry router with default processors and unit scale from model
    let router = GeometryRouter::with_default_processors_and_unit_scale(model.unit_scale());

    // Get resolver for entity lookups
    let resolver = model.resolver();

    // Collect building elements and their info
    let mut meshes = Vec::new();
    let mut entities = Vec::new();

    // PERFORMANCE: Use scanner for fast initial pass to find building elements
    // This avoids decoding all entities just to check their type
    let mut scanner = EntityScanner::new(&content);
    let mut element_ids: Vec<(u32, String)> = Vec::new();

    while let Some((id, type_name, _, _)) = scanner.next_entity() {
        // Fast check using type name string (no entity decoding needed)
        if has_geometry_type_name(type_name) {
            element_ids.push((id, type_name.to_string()));
        }
    }

    crate::log_info(&format!(
        "[Loader] Found {} building elements",
        element_ids.len()
    ));

    // Process each element - only NOW do we decode entities
    for (id, type_name) in element_ids {
        // Get the decoded entity (lazy decode)
        let entity = match resolver.get(EntityId(id)) {
            Some(e) => e,
            None => continue,
        };

        // Get entity name (attribute 2 for most building elements)
        let name: Option<String> = entity.get_string(2).map(|s: &str| s.to_string());

        // Process geometry
        let mesh = match router.process_element(&entity, resolver) {
            Ok(m) => m,
            Err(e) => {
                crate::log(&format!(
                    "[Loader] Failed to process #{} ({}): {}",
                    id, type_name, e
                ));
                continue;
            }
        };

        if mesh.is_empty() {
            continue;
        }

        // Convert to IfcMesh format - takes ownership of mesh, no cloning!
        let color = crate::mesh::get_default_color(&type_name);
        let ifc_mesh = IfcMesh::from_geometry_mesh(
            id as u64,
            mesh, // Move, not clone
            color,
            type_name.clone(),
            name.clone(),
        );
        meshes.push(ifc_mesh);

        // Add entity info
        entities.push(EntityInfo {
            id: id as u64,
            entity_type: type_name,
            name,
            storey: None, // TODO: extract from spatial structure
            storey_elevation: None,
        });
    }

    Ok((meshes, entities))
}

/// Load IFC from content string (for WASM where we get content from JS)
fn load_ifc_content(
    content: &str,
) -> Result<(Vec<IfcMesh>, Vec<EntityInfo>), Box<dyn std::error::Error>> {
    // Parse using trait-based parser
    // Arguments: content, build_spatial (false for now), extract_properties (false)
    let model = Arc::new(ParsedModel::parse(content, false, false)?);

    // Create geometry router with default processors and unit scale from model
    let router = GeometryRouter::with_default_processors_and_unit_scale(model.unit_scale());

    // Get resolver for entity lookups
    let resolver = model.resolver();

    // Collect building elements and their info
    let mut meshes = Vec::new();
    let mut entities = Vec::new();

    // PERFORMANCE: Use scanner for fast initial pass to find building elements
    // This avoids decoding all entities just to check their type
    let mut scanner = EntityScanner::new(content);
    let mut element_ids: Vec<(u32, String)> = Vec::new();

    while let Some((id, type_name, _, _)) = scanner.next_entity() {
        // Fast check using type name string (no entity decoding needed)
        if has_geometry_type_name(type_name) {
            element_ids.push((id, type_name.to_string()));
        }
    }

    crate::log_info(&format!(
        "[Loader] Found {} building elements",
        element_ids.len()
    ));

    // Process each element - only NOW do we decode entities
    for (id, type_name) in element_ids {
        // Get the decoded entity (lazy decode)
        let entity = match resolver.get(EntityId(id)) {
            Some(e) => e,
            None => continue,
        };

        // Get entity name (attribute 2 for most building elements)
        let name: Option<String> = entity.get_string(2).map(|s: &str| s.to_string());

        // Process geometry
        let mesh = match router.process_element(&entity, resolver) {
            Ok(m) => m,
            Err(e) => {
                crate::log(&format!(
                    "[Loader] Failed to process #{} ({}): {}",
                    id, type_name, e
                ));
                continue;
            }
        };

        if mesh.is_empty() {
            continue;
        }

        // Convert to IfcMesh format - takes ownership of mesh, no cloning!
        let color = crate::mesh::get_default_color(&type_name);
        let ifc_mesh = IfcMesh::from_geometry_mesh(
            id as u64,
            mesh, // Move, not clone
            color,
            type_name.clone(),
            name.clone(),
        );
        meshes.push(ifc_mesh);

        // Add entity info
        entities.push(EntityInfo {
            id: id as u64,
            entity_type: type_name,
            name,
            storey: None, // TODO: extract from spatial structure
            storey_elevation: None,
        });
    }

    Ok((meshes, entities))
}

/// Check if an IFC type name (string) can have geometry representation
/// This is used for fast scanning without decoding entities
fn has_geometry_type_name(type_name: &str) -> bool {
    matches!(
        type_name.to_uppercase().as_str(),
        // Walls
        "IFCWALL"
            | "IFCWALLSTANDARDCASE"
            | "IFCCURTAINWALL"
            // Slabs and floors
            | "IFCSLAB"
            // Roofs
            | "IFCROOF"
            // Structural elements
            | "IFCBEAM"
            | "IFCCOLUMN"
            | "IFCMEMBER"
            | "IFCPLATE"
            // Openings
            | "IFCDOOR"
            | "IFCWINDOW"
            // Circulation
            | "IFCSTAIR"
            | "IFCSTAIRFLIGHT"
            | "IFCRAMP"
            | "IFCRAMPFLIGHT"
            | "IFCRAILING"
            // Coverings
            | "IFCCOVERING"
            // Furniture
            | "IFCFURNISHINGELEMENT"
            // Foundations
            | "IFCFOOTING"
            | "IFCPILE"
            // Generic building elements
            | "IFCBUILDINGELEMENTPROXY"
            // MEP
            | "IFCFLOWTERMINAL"
            | "IFCFLOWSEGMENT"
            | "IFCFLOWFITTING"
            | "IFCFLOWCONTROLLER"
            // Spaces (optional, often transparent)
            | "IFCSPACE"
    )
}
