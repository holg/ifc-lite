//! Toolbar component with tool buttons and file operations

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use gloo_file::callbacks::FileReader;
use crate::state::{ViewerAction, ViewerStateContext, Tool, Progress};
use crate::bridge::{self, GeometryData, EntityData};

/// Toolbar component
#[function_component]
pub fn Toolbar() -> Html {
    let state = use_context::<ViewerStateContext>().expect("ViewerStateContext not found");

    // File input ref
    let file_input_ref = use_node_ref();

    // File reader state for async file loading
    let file_reader = use_state(|| None::<FileReader>);

    // Handle file selection
    let on_file_change = {
        let state = state.clone();
        let file_reader = file_reader.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    let file_name = file.name();
                    state.dispatch(ViewerAction::SetFileName(file_name.clone()));
                    state.dispatch(ViewerAction::SetLoading(true));
                    state.dispatch(ViewerAction::SetProgress(Progress {
                        phase: "Reading file".to_string(),
                        percent: 0.0,
                    }));

                    bridge::log(&format!("Loading file: {}", file_name));

                    // Read file contents
                    let gloo_file = gloo_file::File::from(file);
                    let state_clone = state.clone();

                    let reader = gloo_file::callbacks::read_as_bytes(&gloo_file, move |result| {
                        match result {
                            Ok(bytes) => {
                                bridge::log(&format!("File read: {} bytes", bytes.len()));
                                state_clone.dispatch(ViewerAction::SetProgress(Progress {
                                    phase: "Parsing IFC".to_string(),
                                    percent: 10.0,
                                }));

                                // Parse the IFC file
                                let content = String::from_utf8_lossy(&bytes).to_string();

                                // Use spawn_local for the async parsing work
                                let state_inner = state_clone.clone();
                                spawn_local(async move {
                                    match parse_and_process_ifc(&content, &state_inner) {
                                        Ok(_) => {
                                            bridge::log("IFC file processed successfully");
                                            state_inner.dispatch(ViewerAction::SetLoading(false));
                                            state_inner.dispatch(ViewerAction::ClearProgress);
                                        }
                                        Err(e) => {
                                            bridge::log_error(&format!("Failed to process IFC: {}", e));
                                            state_inner.dispatch(ViewerAction::SetLoading(false));
                                            state_inner.dispatch(ViewerAction::ClearProgress);
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                bridge::log_error(&format!("Failed to read file: {:?}", e));
                                state_clone.dispatch(ViewerAction::SetLoading(false));
                            }
                        }
                    });

                    file_reader.set(Some(reader));
                }
            }
        })
    };

    // Tool button helper
    let tool_button = |tool: Tool, state: &ViewerStateContext| {
        let is_active = state.active_tool == tool;
        let state = state.clone();
        html! {
            <button
                class={classes!("tool-btn", is_active.then_some("active"))}
                onclick={Callback::from(move |_| {
                    state.dispatch(ViewerAction::SetActiveTool(tool));
                    // Send camera mode to Bevy for Pan/Orbit/Walk
                    let mode = match tool {
                        Tool::Pan => Some("pan"),
                        Tool::Orbit => Some("orbit"),
                        Tool::Walk => Some("walk"),
                        _ => Some("orbit"), // Default to orbit for other tools
                    };
                    if let Some(m) = mode {
                        crate::bridge::save_camera_cmd(&crate::bridge::CameraCommand {
                            cmd: "set_mode".to_string(),
                            mode: Some(m.to_string()),
                        });
                    }
                })}
                title={tool.label()}
            >
                {tool.icon()}
            </button>
        }
    };

    html! {
        <div class="toolbar">
            // File operations
            <div class="toolbar-group">
                <input
                    ref={file_input_ref.clone()}
                    type="file"
                    accept=".ifc"
                    style="display: none"
                    onchange={on_file_change}
                />
                <button
                    class="tool-btn"
                    onclick={
                        let file_input_ref = file_input_ref.clone();
                        Callback::from(move |_| {
                            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                                input.click();
                            }
                        })
                    }
                    title="Open IFC file"
                >
                    {"📁"}
                </button>
            </div>

            <div class="toolbar-separator" />

            // Tool buttons
            <div class="toolbar-group">
                {tool_button(Tool::Select, &state)}
                {tool_button(Tool::Pan, &state)}
                {tool_button(Tool::Orbit, &state)}
                {tool_button(Tool::Walk, &state)}
            </div>

            <div class="toolbar-separator" />

            <div class="toolbar-group">
                {tool_button(Tool::Measure, &state)}
                {tool_button(Tool::Section, &state)}
                {tool_button(Tool::BoxSelect, &state)}
            </div>

            <div class="toolbar-separator" />

            // Visibility controls
            <div class="toolbar-group">
                <button
                    class="tool-btn"
                    onclick={
                        let state = state.clone();
                        Callback::from(move |_| {
                            state.dispatch(ViewerAction::ShowAll);
                        })
                    }
                    title="Show All (A)"
                >
                    {"👁"}
                </button>
                <button
                    class="tool-btn"
                    onclick={
                        let state = state.clone();
                        Callback::from(move |_| {
                            if !state.selected_ids.is_empty() {
                                let ids = state.selected_ids.clone();
                                state.dispatch(ViewerAction::IsolateEntities(ids));
                            }
                        })
                    }
                    title="Isolate Selection (I)"
                >
                    {"🎯"}
                </button>
                <button
                    class="tool-btn"
                    onclick={
                        let state = state.clone();
                        Callback::from(move |_| {
                            for id in state.selected_ids.iter() {
                                state.dispatch(ViewerAction::HideEntity(*id));
                            }
                        })
                    }
                    title="Hide Selection (Del)"
                >
                    {"🚫"}
                </button>
            </div>

            <div class="toolbar-separator" />

            // View controls
            <div class="toolbar-group">
                <button
                    class="tool-btn"
                    onclick={Callback::from(|_| {
                        crate::bridge::save_camera_cmd(&crate::bridge::CameraCommand {
                            cmd: "home".to_string(),
                            mode: None,
                        });
                    })}
                    title="Home View (H)"
                >
                    {"🏠"}
                </button>
                <button
                    class="tool-btn"
                    onclick={Callback::from(|_| {
                        crate::bridge::save_camera_cmd(&crate::bridge::CameraCommand {
                            cmd: "fit_all".to_string(),
                            mode: None,
                        });
                    })}
                    title="Fit All (F)"
                >
                    {"⬚"}
                </button>
            </div>

            // Spacer
            <div class="toolbar-spacer" />

            // Right side controls
            <div class="toolbar-group">
                <button
                    class="tool-btn"
                    onclick={
                        let state = state.clone();
                        Callback::from(move |_| {
                            state.dispatch(ViewerAction::ToggleTheme);
                        })
                    }
                    title="Toggle Theme (T)"
                >
                    {if state.theme == crate::state::Theme::Dark { "🌙" } else { "☀️" }}
                </button>
                <button
                    class="tool-btn"
                    onclick={
                        let state = state.clone();
                        Callback::from(move |_| {
                            state.dispatch(ViewerAction::ToggleShortcutsDialog);
                        })
                    }
                    title="Keyboard Shortcuts (?)"
                >
                    {"⌨"}
                </button>
            </div>

            // Loading indicator
            if state.loading {
                <div class="toolbar-loading">
                    <span class="loading-spinner" />
                    if let Some(ref progress) = state.progress {
                        <span class="loading-text">
                            {format!("{} {}%", progress.phase, progress.percent as i32)}
                        </span>
                    }
                </div>
            }
        </div>
    }
}

/// Spatial structure entity info
struct SpatialInfo {
    id: u32,
    name: String,
    entity_type: String,
    elevation: Option<f32>,
}

/// Parse IFC content and send geometry to Bevy via localStorage
pub fn parse_and_process_ifc(content: &str, state: &ViewerStateContext) -> Result<(), String> {
    use ifc_lite_core::{EntityDecoder, EntityScanner, build_entity_index};
    use ifc_lite_geometry::GeometryRouter;
    use std::collections::HashMap;
    use crate::state::{SpatialNode, SpatialNodeType};

    bridge::log("Starting IFC parsing...");

    // Build entity index for O(1) lookups
    let index = build_entity_index(content);
    let entity_count = index.len();

    bridge::log(&format!("Found {} entities in IFC file", entity_count));

    // Create decoder with pre-built index
    let mut decoder = EntityDecoder::with_index(content, index);

    state.dispatch(ViewerAction::SetProgress(Progress {
        phase: "Building spatial hierarchy".to_string(),
        percent: 10.0,
    }));

    // First pass: collect spatial structure
    // Spatial entities: Project, Site, Building, Storey, Space
    let mut spatial_entities: HashMap<u32, SpatialInfo> = HashMap::new();
    // IfcRelAggregates: parent -> children (for Project->Site->Building->Storey)
    let mut aggregates: HashMap<u32, Vec<u32>> = HashMap::new();
    // IfcRelContainedInSpatialStructure: spatial_element -> contained elements
    let mut contained_in: HashMap<u32, Vec<u32>> = HashMap::new();
    // Element to storey mapping for flat view
    let mut element_to_storey: HashMap<u32, u32> = HashMap::new();

    // Use simple line-by-line parsing for reliability (scanner has issues with large files)
    // Scan for spatial structure entities and relationships
    let mut scan_count = 0;
    for line in content.lines() {
        let line = line.trim();
        if !line.starts_with('#') {
            continue;
        }

        // Parse: #ID=TYPE(...)
        let eq_pos = match line.find('=') {
            Some(p) => p,
            None => continue,
        };

        let id_str = &line[1..eq_pos];
        let id: u32 = match id_str.parse() {
            Ok(i) => i,
            Err(_) => continue,
        };

        let rest = &line[eq_pos + 1..];
        let paren_pos = rest.find('(').unwrap_or(rest.len());
        let type_name = rest[..paren_pos].trim();
        let type_upper = type_name.to_uppercase();

        scan_count += 1;

        // Parse spatial structure entities
        match type_upper.as_str() {
            "IFCPROJECT" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity.get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Project".to_string());
                    spatial_entities.insert(id, SpatialInfo {
                        id, name, entity_type: type_name.to_string(), elevation: None
                    });
                }
            }
            "IFCSITE" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity.get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Site".to_string());
                    spatial_entities.insert(id, SpatialInfo {
                        id, name, entity_type: type_name.to_string(), elevation: None
                    });
                }
            }
            "IFCBUILDING" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity.get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Building".to_string());
                    spatial_entities.insert(id, SpatialInfo {
                        id, name, entity_type: type_name.to_string(), elevation: None
                    });
                }
            }
            "IFCBUILDINGSTOREY" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity.get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Storey #{}", id));
                    let elevation = entity.get_float(9).map(|e| e as f32);
                    spatial_entities.insert(id, SpatialInfo {
                        id, name, entity_type: type_name.to_string(), elevation
                    });
                }
            }
            "IFCSPACE" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity.get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Space #{}", id));
                    spatial_entities.insert(id, SpatialInfo {
                        id, name, entity_type: type_name.to_string(), elevation: None
                    });
                }
            }
            // Parse IfcRelAggregates for parent-child relationships
            // Structure: (GlobalId, OwnerHistory, Name, Description, RelatingObject, RelatedObjects)
            "IFCRELAGGREGATES" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    bridge::log(&format!("IfcRelAggregates #{}: {} attributes", id, entity.attributes.len()));
                    let parent_id = entity.get_ref(4);
                    let children = entity.get_ref_list(5);
                    bridge::log(&format!("  parent: {:?}, children: {:?}", parent_id, children));
                    if let (Some(parent_id), Some(children)) = (parent_id, children) {
                        aggregates.entry(parent_id).or_default().extend(children);
                    }
                }
            }
            // Parse IfcRelContainedInSpatialStructure
            // Structure: (GlobalId, OwnerHistory, Name, Description, RelatedElements, RelatingStructure)
            "IFCRELCONTAINEDINSPATIALSTRUCTURE" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    if let Some(structure_id) = entity.get_ref(5) {
                        if let Some(elements) = entity.get_ref_list(4) {
                            contained_in.entry(structure_id).or_default().extend(elements.clone());
                            // Also track element -> storey for flat view
                            for elem_id in elements {
                                element_to_storey.insert(elem_id, structure_id);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    bridge::log(&format!("Scanned {} entities total", scan_count));
    bridge::log(&format!("Found {} spatial entities, {} aggregate relationships, {} containment relationships",
        spatial_entities.len(), aggregates.len(), contained_in.len()));

    // Debug: log spatial entities
    for (id, info) in &spatial_entities {
        bridge::log(&format!("Spatial entity #{}: {} ({})", id, info.name, info.entity_type));
    }

    // Debug: log aggregates
    for (parent, children) in &aggregates {
        bridge::log(&format!("Aggregate: #{} -> {:?}", parent, children));
    }

    // Create geometry router
    let router = GeometryRouter::new();

    state.dispatch(ViewerAction::SetProgress(Progress {
        phase: "Processing geometry".to_string(),
        percent: 30.0,
    }));

    // Second pass: process geometry
    let mut scanner = EntityScanner::new(content);
    let mut geometry_data: Vec<GeometryData> = Vec::new();
    let mut entity_data: Vec<EntityData> = Vec::new();
    let mut processed = 0;
    let mut errors = 0;

    while let Some((id, type_name, _start, _end)) = scanner.next_entity() {
        // Check if this is an element with potential geometry (using comprehensive check)
        if ifc_lite_core::has_geometry_by_name(type_name) {
            if let Some(ifc_type) = ifc_lite_core::IfcType::from_str(type_name) {
                // Skip Unknown types - we can't properly process them
                if matches!(ifc_type, ifc_lite_core::IfcType::Unknown(_)) {
                    bridge::log(&format!("Skipping #{} ({}): Unknown IFC type", id, type_name));
                    continue;
                }

                // Decode the entity
                match decoder.decode_by_id(id) {
                    Ok(entity) => {
                        // Get entity name (attribute 2 for most building elements)
                        let name = entity.get_string(2).map(|s| s.to_string());

                        // Look up storey information from spatial_entities
                        let (storey_name, storey_elevation) = if let Some(&storey_id) = element_to_storey.get(&id) {
                            if let Some(storey) = spatial_entities.get(&storey_id) {
                                (Some(storey.name.clone()), storey.elevation)
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        };

                        // Always add to entity_data for hierarchy panel (even if geometry fails)
                        // Use original type_name to preserve the actual IFC type
                        entity_data.push(EntityData {
                            id: id as u64,
                            entity_type: type_name.to_string(),
                            name: name.clone(),
                            storey: storey_name,
                            storey_elevation,
                        });

                        // Process geometry
                        match router.process_element(&entity, &mut decoder) {
                            Ok(mesh) => {
                                if !mesh.is_empty() {
                                    // Convert mesh to bridge format
                                    // Mesh has positions/normals as flat f32 arrays, indices as u32
                                    // Sanitize values: replace NaN/Infinity with 0.0
                                    let sanitize = |arr: &[f32]| -> Vec<f32> {
                                        arr.iter()
                                            .map(|v| if v.is_finite() { *v } else { 0.0 })
                                            .collect()
                                    };

                                    let positions = sanitize(&mesh.positions);
                                    let normals = sanitize(&mesh.normals);
                                    let indices = mesh.indices.clone();

                                    // Skip if all positions are zero (degenerate mesh)
                                    if positions.iter().all(|v| *v == 0.0) {
                                        bridge::log(&format!("Skipping #{} ({}): degenerate geometry", id, type_name));
                                        errors += 1;
                                        continue;
                                    }

                                    // Default color based on element type
                                    let color = get_element_color(&ifc_type);

                                    // Identity transform (placement already applied by router)
                                    let transform = [
                                        1.0, 0.0, 0.0, 0.0,
                                        0.0, 1.0, 0.0, 0.0,
                                        0.0, 0.0, 1.0, 0.0,
                                        0.0, 0.0, 0.0, 1.0,
                                    ];

                                    geometry_data.push(GeometryData {
                                        entity_id: id as u64,
                                        positions,
                                        normals,
                                        indices,
                                        color,
                                        transform,
                                        entity_type: type_name.to_string(),
                                        name: name.clone(),
                                    });

                                    processed += 1;
                                }
                            }
                            Err(e) => {
                                // Log but don't fail - some entities may not have geometry
                                bridge::log(&format!("Skipping #{} ({}): {}", id, type_name, e));
                                errors += 1;
                            }
                        }
                    }
                    Err(e) => {
                        bridge::log_error(&format!("Failed to decode #{}: {:?}", id, e));
                        errors += 1;
                    }
                }

                // Update progress periodically
                if processed % 50 == 0 {
                    let percent = 30.0 + (processed as f32 / entity_count as f32) * 50.0;
                    state.dispatch(ViewerAction::SetProgress(Progress {
                        phase: format!("Processing geometry ({}/{})", processed, entity_count),
                        percent,
                    }));
                }
            }
        }
    }

    bridge::log(&format!("Processed {} meshes ({} errors)", processed, errors));

    state.dispatch(ViewerAction::SetProgress(Progress {
        phase: "Sending to viewer".to_string(),
        percent: 90.0,
    }));

    // Save to localStorage for Bevy
    bridge::save_geometry(&geometry_data);
    bridge::save_entities(&entity_data);

    // Build storey info for UI (from spatial_entities that are storeys)
    let mut storey_infos: Vec<crate::state::StoreyInfo> = spatial_entities.values()
        .filter(|s| s.entity_type.to_uppercase() == "IFCBUILDINGSTOREY")
        .map(|s| {
            let entity_count = entity_data.iter()
                .filter(|e| e.storey.as_ref() == Some(&s.name))
                .count();
            crate::state::StoreyInfo {
                name: s.name.clone(),
                elevation: s.elevation.unwrap_or(0.0),
                entity_count,
            }
        })
        .collect();
    // Sort by elevation (descending - top floors first)
    storey_infos.sort_by(|a, b| b.elevation.partial_cmp(&a.elevation).unwrap_or(std::cmp::Ordering::Equal));

    // Build entity_infos for flat view
    let entity_infos: Vec<crate::state::EntityInfo> = entity_data
        .iter()
        .map(|e| crate::state::EntityInfo {
            id: e.id,
            entity_type: e.entity_type.clone(),
            name: e.name.clone(),
            global_id: None,
            storey: e.storey.clone(),
            storey_elevation: e.storey_elevation,
        })
        .collect();

    // Track which entities have geometry
    let entities_with_geometry: std::collections::HashSet<u64> = geometry_data.iter()
        .map(|g| g.entity_id)
        .collect();

    // Build spatial tree
    // Helper to get node type from entity type
    let get_node_type = |entity_type: &str| -> SpatialNodeType {
        match entity_type.to_uppercase().as_str() {
            "IFCPROJECT" => SpatialNodeType::Project,
            "IFCSITE" => SpatialNodeType::Site,
            "IFCBUILDING" => SpatialNodeType::Building,
            "IFCBUILDINGSTOREY" => SpatialNodeType::Storey,
            "IFCSPACE" => SpatialNodeType::Space,
            _ => SpatialNodeType::Element,
        }
    };

    // Recursive function to build tree
    fn build_node(
        id: u32,
        spatial_entities: &HashMap<u32, SpatialInfo>,
        aggregates: &HashMap<u32, Vec<u32>>,
        contained_in: &HashMap<u32, Vec<u32>>,
        entity_data: &[EntityData],
        entities_with_geometry: &std::collections::HashSet<u64>,
        get_node_type: &dyn Fn(&str) -> SpatialNodeType,
    ) -> Option<SpatialNode> {
        let info = spatial_entities.get(&id)?;
        let node_type = get_node_type(&info.entity_type);

        let mut children: Vec<SpatialNode> = Vec::new();

        // Add aggregated children (Site->Building->Storey hierarchy)
        if let Some(child_ids) = aggregates.get(&id) {
            for &child_id in child_ids {
                if let Some(child_node) = build_node(
                    child_id, spatial_entities, aggregates, contained_in,
                    entity_data, entities_with_geometry, get_node_type
                ) {
                    children.push(child_node);
                }
            }
        }

        // Add contained elements (elements in this storey/space)
        if let Some(element_ids) = contained_in.get(&id) {
            for &elem_id in element_ids {
                // Find the entity data for this element
                if let Some(elem) = entity_data.iter().find(|e| e.id == elem_id as u64) {
                    let has_geometry = entities_with_geometry.contains(&(elem_id as u64));
                    children.push(SpatialNode {
                        id: elem_id as u64,
                        node_type: SpatialNodeType::Element,
                        name: elem.name.clone().unwrap_or_else(|| format!("#{}", elem_id)),
                        entity_type: elem.entity_type.clone(),
                        elevation: None,
                        children: Vec::new(),
                        has_geometry,
                    });
                }
            }
        }

        // Sort children: spatial structures first (by elevation desc), then elements by type/name
        children.sort_by(|a, b| {
            // Spatial structures come first
            let a_is_spatial = !matches!(a.node_type, SpatialNodeType::Element);
            let b_is_spatial = !matches!(b.node_type, SpatialNodeType::Element);
            if a_is_spatial != b_is_spatial {
                return b_is_spatial.cmp(&a_is_spatial);
            }
            // For storeys, sort by elevation (descending)
            if matches!(a.node_type, SpatialNodeType::Storey) && matches!(b.node_type, SpatialNodeType::Storey) {
                return b.elevation.partial_cmp(&a.elevation).unwrap_or(std::cmp::Ordering::Equal);
            }
            // Otherwise sort by type then name
            match a.entity_type.cmp(&b.entity_type) {
                std::cmp::Ordering::Equal => a.name.cmp(&b.name),
                other => other,
            }
        });

        Some(SpatialNode {
            id: id as u64,
            node_type,
            name: info.name.clone(),
            entity_type: info.entity_type.clone(),
            elevation: info.elevation,
            children,
            has_geometry: false, // Spatial structures don't have geometry
        })
    }

    // Find the root (usually IfcProject)
    let root_id = spatial_entities.iter()
        .find(|(_, info)| info.entity_type.to_uppercase() == "IFCPROJECT")
        .map(|(id, _)| *id);

    if let Some(root_id) = root_id {
        if let Some(tree) = build_node(
            root_id, &spatial_entities, &aggregates, &contained_in,
            &entity_data, &entities_with_geometry, &get_node_type
        ) {
            state.dispatch(ViewerAction::SetSpatialTree(tree));
        }
    }

    state.dispatch(ViewerAction::SetEntities(entity_infos));
    state.dispatch(ViewerAction::SetStoreys(storey_infos));

    bridge::log(&format!("Geometry sent to Bevy viewer: {} entities", geometry_data.len()));

    Ok(())
}

/// Get default color for element type (matches TypeScript viewer default-materials.ts)
fn get_element_color(ifc_type: &ifc_lite_core::IfcType) -> [f32; 4] {
    use ifc_lite_core::IfcType;
    match ifc_type {
        // Walls - warm white (matte plaster look)
        IfcType::IfcWall | IfcType::IfcWallStandardCase => [0.95, 0.93, 0.88, 1.0],
        // Slabs - cool gray (concrete)
        IfcType::IfcSlab => [0.75, 0.75, 0.78, 1.0],
        // Beams - steel blue metallic
        IfcType::IfcBeam => [0.55, 0.55, 0.6, 1.0],
        // Columns - steel blue metallic
        IfcType::IfcColumn => [0.55, 0.55, 0.6, 1.0],
        // Doors - warm wood
        IfcType::IfcDoor => [0.6, 0.45, 0.3, 1.0],
        // Windows - sky blue transparent glass
        IfcType::IfcWindow => [0.6, 0.8, 0.95, 0.3],
        // Roof - terracotta tile
        IfcType::IfcRoof => [0.7, 0.45, 0.35, 1.0],
        // Stairs - light warm gray
        IfcType::IfcStair => [0.8, 0.78, 0.75, 1.0],
        // Railings - dark metal
        IfcType::IfcRailing => [0.35, 0.35, 0.4, 1.0],
        // Plates - steel
        IfcType::IfcPlate => [0.6, 0.6, 0.65, 1.0],
        // Members - steel
        IfcType::IfcMember => [0.55, 0.55, 0.6, 1.0],
        // Curtain walls - glass blue transparent
        IfcType::IfcCurtainWall => [0.5, 0.7, 0.85, 0.4],
        // Coverings - light gray
        IfcType::IfcCovering => [0.85, 0.85, 0.85, 1.0],
        // Footings - concrete gray
        IfcType::IfcFooting => [0.65, 0.65, 0.68, 1.0],
        // Piles - concrete
        IfcType::IfcPile => [0.6, 0.6, 0.62, 1.0],
        // Opening elements - invisible/very light
        IfcType::IfcOpeningElement => [0.9, 0.9, 0.9, 0.1],
        // Building element proxy - neutral gray
        IfcType::IfcBuildingElementProxy => [0.7, 0.7, 0.7, 1.0],
        // Reinforcing - dark steel
        IfcType::IfcReinforcingBar | IfcType::IfcReinforcingMesh | IfcType::IfcTendon => [0.4, 0.4, 0.45, 1.0],
        // Default - neutral warm gray
        _ => [0.8, 0.78, 0.75, 1.0],
    }
}
