//! IFC file loading - handles file dialog and drag-and-drop

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
use ifc_lite_core::{EntityDecoder, EntityScanner};
use ifc_lite_geometry::GeometryRouter;
use std::path::PathBuf;

/// Plugin for file loading functionality
pub struct LoaderPlugin;

impl Plugin for LoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LoadIfcFileEvent>()
            .add_message::<IfcFileLoadedEvent>()
            .add_message::<OpenFileDialogRequest>()
            .init_resource::<FileDialogState>()
            .add_systems(
                Update,
                (
                    handle_open_dialog_request,
                    poll_file_dialog,
                    handle_load_file_event,
                    handle_file_drop,
                ),
            );
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

/// Message to trigger file loading (from button or other sources)
#[derive(Message)]
pub struct LoadIfcFileEvent {
    pub path: std::path::PathBuf,
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

/// Stub for platforms that don't support rfd (WASM, iOS, macOS)
#[cfg(any(target_arch = "wasm32", target_os = "ios", target_os = "macos"))]
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

/// Load an IFC file and convert to viewer format
fn load_ifc_file(
    path: &std::path::Path,
) -> Result<(Vec<IfcMesh>, Vec<EntityInfo>), Box<dyn std::error::Error>> {
    // Read file content
    let content = std::fs::read_to_string(path)?;

    // Create decoder and router
    let mut decoder = EntityDecoder::new(&content);
    let router = GeometryRouter::with_units(&content, &mut decoder);

    // Collect building elements and their info
    let mut meshes = Vec::new();
    let mut entities = Vec::new();
    let mut scanner = EntityScanner::new(&content);

    // First pass: collect all elements with potential geometry
    // Use the same comprehensive check as the Yew viewer
    let mut element_ids: Vec<(u32, String)> = Vec::new();

    while let Some((id, type_name, _, _)) = scanner.next_entity() {
        // Use ifc_lite_core's comprehensive geometry check
        if ifc_lite_core::has_geometry_by_name(type_name) {
            // Skip Unknown types - we can't properly process them
            let ifc_type = ifc_lite_core::IfcType::from_str(type_name);
            if !matches!(ifc_type, ifc_lite_core::IfcType::Unknown(_)) {
                element_ids.push((id, type_name.to_string()));
            }
        }
    }

    crate::log_info(&format!(
        "[Loader] Found {} building elements",
        element_ids.len()
    ));

    // Process each element
    for (id, type_name) in element_ids {
        let entity = match decoder.decode_by_id(id) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Get entity name (attribute 2 for most building elements)
        let name = entity.get_string(2).map(|s| s.to_string());

        // Process geometry
        let mesh = match router.process_element(&entity, &mut decoder) {
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
