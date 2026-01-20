//! Hierarchy panel - entity tree view with virtual scrolling

use crate::bridge;
use crate::components::toolbar::parse_and_process_ifc;
use crate::state::{Progress, SpatialNode, SpatialNodeType, ViewerAction, ViewerStateContext};
use gloo_file::callbacks::FileReader;
use std::collections::HashSet;
use wasm_bindgen_futures::spawn_local;
use web_sys::{DragEvent, Element, HtmlInputElement};
use yew::prelude::*;

/// Row height in pixels (must match CSS)
const ROW_HEIGHT: f64 = 28.0;
/// Number of extra rows to render above/below viewport for smooth scrolling
const OVERSCAN: usize = 5;

/// Get icon for spatial node type
fn get_node_icon(node_type: &SpatialNodeType, entity_type: &str) -> &'static str {
    match node_type {
        SpatialNodeType::Project => "📋",
        SpatialNodeType::Site => "🌍",
        SpatialNodeType::Building => "🏢",
        SpatialNodeType::Storey => "📐",
        SpatialNodeType::Space => "🚪",
        SpatialNodeType::Element => crate::utils::get_entity_icon(entity_type),
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
    expanded: &HashSet<u64>,
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

    // Count visible children (respecting search filter)
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

    // Recurse into children if expanded
    if is_expanded {
        for child in visible_children {
            flatten_tree(child, depth + 1, expanded, search_query, rows);
        }
    }
}

/// Single row component (memoized for performance)
#[derive(Properties, PartialEq)]
struct RowProps {
    row: FlatRow,
    is_expanded: bool,
    is_selected: bool,
    is_hidden: bool,
    on_toggle: Callback<u64>,
    on_select: Callback<u64>,
    on_toggle_visibility: Callback<u64>,
}

#[function_component]
fn TreeRow(props: &RowProps) -> Html {
    let row = &props.row;
    let is_element = matches!(row.node_type, SpatialNodeType::Element);

    let on_toggle_click = {
        let on_toggle = props.on_toggle.clone();
        let id = row.id;
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_toggle.emit(id);
        })
    };

    let on_name_click = {
        let on_select = props.on_select.clone();
        let on_toggle = props.on_toggle.clone();
        let id = row.id;
        let is_elem = is_element;
        Callback::from(move |_| {
            if is_elem {
                on_select.emit(id);
            } else {
                on_toggle.emit(id);
            }
        })
    };

    let on_visibility_click = {
        let on_toggle_visibility = props.on_toggle_visibility.clone();
        let id = row.id;
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_toggle_visibility.emit(id);
        })
    };

    html! {
        <div
            class={classes!(
                "tree-row",
                props.is_selected.then_some("selected"),
                props.is_hidden.then_some("hidden"),
                (!row.has_geometry && is_element).then_some("no-geometry")
            )}
            style={format!("padding-left: {}px;", 8 + row.depth * 16)}
        >
            // Expand/collapse toggle
            <span
                class={classes!("tree-toggle", (!row.has_children).then_some("empty"))}
                onclick={on_toggle_click}
            >
                {if row.has_children {
                    if props.is_expanded { "▼" } else { "▶" }
                } else {
                    ""
                }}
            </span>

            // Icon
            <span class="tree-icon">{get_node_icon(&row.node_type, &row.entity_type)}</span>

            // Name
            <span class="tree-name" onclick={on_name_click}>
                {&row.name}
            </span>

            // Child count badge
            if row.child_count > 0 && !is_element {
                <span class="tree-count">{row.child_count}</span>
            }

            // Visibility toggle for elements
            if is_element && row.has_geometry {
                <button
                    class={classes!("visibility-btn", props.is_hidden.then_some("hidden"))}
                    onclick={on_visibility_click}
                    title={if props.is_hidden { "Show" } else { "Hide" }}
                >
                    {if props.is_hidden { "👁‍🗨" } else { "👁" }}
                </button>
            }
        </div>
    }
}

/// Hierarchy panel component with virtual scrolling
#[function_component]
pub fn HierarchyPanel() -> Html {
    let state = use_context::<ViewerStateContext>().expect("ViewerStateContext not found");
    let is_dragging = use_state(|| false);
    let file_reader = use_state(|| None::<FileReader>);
    let scroll_top = use_state(|| 0.0_f64);
    let container_height = use_state(|| 400.0_f64);
    let scroll_container_ref = use_node_ref();

    // Handle scroll events
    let onscroll = {
        let scroll_top = scroll_top.clone();
        let container_height = container_height.clone();
        let scroll_container_ref = scroll_container_ref.clone();
        Callback::from(move |_: Event| {
            if let Some(element) = scroll_container_ref.cast::<Element>() {
                scroll_top.set(element.scroll_top() as f64);
                container_height.set(element.client_height() as f64);
            }
        })
    };

    // Update container height on mount
    {
        let container_height = container_height.clone();
        let scroll_container_ref = scroll_container_ref.clone();
        use_effect_with((), move |_| {
            if let Some(element) = scroll_container_ref.cast::<Element>() {
                container_height.set(element.client_height() as f64);
            }
            || ()
        });
    }

    // Handle file loading (shared between drag-drop and click)
    let load_file = {
        let state = state.clone();
        let file_reader = file_reader.clone();
        Callback::from(move |file: web_sys::File| {
            let file_name = file.name();
            state.dispatch(ViewerAction::SetFileName(file_name.clone()));
            state.dispatch(ViewerAction::SetLoading(true));
            state.dispatch(ViewerAction::SetProgress(Progress {
                phase: "Reading file".to_string(),
                percent: 0.0,
            }));

            bridge::log(&format!("Loading file: {}", file_name));

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

                        let content = String::from_utf8_lossy(&bytes).to_string();
                        let state_inner = state_clone.clone();
                        spawn_local(async move {
                            match parse_and_process_ifc(&content, &state_inner) {
                                Ok(_) => {
                                    bridge::log_info("IFC file loaded successfully");
                                    state_inner.dispatch(ViewerAction::SetLoading(false));
                                    state_inner.dispatch(ViewerAction::ClearProgress);
                                    // Trigger "Fit All" to frame the loaded model
                                    bridge::save_camera_cmd(&bridge::CameraCommand {
                                        cmd: "fit_all".to_string(),
                                        mode: None,
                                    });
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
        })
    };

    // Drag and drop handlers
    let ondragover = {
        let is_dragging = is_dragging.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            is_dragging.set(true);
        })
    };

    let ondragleave = {
        let is_dragging = is_dragging.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            is_dragging.set(false);
        })
    };

    let ondrop = {
        let is_dragging = is_dragging.clone();
        let load_file = load_file.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            is_dragging.set(false);

            if let Some(data_transfer) = e.data_transfer() {
                if let Some(files) = data_transfer.files() {
                    if let Some(file) = files.get(0) {
                        // Check if it's an IFC file
                        let name = file.name().to_lowercase();
                        if name.ends_with(".ifc") {
                            load_file.emit(file);
                        } else {
                            bridge::log_error("Please drop an IFC file (.ifc)");
                        }
                    }
                }
            }
        })
    };

    // Callbacks for row interactions
    let on_toggle = {
        let state = state.clone();
        Callback::from(move |id: u64| {
            state.dispatch(ViewerAction::ToggleNodeExpanded(id));
        })
    };

    let on_select = {
        let state = state.clone();
        Callback::from(move |id: u64| {
            state.dispatch(ViewerAction::Select(id));
        })
    };

    let on_toggle_visibility = {
        let state = state.clone();
        Callback::from(move |id: u64| {
            state.dispatch(ViewerAction::ToggleVisibility(id));
        })
    };

    // Flatten tree and compute visible range
    let (rows, total_height, visible_rows) = if let Some(ref tree) = state.spatial_tree {
        let mut rows = Vec::new();
        flatten_tree(
            tree,
            0,
            &state.expanded_nodes,
            &state.search_query,
            &mut rows,
        );

        let total_height = rows.len() as f64 * ROW_HEIGHT;
        let start_idx = ((*scroll_top / ROW_HEIGHT) as usize).saturating_sub(OVERSCAN);
        let visible_count = ((*container_height / ROW_HEIGHT) as usize) + OVERSCAN * 2;
        let end_idx = (start_idx + visible_count).min(rows.len());

        let visible: Vec<_> = rows[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(i, row)| (start_idx + i, row.clone()))
            .collect();

        (rows, total_height, visible)
    } else {
        (Vec::new(), 0.0, Vec::new())
    };

    let spacer_top = if !visible_rows.is_empty() {
        visible_rows[0].0 as f64 * ROW_HEIGHT
    } else {
        0.0
    };

    let spacer_bottom = if !visible_rows.is_empty() {
        let last_idx = visible_rows.last().map(|(i, _)| *i).unwrap_or(0);
        ((rows.len() - 1 - last_idx) as f64 * ROW_HEIGHT).max(0.0)
    } else {
        0.0
    };

    html! {
        <div
            class={classes!("hierarchy-panel", (*is_dragging).then_some("drag-over"))}
            ondragover={ondragover}
            ondragleave={ondragleave}
            ondrop={ondrop}
        >
            // Search bar
            <div class="search-bar">
                <input
                    type="text"
                    class="search-input"
                    placeholder="Search entities..."
                    value={state.search_query.clone()}
                    oninput={
                        let state = state.clone();
                        Callback::from(move |e: InputEvent| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            state.dispatch(ViewerAction::SetSearchQuery(input.value()));
                        })
                    }
                />
                if !state.search_query.is_empty() {
                    <button
                        class="search-clear"
                        onclick={
                            let state = state.clone();
                            Callback::from(move |_| {
                                state.dispatch(ViewerAction::SetSearchQuery(String::new()));
                            })
                        }
                    >
                        {"×"}
                    </button>
                }
            </div>

            // Expand/collapse all buttons + entity count
            if state.spatial_tree.is_some() {
                <div class="tree-controls">
                    <span class="tree-count-total">{format!("{} items", rows.len())}</span>
                    <button
                        class="tree-control-btn"
                        onclick={
                            let state = state.clone();
                            Callback::from(move |_| {
                                state.dispatch(ViewerAction::ExpandAll);
                            })
                        }
                        title="Expand all"
                    >
                        {"⊞"}
                    </button>
                    <button
                        class="tree-control-btn"
                        onclick={
                            let state = state.clone();
                            Callback::from(move |_| {
                                state.dispatch(ViewerAction::CollapseAll);
                            })
                        }
                        title="Collapse all"
                    >
                        {"⊟"}
                    </button>
                </div>
            }

            // Entity tree with virtual scrolling
            <div
                class="entity-list"
                ref={scroll_container_ref}
                onscroll={onscroll}
            >
                if state.spatial_tree.is_none() && state.entities.is_empty() {
                    <div class={classes!("empty-state", "drop-zone", (*is_dragging).then_some("active"))}>
                        <span class="empty-icon">{if *is_dragging { "📥" } else { "📂" }}</span>
                        <span class="empty-text">{if *is_dragging { "Drop IFC file here" } else { "No model loaded" }}</span>
                        <span class="empty-hint">{"Drag & drop an IFC file or use the toolbar"}</span>
                    </div>
                } else if state.spatial_tree.is_some() {
                    // Virtual scrolling container
                    <div class="virtual-scroll-content" style={format!("height: {}px;", total_height)}>
                        // Top spacer
                        <div style={format!("height: {}px;", spacer_top)} />

                        // Visible rows
                        { for visible_rows.iter().map(|(_, row)| {
                            let is_expanded = state.expanded_nodes.contains(&row.id);
                            let is_selected = state.selected_ids.contains(&row.id);
                            let is_hidden = state.hidden_ids.contains(&row.id);

                            html! {
                                <TreeRow
                                    row={row.clone()}
                                    is_expanded={is_expanded}
                                    is_selected={is_selected}
                                    is_hidden={is_hidden}
                                    on_toggle={on_toggle.clone()}
                                    on_select={on_select.clone()}
                                    on_toggle_visibility={on_toggle_visibility.clone()}
                                />
                            }
                        })}

                        // Bottom spacer
                        <div style={format!("height: {}px;", spacer_bottom)} />
                    </div>
                } else {
                    // Fallback to flat list if no tree (also virtualized)
                    <div class="flat-list">
                        { for state.entities.iter().map(|entity| {
                            let is_selected = state.selected_ids.contains(&entity.id);
                            let is_hidden = state.hidden_ids.contains(&entity.id);
                            let entity_id = entity.id;

                            html! {
                                <div
                                    class={classes!(
                                        "entity-row",
                                        is_selected.then_some("selected"),
                                        is_hidden.then_some("hidden")
                                    )}
                                    onclick={
                                        let state = state.clone();
                                        Callback::from(move |_| {
                                            state.dispatch(ViewerAction::Select(entity_id));
                                        })
                                    }
                                >
                                    <span class="entity-icon">
                                        {crate::utils::get_entity_icon(&entity.entity_type)}
                                    </span>
                                    <span class="entity-name">
                                        {entity.display_label()}
                                    </span>
                                </div>
                            }
                        })}
                    </div>
                }
            </div>
        </div>
    }
}
