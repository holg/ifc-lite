//! Toolbar component with tool buttons and file operations

use crate::bridge::{self, CameraCommand, EntityData, GeometryData};
use crate::state::{use_viewer_state, EntityInfo, Progress, SpatialNode, SpatialNodeType, Tool};
use ifc_lite_core::DecodedEntity;
use ifc_lite_geometry::GeometryRouter;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

/// Toolbar component
#[component]
pub fn Toolbar() -> impl IntoView {
    let state = use_viewer_state();
    let file_input_ref = NodeRef::<leptos::html::Input>::new();

    // Handle file selection
    let on_file_change = move |ev: leptos::ev::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.unchecked_into();

        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                load_file(file, state);
            }
        }
    };

    view! {
        <div class="toolbar">
            // File operations
            <div class="toolbar-group">
                <input
                    node_ref=file_input_ref
                    type="file"
                    accept=".ifc"
                    style="display: none"
                    on:change=on_file_change
                />
                <button
                    class="tool-btn"
                    on:click=move |_| {
                        if let Some(input) = file_input_ref.get() {
                            input.click();
                        }
                    }
                    title="Open IFC file"
                >
                    "📁"
                </button>
            </div>

            <div class="toolbar-separator"></div>

            // Tool buttons
            <div class="toolbar-group">
                <ToolButton tool=Tool::Select />
                <ToolButton tool=Tool::Pan />
                <ToolButton tool=Tool::Orbit />
                <ToolButton tool=Tool::Walk />
            </div>

            <div class="toolbar-separator"></div>

            <div class="toolbar-group">
                <ToolButton tool=Tool::Measure />
                <ToolButton tool=Tool::Section />
                <ToolButton tool=Tool::BoxSelect />
            </div>

            <div class="toolbar-separator"></div>

            // Visibility controls
            <div class="toolbar-group">
                <button
                    class="tool-btn"
                    on:click=move |_| state.visibility.show_all()
                    title="Show All (A)"
                >
                    "👁"
                </button>
                <button
                    class="tool-btn"
                    on:click=move |_| {
                        let ids = state.selection.selected_ids.get();
                        if !ids.is_empty() {
                            state.visibility.isolate_many(ids);
                        }
                    }
                    title="Isolate Selection (I)"
                >
                    "🎯"
                </button>
                <button
                    class="tool-btn"
                    on:click=move |_| {
                        for id in state.selection.selected_ids.get().iter() {
                            state.visibility.hide(*id);
                        }
                    }
                    title="Hide Selection (Del)"
                >
                    "🚫"
                </button>
            </div>

            <div class="toolbar-separator"></div>

            // View controls
            <div class="toolbar-group">
                <button
                    class="tool-btn"
                    on:click=move |_| {
                        bridge::save_camera_cmd(&CameraCommand {
                            cmd: "home".to_string(),
                            mode: None,
                        });
                    }
                    title="Home View (H)"
                >
                    "🏠"
                </button>
                <button
                    class="tool-btn"
                    on:click=move |_| {
                        bridge::save_camera_cmd(&CameraCommand {
                            cmd: "fit_all".to_string(),
                            mode: None,
                        });
                    }
                    title="Fit All (F)"
                >
                    "⬚"
                </button>
            </div>

            // Spacer
            <div class="toolbar-spacer"></div>

            // Right side controls
            <div class="toolbar-group">
                <button
                    class="tool-btn"
                    on:click=move |_| state.ui.toggle_theme()
                    title="Toggle Theme (T)"
                >
                    {move || if state.ui.theme.get() == crate::state::Theme::Dark { "🌙" } else { "☀️" }}
                </button>
                <button
                    class="tool-btn"
                    on:click=move |_| state.ui.toggle_shortcuts_dialog()
                    title="Keyboard Shortcuts (?)"
                >
                    "⌨"
                </button>
            </div>

            // Loading indicator
            {move || {
                let loading = state.loading.loading.get();
                let progress = state.loading.progress.get();

                if loading {
                    Some(view! {
                        <div class="toolbar-loading">
                            <span class="loading-spinner"></span>
                            {progress.map(|p| view! {
                                <span class="loading-text">
                                    {format!("{} {}%", p.phase, p.percent as i32)}
                                </span>
                            })}
                        </div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}

/// Tool button component
#[component]
fn ToolButton(tool: Tool) -> impl IntoView {
    let state = use_viewer_state();
    let is_active = Memo::new(move |_| state.ui.active_tool.get() == tool);

    view! {
        <button
            class=move || if is_active.get() { "tool-btn active" } else { "tool-btn" }
            on:click=move |_| {
                state.ui.set_tool(tool);
                // Send camera mode to Bevy
                let mode = match tool {
                    Tool::Pan => Some("pan"),
                    Tool::Orbit => Some("orbit"),
                    Tool::Walk => Some("walk"),
                    _ => Some("orbit"),
                };
                if let Some(m) = mode {
                    bridge::save_camera_cmd(&CameraCommand {
                        cmd: "set_mode".to_string(),
                        mode: Some(m.to_string()),
                    });
                }
            }
            title=tool.label()
        >
            {tool.icon()}
        </button>
    }
}

/// Load a file and parse it
fn load_file(file: web_sys::File, state: crate::state::ViewerState) {
    let file_name = file.name();
    state.scene.set_file_name(file_name.clone());
    state.loading.set_loading(true);
    state.loading.set_progress(Progress {
        phase: "Reading file".to_string(),
        percent: 0.0,
    });

    bridge::log(&format!("Loading file: {}", file_name));

    // Read file contents using gloo
    let gloo_file = gloo_file::File::from(file);

    spawn_local(async move {
        match gloo_file::futures::read_as_bytes(&gloo_file).await {
            Ok(bytes) => {
                bridge::log(&format!("File read: {} bytes", bytes.len()));
                state.loading.set_progress(Progress {
                    phase: "Parsing IFC".to_string(),
                    percent: 10.0,
                });

                let content = String::from_utf8_lossy(&bytes).to_string();

                match parse_and_process_ifc(&content, state) {
                    Ok(_) => {
                        bridge::log_info("IFC file loaded successfully");
                        state.loading.set_loading(false);
                        state.loading.clear_progress();
                        // Trigger "Fit All" to frame the loaded model
                        bridge::save_camera_cmd(&CameraCommand {
                            cmd: "fit_all".to_string(),
                            mode: None,
                        });
                    }
                    Err(e) => {
                        bridge::log_error(&format!("Failed to process IFC: {}", e));
                        state.loading.set_loading(false);
                        state.loading.clear_progress();
                    }
                }
            }
            Err(e) => {
                bridge::log_error(&format!("Failed to read file: {:?}", e));
                state.loading.set_loading(false);
            }
        }
    });
}

// ============================================================================
// IFC Parsing (ported from Yew version)
// ============================================================================

/// Helper to extract entity refs from a list attribute
fn get_ref_list(entity: &DecodedEntity, index: usize) -> Option<Vec<u32>> {
    entity
        .get_list(index)
        .map(|list| list.iter().filter_map(|v| v.as_entity_ref()).collect())
}

/// Spatial structure entity info
#[allow(dead_code)]
struct SpatialInfo {
    id: u32,
    name: String,
    entity_type: String,
    elevation: Option<f32>,
}

/// Get current time in milliseconds
fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// Extract property sets and quantities for an element
fn extract_properties_and_quantities(
    element_id: u32,
    element_properties: &std::collections::HashMap<u32, Vec<u32>>,
    element_to_type: &std::collections::HashMap<u32, u32>,
    decoder: &mut ifc_lite_core::EntityDecoder,
    unit_scale: f64,
) -> (Vec<crate::state::PropertySet>, Vec<crate::state::QuantityValue>) {
    use crate::state::{PropertySet, PropertyValue, QuantityValue};

    let mut property_sets = Vec::new();
    let mut quantities = Vec::new();

    // Collect property definition IDs from both element and its type
    let mut prop_def_ids: Vec<u32> = Vec::new();

    // Get direct properties on this element
    if let Some(ids) = element_properties.get(&element_id) {
        prop_def_ids.extend(ids.iter().cloned());
    }

    // Get properties from element's type (inherited)
    if let Some(&type_id) = element_to_type.get(&element_id) {
        if let Some(ids) = element_properties.get(&type_id) {
            prop_def_ids.extend(ids.iter().cloned());
        }
    }

    if prop_def_ids.is_empty() {
        return (property_sets, quantities);
    }

    for prop_def_id in prop_def_ids {
        // Decode the property definition
        let prop_def = match decoder.decode_by_id(prop_def_id) {
            Ok(e) => e,
            Err(_) => continue,
        };

        match prop_def.ifc_type {
            ifc_lite_core::IfcType::IfcPropertySet => {
                // IfcPropertySet: (GlobalId, OwnerHistory, Name, Description, HasProperties)
                let pset_name = prop_def
                    .get_string(2)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("PropertySet #{}", prop_def_id));

                let mut properties = Vec::new();

                // Get HasProperties list (attribute 4)
                if let Some(prop_refs) = get_ref_list(&prop_def, 4) {
                    for prop_id in prop_refs {
                        if let Ok(prop) = decoder.decode_by_id(prop_id) {
                            // IfcPropertySingleValue: (Name, Description, NominalValue, Unit)
                            if prop.ifc_type == ifc_lite_core::IfcType::IfcPropertySingleValue {
                                let name = prop
                                    .get_string(0)
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();

                                // Get value - can be various types
                                let value = if let Some(val) = prop.get(2) {
                                    format_property_value(val)
                                } else {
                                    String::new()
                                };

                                // Get unit if present
                                let unit = prop.get_string(3).map(|s| s.to_string());

                                if !name.is_empty() {
                                    properties.push(PropertyValue { name, value, unit });
                                }
                            }
                        }
                    }
                }

                if !properties.is_empty() {
                    property_sets.push(PropertySet {
                        name: pset_name,
                        properties,
                    });
                }
            }
            ifc_lite_core::IfcType::IfcElementQuantity => {
                // IfcElementQuantity: (GlobalId, OwnerHistory, Name, Description, MethodOfMeasurement, Quantities)
                let qset_name = prop_def
                    .get_string(2)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("Quantities #{}", prop_def_id));

                // Get Quantities list (attribute 5)
                if let Some(qty_refs) = get_ref_list(&prop_def, 5) {
                    for qty_id in qty_refs {
                        if let Ok(qty) = decoder.decode_by_id(qty_id) {
                            // IfcPhysicalQuantity subtypes: Name, Description, ...values
                            let name = qty.get_string(0).map(|s| s.to_string()).unwrap_or_default();

                            // Apply unit scale: length * scale, area * scale², volume * scale³
                            let (value, unit, qty_type) = match qty.ifc_type {
                                ifc_lite_core::IfcType::IfcQuantityLength => {
                                    let val = qty.get_float(3).unwrap_or(0.0) * unit_scale;
                                    (val, "m".to_string(), "Length".to_string())
                                }
                                ifc_lite_core::IfcType::IfcQuantityArea => {
                                    let val = qty.get_float(3).unwrap_or(0.0) * unit_scale * unit_scale;
                                    (val, "m²".to_string(), "Area".to_string())
                                }
                                ifc_lite_core::IfcType::IfcQuantityVolume => {
                                    let val = qty.get_float(3).unwrap_or(0.0) * unit_scale * unit_scale * unit_scale;
                                    (val, "m³".to_string(), "Volume".to_string())
                                }
                                ifc_lite_core::IfcType::IfcQuantityCount => {
                                    let val = qty.get_float(3).unwrap_or(0.0);
                                    (val, "".to_string(), "Count".to_string())
                                }
                                ifc_lite_core::IfcType::IfcQuantityWeight => {
                                    let val = qty.get_float(3).unwrap_or(0.0);
                                    (val, "kg".to_string(), "Weight".to_string())
                                }
                                ifc_lite_core::IfcType::IfcQuantityTime => {
                                    let val = qty.get_float(3).unwrap_or(0.0);
                                    (val, "s".to_string(), "Time".to_string())
                                }
                                _ => continue,
                            };

                            if !name.is_empty() {
                                quantities.push(QuantityValue {
                                    name: format!("{}: {}", qset_name, name),
                                    value,
                                    unit,
                                    quantity_type: qty_type,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    (property_sets, quantities)
}

/// Format a property value for display
fn format_property_value(val: &ifc_lite_core::AttributeValue) -> String {
    match val {
        ifc_lite_core::AttributeValue::String(s) => s.clone(),
        ifc_lite_core::AttributeValue::Float(f) => format!("{:.4}", f),
        ifc_lite_core::AttributeValue::Integer(i) => i.to_string(),
        ifc_lite_core::AttributeValue::Enum(e) => e.clone(),
        ifc_lite_core::AttributeValue::List(items) => {
            // Check if this is a typed value (first element is type name)
            if items.len() >= 2 {
                if let ifc_lite_core::AttributeValue::String(type_name) = &items[0] {
                    let value_parts: Vec<String> = items[1..].iter().map(format_property_value).collect();
                    return format!("{}({})", type_name, value_parts.join(", "));
                }
            }
            let formatted: Vec<String> = items.iter().map(format_property_value).collect();
            format!("[{}]", formatted.join(", "))
        }
        ifc_lite_core::AttributeValue::EntityRef(id) => format!("#{}", id),
        ifc_lite_core::AttributeValue::Null => "".to_string(),
        ifc_lite_core::AttributeValue::Derived => "*".to_string(),
    }
}

/// Load model from cache and send to Bevy
fn load_from_cache(
    cached: bridge::CachedModel,
    state: crate::state::ViewerState,
    load_start: f64,
) -> Result<(), String> {
    state.loading.set_progress(Progress {
        phase: "Loading from cache".to_string(),
        percent: 50.0,
    });

    let geometry_count = cached.geometry.len();
    let entity_count = cached.entities.len();

    // Send geometry to Bevy
    bridge::save_geometry(cached.geometry);

    // Build entity infos from cached entity data
    let entity_infos: Vec<EntityInfo> = cached
        .entities
        .into_iter()
        .map(|e| EntityInfo {
            id: e.id,
            entity_type: e.entity_type,
            name: e.name,
            description: e.description,
            global_id: e.global_id,
            storey: e.storey,
            storey_elevation: e.storey_elevation,
            // Note: property_sets and quantities not cached (would be too large)
            property_sets: Vec::new(),
            quantities: Vec::new(),
        })
        .collect();

    // Load spatial tree if cached
    if let Some(tree_json) = cached.spatial_tree_json {
        if let Ok(tree) = serde_json::from_str::<SpatialNode>(&tree_json) {
            state.scene.set_spatial_tree(tree);
        }
    }

    // Load storeys if cached
    if let Some(storeys_json) = cached.storeys_json {
        if let Ok(storeys) = serde_json::from_str::<Vec<crate::state::StoreyInfo>>(&storeys_json) {
            state.scene.set_storeys(storeys);
        }
    }

    state.scene.set_entities(entity_infos);

    let total_time = now_ms() - load_start;
    bridge::log_info(&format!(
        "[IFC-Lite] Cache load complete: {:.2}s | {} entities, {} meshes",
        total_time / 1000.0,
        entity_count,
        geometry_count
    ));

    Ok(())
}

/// Parse IFC content and send geometry to Bevy
/// Uses localStorage cache for faster reloads of previously loaded files
pub fn parse_and_process_ifc(
    content: &str,
    state: crate::state::ViewerState,
) -> Result<(), String> {
    use ifc_lite_core::{build_entity_index, EntityDecoder, EntityScanner};
    use std::collections::HashSet;

    let load_start = now_ms();
    let file_size_mb = content.len() as f64 / (1024.0 * 1024.0);
    bridge::log_info(&format!(
        "[IFC-Lite] Starting load, file size: {:.2} MB",
        file_size_mb
    ));

    // Check cache first
    let file_hash = bridge::compute_file_hash(content);
    if let Some(cached) = bridge::load_cached_model(&file_hash) {
        bridge::log_info(&format!(
            "[IFC-Lite] Cache hit! Loading {} entities, {} meshes from cache",
            cached.entities.len(),
            cached.geometry.len()
        ));
        return load_from_cache(cached, state, load_start);
    }

    bridge::log_info("[IFC-Lite] Cache miss, parsing IFC file...");

    // Build entity index for O(1) lookups
    let index_start = now_ms();
    let index = build_entity_index(content);
    let entity_count = index.len();
    bridge::log_info(&format!(
        "[IFC-Lite] Entity index: {} entities in {:.0}ms",
        entity_count,
        now_ms() - index_start
    ));

    // Create decoder with pre-built index
    let mut decoder = EntityDecoder::with_index(content, index);

    state.loading.set_progress(Progress {
        phase: "Building spatial hierarchy".to_string(),
        percent: 10.0,
    });

    // First pass: collect spatial structure and property relationships
    let mut spatial_entities: HashMap<u32, SpatialInfo> = HashMap::new();
    let mut aggregates: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut contained_in: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut element_to_storey: HashMap<u32, u32> = HashMap::new();
    let mut project_id: Option<u32> = None;

    // Property relationships: element -> property definition IDs
    let mut element_properties: HashMap<u32, Vec<u32>> = HashMap::new();
    // Type relationships: element -> type object ID
    let mut element_to_type: HashMap<u32, u32> = HashMap::new();

    // Scan for spatial structure
    for line in content.lines() {
        let line = line.trim();
        if !line.starts_with('#') {
            continue;
        }

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

        match type_upper.as_str() {
            "IFCPROJECT" => {
                project_id = Some(id);
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Project".to_string());
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            id,
                            name,
                            entity_type: type_name.to_string(),
                            elevation: None,
                        },
                    );
                }
            }
            "IFCSITE" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Site".to_string());
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            id,
                            name,
                            entity_type: type_name.to_string(),
                            elevation: None,
                        },
                    );
                }
            }
            "IFCBUILDING" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Building".to_string());
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            id,
                            name,
                            entity_type: type_name.to_string(),
                            elevation: None,
                        },
                    );
                }
            }
            "IFCBUILDINGSTOREY" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Storey #{}", id));
                    let elevation = entity.get_float(9).map(|e| e as f32);
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            id,
                            name,
                            entity_type: type_name.to_string(),
                            elevation,
                        },
                    );
                }
            }
            "IFCSPACE" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let name = entity
                        .get_string(2)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Space #{}", id));
                    spatial_entities.insert(
                        id,
                        SpatialInfo {
                            id,
                            name,
                            entity_type: type_name.to_string(),
                            elevation: None,
                        },
                    );
                }
            }
            "IFCRELAGGREGATES" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    let parent_id = entity.get_ref(4);
                    let children = get_ref_list(&entity, 5);
                    if let (Some(parent_id), Some(children)) = (parent_id, children) {
                        aggregates.entry(parent_id).or_default().extend(children);
                    }
                }
            }
            "IFCRELCONTAINEDINSPATIALSTRUCTURE" => {
                if let Ok(entity) = decoder.decode_by_id(id) {
                    if let Some(structure_id) = entity.get_ref(5) {
                        if let Some(elements) = get_ref_list(&entity, 4) {
                            contained_in
                                .entry(structure_id)
                                .or_default()
                                .extend(elements.clone());
                            for elem_id in elements {
                                element_to_storey.insert(elem_id, structure_id);
                            }
                        }
                    }
                }
            }
            "IFCRELDEFINESBYPROPERTIES" => {
                // IfcRelDefinesByProperties: (GlobalId, OwnerHistory, Name, Description, RelatedObjects, RelatingPropertyDefinition)
                if let Ok(entity) = decoder.decode_by_id(id) {
                    if let Some(prop_def_id) = entity.get_ref(5) {
                        if let Some(related_objects) = get_ref_list(&entity, 4) {
                            for obj_id in related_objects {
                                element_properties
                                    .entry(obj_id)
                                    .or_default()
                                    .push(prop_def_id);
                            }
                        }
                    }
                }
            }
            "IFCRELDEFINESBYTYPE" => {
                // IfcRelDefinesByType: (GlobalId, OwnerHistory, Name, Description, RelatedObjects, RelatingType)
                if let Ok(entity) = decoder.decode_by_id(id) {
                    if let Some(type_id) = entity.get_ref(5) {
                        if let Some(related_objects) = get_ref_list(&entity, 4) {
                            for obj_id in related_objects {
                                element_to_type.insert(obj_id, type_id);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Extract unit scale
    let unit_scale = if let Some(proj_id) = project_id {
        match decoder.extract_unit_scale(proj_id) {
            Ok(scale) => {
                bridge::log(&format!("Unit scale: {}", scale));
                decoder.set_length_unit_scale(scale);
                scale as f32
            }
            Err(_) => {
                decoder.set_length_unit_scale(1.0);
                1.0
            }
        }
    } else {
        decoder.set_length_unit_scale(1.0);
        1.0
    };

    // Apply unit scale to elevations
    for info in spatial_entities.values_mut() {
        if let Some(ref mut elev) = info.elevation {
            *elev *= unit_scale;
        }
    }

    decoder.clear_cache();

    let spatial_time = now_ms() - load_start;
    bridge::log_info(&format!(
        "[IFC-Lite] Spatial structure parsed in {:.0}ms",
        spatial_time
    ));

    // Create geometry router
    let router = GeometryRouter::new();

    state.loading.set_progress(Progress {
        phase: "Processing geometry".to_string(),
        percent: 30.0,
    });

    // Second pass: process geometry
    let geometry_start = now_ms();
    let mut scanner = EntityScanner::new(content);
    let mut geometry_data: Vec<GeometryData> = Vec::new();
    let mut entity_data: Vec<EntityData> = Vec::new();
    let mut processed = 0;
    let mut _errors = 0;
    let mut total_vertices = 0usize;
    let mut total_triangles = 0usize;

    let color_palette = state.ui.color_palette.get_untracked();

    while let Some((id, type_name, _start, _end)) = scanner.next_entity() {
        if ifc_lite_core::has_geometry_by_name(type_name) {
            let ifc_type = ifc_lite_core::IfcType::from_str(type_name);
            if matches!(ifc_type, ifc_lite_core::IfcType::Unknown(_)) {
                continue;
            }

            match decoder.decode_by_id(id) {
                Ok(entity) => {
                    let global_id = entity.get_string(0).map(|s| s.to_string());
                    let name = entity.get_string(2).map(|s| s.to_string());
                    let description = entity.get_string(3).map(|s| s.to_string());

                    let (storey_name, storey_elevation) =
                        if let Some(&storey_id) = element_to_storey.get(&id) {
                            if let Some(storey) = spatial_entities.get(&storey_id) {
                                (Some(storey.name.clone()), storey.elevation)
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        };

                    entity_data.push(EntityData {
                        id: id as u64,
                        entity_type: type_name.to_string(),
                        name: name.clone(),
                        description: description.clone(),
                        global_id: global_id.clone(),
                        storey: storey_name,
                        storey_elevation,
                    });

                    match router.process_element(&entity, &mut decoder) {
                        Ok(mesh) => {
                            if !mesh.is_empty() {
                                let sanitize = |arr: &[f32]| -> Vec<f32> {
                                    arr.iter()
                                        .map(|v| if v.is_finite() { *v } else { 0.0 })
                                        .collect()
                                };

                                let positions = sanitize(&mesh.positions);
                                let normals = sanitize(&mesh.normals);
                                let indices = mesh.indices.clone();

                                if positions.iter().all(|v| *v == 0.0) {
                                    _errors += 1;
                                    continue;
                                }

                                let color = get_element_color(type_name, color_palette);
                                let transform = [
                                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
                                    0.0, 0.0, 0.0, 1.0,
                                ];

                                total_vertices += positions.len() / 3;
                                total_triangles += indices.len() / 3;

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
                        Err(_) => {
                            _errors += 1;
                        }
                    }
                }
                Err(_) => {
                    _errors += 1;
                }
            }

            if processed % 50 == 0 {
                let percent = 30.0 + (processed as f32 / entity_count as f32) * 50.0;
                state.loading.set_progress(Progress {
                    phase: format!("Processing geometry ({}/{})", processed, entity_count),
                    percent,
                });
            }
        }
    }

    let geometry_time = now_ms() - geometry_start;
    bridge::log_info(&format!(
        "[IFC-Lite] Geometry: {} meshes, {} vertices, {} triangles in {:.0}ms",
        processed, total_vertices, total_triangles, geometry_time
    ));

    state.loading.set_progress(Progress {
        phase: "Sending to viewer".to_string(),
        percent: 90.0,
    });

    // Track entities with geometry
    let geometry_count = geometry_data.len();
    let entities_with_geometry: HashSet<u64> =
        geometry_data.iter().map(|g| g.entity_id).collect();

    // Send geometry to Bevy (clone for cache)
    bridge::save_geometry(geometry_data.clone());
    bridge::log(&format!(
        "[Leptos] Transferred {} meshes to Bevy",
        geometry_count
    ));

    decoder.clear_cache();

    // Build entity lookup for tree building
    let entity_lookup: HashMap<u64, &EntityData> =
        entity_data.iter().map(|e| (e.id, e)).collect();

    // Build spatial tree
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

    fn build_node(
        id: u32,
        spatial_entities: &HashMap<u32, SpatialInfo>,
        aggregates: &HashMap<u32, Vec<u32>>,
        contained_in: &HashMap<u32, Vec<u32>>,
        entity_lookup: &HashMap<u64, &EntityData>,
        entities_with_geometry: &HashSet<u64>,
        get_node_type: &dyn Fn(&str) -> SpatialNodeType,
    ) -> Option<SpatialNode> {
        let info = spatial_entities.get(&id)?;
        let node_type = get_node_type(&info.entity_type);

        let mut children: Vec<SpatialNode> = Vec::new();

        if let Some(child_ids) = aggregates.get(&id) {
            for &child_id in child_ids {
                if let Some(child_node) = build_node(
                    child_id,
                    spatial_entities,
                    aggregates,
                    contained_in,
                    entity_lookup,
                    entities_with_geometry,
                    get_node_type,
                ) {
                    children.push(child_node);
                }
            }
        }

        if let Some(element_ids) = contained_in.get(&id) {
            for &elem_id in element_ids {
                if let Some(elem) = entity_lookup.get(&(elem_id as u64)) {
                    let has_geometry = entities_with_geometry.contains(&(elem_id as u64));
                    children.push(SpatialNode {
                        id: elem_id as u64,
                        node_type: SpatialNodeType::Element,
                        name: elem.display_label(),
                        entity_type: elem.entity_type.clone(),
                        elevation: None,
                        children: Vec::new(),
                        has_geometry,
                    });
                }
            }
        }

        children.sort_by(|a, b| {
            let a_is_spatial = !matches!(a.node_type, SpatialNodeType::Element);
            let b_is_spatial = !matches!(b.node_type, SpatialNodeType::Element);
            if a_is_spatial != b_is_spatial {
                return b_is_spatial.cmp(&a_is_spatial);
            }
            if matches!(a.node_type, SpatialNodeType::Storey)
                && matches!(b.node_type, SpatialNodeType::Storey)
            {
                return b
                    .elevation
                    .partial_cmp(&a.elevation)
                    .unwrap_or(std::cmp::Ordering::Equal);
            }
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
            has_geometry: false,
        })
    }

    let root_id = spatial_entities
        .iter()
        .find(|(_, info)| info.entity_type.to_uppercase() == "IFCPROJECT")
        .map(|(id, _)| *id);

    if let Some(root_id) = root_id {
        if let Some(tree) = build_node(
            root_id,
            &spatial_entities,
            &aggregates,
            &contained_in,
            &entity_lookup,
            &entities_with_geometry,
            &get_node_type,
        ) {
            state.scene.set_spatial_tree(tree);
        }
    }

    // Build entity infos with property extraction
    let entity_infos: Vec<EntityInfo> = entity_data
        .iter()
        .map(|e| {
            let (property_sets, quantities) = extract_properties_and_quantities(
                e.id as u32,
                &element_properties,
                &element_to_type,
                &mut decoder,
                unit_scale as f64,
            );
            EntityInfo {
                id: e.id,
                entity_type: e.entity_type.clone(),
                name: e.name.clone(),
                description: e.description.clone(),
                global_id: e.global_id.clone(),
                storey: e.storey.clone(),
                storey_elevation: e.storey_elevation,
                property_sets,
                quantities,
            }
        })
        .collect();

    // Build storey infos
    let mut storey_infos: Vec<crate::state::StoreyInfo> = spatial_entities
        .values()
        .filter(|s| s.entity_type.to_uppercase() == "IFCBUILDINGSTOREY")
        .map(|s| {
            let entity_count = entity_data
                .iter()
                .filter(|e| e.storey.as_ref() == Some(&s.name))
                .count();
            crate::state::StoreyInfo {
                name: s.name.clone(),
                elevation: s.elevation.unwrap_or(0.0),
                entity_count,
            }
        })
        .collect();
    storey_infos.sort_by(|a, b| {
        b.elevation
            .partial_cmp(&a.elevation)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    state.scene.set_entities(entity_infos);
    state.scene.set_storeys(storey_infos.clone());

    // Save to cache for faster reload next time
    let spatial_tree_json = state.scene.spatial_tree.get_untracked()
        .map(|tree| serde_json::to_string(&tree).ok())
        .flatten();
    let storeys_json = serde_json::to_string(&storey_infos).ok();

    let file_name = state.scene.file_name.get_untracked().unwrap_or_default();
    let cached_model = bridge::CachedModel {
        geometry: geometry_data,
        entities: entity_data,
        spatial_tree_json,
        storeys_json,
    };
    bridge::save_model_to_cache(&file_hash, &file_name, &cached_model);

    let total_time = now_ms() - load_start;
    bridge::log_info(&format!(
        "[IFC-Lite] Load complete: {:.2}s | {} entities, {} meshes",
        total_time / 1000.0,
        entity_count,
        geometry_count
    ));

    Ok(())
}

/// Get color for element type based on palette
fn get_element_color(entity_type: &str, palette: crate::bridge::ColorPalette) -> [f32; 4] {
    match palette {
        crate::bridge::ColorPalette::Vibrant => get_vibrant_color(entity_type),
        crate::bridge::ColorPalette::Realistic => get_realistic_color(entity_type),
        crate::bridge::ColorPalette::HighContrast => get_high_contrast_color(entity_type),
        crate::bridge::ColorPalette::Monochrome => get_monochrome_color(entity_type),
    }
}

fn get_vibrant_color(entity_type: &str) -> [f32; 4] {
    let upper = entity_type.to_uppercase();
    if upper.contains("WALL") {
        [0.95, 0.90, 0.80, 1.0]
    } else if upper.contains("SLAB") {
        [0.85, 0.82, 0.78, 1.0]
    } else if upper.contains("ROOF") {
        [0.85, 0.45, 0.35, 1.0]
    } else if upper.contains("BEAM") || upper.contains("COLUMN") || upper.contains("MEMBER") {
        [0.45, 0.55, 0.75, 1.0]
    } else if upper.contains("DOOR") {
        [0.65, 0.40, 0.25, 1.0]
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        [0.4, 0.7, 0.9, 0.4]
    } else if upper.contains("STAIR") || upper.contains("RAMP") {
        [0.75, 0.70, 0.65, 1.0]
    } else if upper.contains("RAILING") {
        [0.30, 0.30, 0.35, 1.0]
    } else if upper.contains("FURNITURE") || upper.contains("FURNISHING") {
        [0.70, 0.50, 0.30, 1.0]
    } else if upper.contains("SPACE") {
        [0.7, 0.85, 0.95, 0.15]
    } else {
        [0.80, 0.78, 0.75, 1.0]
    }
}

fn get_realistic_color(entity_type: &str) -> [f32; 4] {
    let upper = entity_type.to_uppercase();
    if upper.contains("WALL") {
        [0.92, 0.85, 0.75, 1.0]
    } else if upper.contains("SLAB") {
        [0.75, 0.73, 0.70, 1.0]
    } else if upper.contains("ROOF") {
        [0.72, 0.55, 0.45, 1.0]
    } else if upper.contains("BEAM") || upper.contains("COLUMN") || upper.contains("MEMBER") {
        [0.60, 0.65, 0.72, 1.0]
    } else if upper.contains("DOOR") {
        [0.55, 0.35, 0.20, 1.0]
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        [0.5, 0.7, 0.85, 0.35]
    } else {
        [0.75, 0.72, 0.70, 1.0]
    }
}

fn get_high_contrast_color(entity_type: &str) -> [f32; 4] {
    let upper = entity_type.to_uppercase();
    if upper.contains("WALL") {
        [1.0, 0.95, 0.85, 1.0]
    } else if upper.contains("SLAB") {
        [0.7, 0.7, 0.7, 1.0]
    } else if upper.contains("ROOF") {
        [0.9, 0.3, 0.2, 1.0]
    } else if upper.contains("BEAM") || upper.contains("COLUMN") || upper.contains("MEMBER") {
        [0.2, 0.4, 0.8, 1.0]
    } else if upper.contains("DOOR") {
        [0.6, 0.3, 0.1, 1.0]
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        [0.3, 0.7, 1.0, 0.5]
    } else {
        [0.85, 0.85, 0.85, 1.0]
    }
}

fn get_monochrome_color(entity_type: &str) -> [f32; 4] {
    let upper = entity_type.to_uppercase();
    if upper.contains("WALL") {
        [0.85, 0.85, 0.85, 1.0]
    } else if upper.contains("SLAB") {
        [0.70, 0.70, 0.70, 1.0]
    } else if upper.contains("ROOF") {
        [0.60, 0.60, 0.60, 1.0]
    } else if upper.contains("BEAM") || upper.contains("COLUMN") || upper.contains("MEMBER") {
        [0.50, 0.50, 0.50, 1.0]
    } else if upper.contains("DOOR") {
        [0.40, 0.40, 0.40, 1.0]
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        [0.75, 0.75, 0.75, 0.4]
    } else {
        [0.75, 0.75, 0.75, 1.0]
    }
}
