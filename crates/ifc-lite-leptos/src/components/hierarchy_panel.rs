//! Hierarchy panel - entity tree view with virtual scrolling

use crate::bridge;
use crate::components::toolbar::parse_and_process_ifc;
use crate::state::{use_viewer_state, Progress, SpatialNode, SpatialNodeType, ViewerState};
use crate::utils::get_entity_icon;
use leptos::prelude::*;
use rustc_hash::FxHashSet;
use wasm_bindgen_futures::spawn_local;

/// Row height in pixels (must match CSS)
const ROW_HEIGHT: f64 = 28.0;
/// Number of extra rows to render above/below viewport
const OVERSCAN: usize = 5;

/// Get icon for spatial node type
fn get_node_icon(node_type: &SpatialNodeType, entity_type: &str) -> &'static str {
    match node_type {
        SpatialNodeType::Project => "📋",
        SpatialNodeType::Site => "🌍",
        SpatialNodeType::Building => "🏢",
        SpatialNodeType::Storey => "📐",
        SpatialNodeType::Space => "🚪",
        SpatialNodeType::Element => get_entity_icon(entity_type),
    }
}

/// Flattened tree row for virtual scrolling
#[derive(Clone, PartialEq)]
struct FlatRow {
    id: u64,
    name: String,
    entity_type: String,
    node_type: SpatialNodeType,
    depth: usize,
    has_children: bool,
    has_geometry: bool,
    child_count: usize,
}

/// Flatten tree into visible rows based on expanded state
fn flatten_tree(
    node: &SpatialNode,
    depth: usize,
    expanded: &FxHashSet<u64>,
    search_query: &str,
    rows: &mut Vec<FlatRow>,
) {
    // Filter check for search
    if !search_query.is_empty() {
        let query = search_query.to_lowercase();
        fn matches_query(n: &SpatialNode, q: &str) -> bool {
            n.name.to_lowercase().contains(q)
                || n.entity_type.to_lowercase().contains(q)
                || n.children.iter().any(|c| matches_query(c, q))
        }
        if !matches_query(node, &query) {
            return;
        }
    }

    let is_expanded = expanded.contains(&node.id);

    // Count visible children
    let visible_children: Vec<_> = if search_query.is_empty() {
        node.children.iter().collect()
    } else {
        let query = search_query.to_lowercase();
        node.children
            .iter()
            .filter(|child| {
                fn matches_query(n: &SpatialNode, q: &str) -> bool {
                    n.name.to_lowercase().contains(q)
                        || n.entity_type.to_lowercase().contains(q)
                        || n.children.iter().any(|c| matches_query(c, q))
                }
                matches_query(child, &query)
            })
            .collect()
    };

    rows.push(FlatRow {
        id: node.id,
        name: node.name.clone(),
        entity_type: node.entity_type.clone(),
        node_type: node.node_type.clone(),
        depth,
        has_children: !visible_children.is_empty(),
        has_geometry: node.has_geometry,
        child_count: visible_children.len(),
    });

    if is_expanded {
        for child in visible_children {
            flatten_tree(child, depth + 1, expanded, search_query, rows);
        }
    }
}

/// Hierarchy panel component with virtual scrolling
#[component]
pub fn HierarchyPanel() -> impl IntoView {
    let state = use_viewer_state();
    let is_dragging = RwSignal::new(false);
    let scroll_top = RwSignal::new(0.0_f64);
    let container_height = RwSignal::new(400.0_f64);
    let scroll_container_ref = NodeRef::<leptos::html::Div>::new();

    // Flatten tree and compute visible range
    let flattened_rows = Memo::new(move |_| {
        let tree = state.scene.spatial_tree.get();
        let expanded = state.scene.expanded_nodes.get();
        let search = state.ui.search_query.get();

        tree.map(|t| {
            let mut rows = Vec::new();
            flatten_tree(&t, 0, &expanded, &search, &mut rows);
            rows
        })
        .unwrap_or_default()
    });

    let total_height = Memo::new(move |_| flattened_rows.get().len() as f64 * ROW_HEIGHT);

    let visible_rows = Memo::new(move |_| {
        let rows = flattened_rows.get();
        let scroll = scroll_top.get();
        let height = container_height.get();

        let start_idx = ((scroll / ROW_HEIGHT) as usize).saturating_sub(OVERSCAN);
        let visible_count = ((height / ROW_HEIGHT) as usize) + OVERSCAN * 2;
        let end_idx = (start_idx + visible_count).min(rows.len());

        rows[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(i, row)| (start_idx + i, row.clone()))
            .collect::<Vec<_>>()
    });

    // Handle scroll
    let on_scroll = move |_| {
        if let Some(element) = scroll_container_ref.get() {
            let el: &web_sys::Element = element.as_ref();
            scroll_top.set(el.scroll_top() as f64);
            container_height.set(el.client_height() as f64);
        }
    };

    // Update container height on mount
    Effect::new(move |_| {
        if let Some(element) = scroll_container_ref.get() {
            let el: &web_sys::Element = element.as_ref();
            container_height.set(el.client_height() as f64);
        }
    });

    // Drag and drop handlers
    let on_dragover = move |ev: leptos::ev::DragEvent| {
        ev.prevent_default();
        is_dragging.set(true);
    };

    let on_dragleave = move |ev: leptos::ev::DragEvent| {
        ev.prevent_default();
        is_dragging.set(false);
    };

    let on_drop = move |ev: leptos::ev::DragEvent| {
        ev.prevent_default();
        is_dragging.set(false);

        if let Some(data_transfer) = ev.data_transfer() {
            if let Some(files) = data_transfer.files() {
                if let Some(file) = files.get(0) {
                    let name = file.name().to_lowercase();
                    if name.ends_with(".ifc") {
                        load_file(file, state);
                    } else {
                        bridge::log_error("Please drop an IFC file (.ifc)");
                    }
                }
            }
        }
    };

    // Callbacks
    let on_toggle = move |id: u64| {
        state.scene.toggle_node_expanded(id);
    };

    let on_select = move |id: u64| {
        state.selection.select(id);
    };

    let on_toggle_visibility = move |id: u64| {
        state.visibility.toggle_visibility(id);
    };

    view! {
        <div
            class=move || if is_dragging.get() { "hierarchy-panel drag-over" } else { "hierarchy-panel" }
            on:dragover=on_dragover
            on:dragleave=on_dragleave
            on:drop=on_drop
        >
            // Search bar
            <div class="search-bar">
                <input
                    type="text"
                    class="search-input"
                    placeholder="Search entities..."
                    prop:value=move || state.ui.search_query.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        state.ui.set_search(value);
                    }
                />
                {move || {
                    let query = state.ui.search_query.get();
                    if !query.is_empty() {
                        Some(view! {
                            <button
                                class="search-clear"
                                on:click=move |_| state.ui.set_search(String::new())
                            >
                                "×"
                            </button>
                        })
                    } else {
                        None
                    }
                }}
            </div>

            // Expand/collapse controls + count
            {move || {
                state.scene.spatial_tree.get().map(|_| view! {
                    <div class="tree-controls">
                        <span class="tree-count-total">
                            {move || format!("{} items", flattened_rows.get().len())}
                        </span>
                        <button
                            class="tree-control-btn"
                            on:click=move |_| state.scene.expand_all()
                            title="Expand all"
                        >
                            "⊞"
                        </button>
                        <button
                            class="tree-control-btn"
                            on:click=move |_| state.scene.collapse_all()
                            title="Collapse all"
                        >
                            "⊟"
                        </button>
                    </div>
                })
            }}

            // Entity tree with virtual scrolling
            <div
                class="entity-list"
                node_ref=scroll_container_ref
                on:scroll=on_scroll
            >
                {move || {
                    let has_tree = state.scene.spatial_tree.get().is_some();
                    let has_entities = !state.scene.entities.get().is_empty();

                    if !has_tree && !has_entities {
                        // Empty state
                        view! {
                            <div class=move || if is_dragging.get() { "empty-state drop-zone active" } else { "empty-state drop-zone" }>
                                <span class="empty-icon">{move || if is_dragging.get() { "📥" } else { "📂" }}</span>
                                <span class="empty-text">{move || if is_dragging.get() { "Drop IFC file here" } else { "No model loaded" }}</span>
                                <span class="empty-hint">"Drag & drop an IFC file or use the toolbar"</span>
                            </div>
                        }.into_any()
                    } else if has_tree {
                        // Virtual scrolling tree
                        view! {
                            <div
                                class="virtual-scroll-content"
                                style=move || format!("height: {}px; position: relative;", total_height.get())
                            >
                                <For
                                    each=move || visible_rows.get()
                                    key=|(idx, row)| (*idx, row.id)
                                    children=move |(idx, row)| {
                                        let top = idx as f64 * ROW_HEIGHT;
                                        let row_id = row.id;
                                        let _is_element = matches!(row.node_type, SpatialNodeType::Element);

                                        // Create derived signals for the row
                                        let is_expanded = Signal::derive(move || state.scene.expanded_nodes.get().contains(&row_id));
                                        let is_selected = Signal::derive(move || state.selection.selected_ids.get().contains(&row_id));
                                        let is_hidden = Signal::derive(move || state.visibility.hidden_ids.get().contains(&row_id));

                                        view! {
                                            <TreeRow
                                                row=row
                                                top=top
                                                is_expanded=is_expanded
                                                is_selected=is_selected
                                                is_hidden=is_hidden
                                                on_toggle=on_toggle
                                                on_select=on_select
                                                on_toggle_visibility=on_toggle_visibility
                                            />
                                        }
                                    }
                                />
                            </div>
                        }.into_any()
                    } else {
                        // Flat list fallback
                        view! {
                            <div class="flat-list">
                                <For
                                    each=move || state.scene.entities.get()
                                    key=|entity| entity.id
                                    children=move |entity| {
                                        let entity_id = entity.id;
                                        view! {
                                            <div
                                                class=move || {
                                                    let selected = state.selection.selected_ids.get().contains(&entity_id);
                                                    let hidden = state.visibility.hidden_ids.get().contains(&entity_id);
                                                    let mut classes = "entity-row".to_string();
                                                    if selected { classes.push_str(" selected"); }
                                                    if hidden { classes.push_str(" hidden"); }
                                                    classes
                                                }
                                                on:click=move |_| state.selection.select(entity_id)
                                            >
                                                <span class="entity-icon">
                                                    {get_entity_icon(&entity.entity_type)}
                                                </span>
                                                <span class="entity-name">
                                                    {entity.display_label()}
                                                </span>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Single tree row component
#[component]
fn TreeRow(
    row: FlatRow,
    top: f64,
    #[prop(into)] is_expanded: Signal<bool>,
    #[prop(into)] is_selected: Signal<bool>,
    #[prop(into)] is_hidden: Signal<bool>,
    on_toggle: impl Fn(u64) + Copy + 'static,
    on_select: impl Fn(u64) + Copy + 'static,
    on_toggle_visibility: impl Fn(u64) + Copy + 'static,
) -> impl IntoView {
    let is_element = matches!(row.node_type, SpatialNodeType::Element);
    let row_id = row.id;

    view! {
        <div
            class=move || {
                let mut classes = "tree-row".to_string();
                if is_selected.get() { classes.push_str(" selected"); }
                if is_hidden.get() { classes.push_str(" hidden"); }
                if !row.has_geometry && is_element { classes.push_str(" no-geometry"); }
                classes
            }
            style=move || format!(
                "position: absolute; top: {}px; width: 100%; padding-left: {}px;",
                top,
                8 + row.depth * 16
            )
        >
            // Expand/collapse toggle
            <span
                class=move || if row.has_children { "tree-toggle" } else { "tree-toggle empty" }
                on:click=move |ev| {
                    ev.stop_propagation();
                    on_toggle(row_id);
                }
            >
                {move || {
                    if row.has_children {
                        if is_expanded.get() { "▼" } else { "▶" }
                    } else {
                        ""
                    }
                }}
            </span>

            // Icon
            <span class="tree-icon">{get_node_icon(&row.node_type, &row.entity_type)}</span>

            // Name
            <span
                class="tree-name"
                on:click=move |_| {
                    if is_element {
                        on_select(row_id);
                    } else {
                        on_toggle(row_id);
                    }
                }
            >
                {row.name.clone()}
            </span>

            // Child count badge
            {if row.child_count > 0 && !is_element {
                Some(view! {
                    <span class="tree-count">{row.child_count}</span>
                })
            } else {
                None
            }}

            // Visibility toggle for elements
            {if is_element && row.has_geometry {
                Some(view! {
                    <button
                        class=move || if is_hidden.get() { "visibility-btn hidden" } else { "visibility-btn" }
                        on:click=move |ev| {
                            ev.stop_propagation();
                            on_toggle_visibility(row_id);
                        }
                        title=move || if is_hidden.get() { "Show" } else { "Hide" }
                    >
                        {move || if is_hidden.get() { "👁‍🗨" } else { "👁" }}
                    </button>
                })
            } else {
                None
            }}
        </div>
    }
}

/// Load a file (shared between drag-drop and toolbar)
fn load_file(file: web_sys::File, state: ViewerState) {
    let file_name = file.name();
    state.scene.set_file_name(file_name.clone());
    state.loading.set_loading(true);
    state.loading.set_progress(Progress {
        phase: "Reading file".to_string(),
        percent: 0.0,
    });

    bridge::log(&format!("Loading file: {}", file_name));

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
                        bridge::save_camera_cmd(&bridge::CameraCommand {
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
