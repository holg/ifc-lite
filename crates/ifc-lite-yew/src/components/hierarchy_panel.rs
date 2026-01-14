//! Hierarchy panel - entity tree view

use yew::prelude::*;
use web_sys::HtmlInputElement;
use crate::state::{ViewerAction, ViewerStateContext, SpatialNode, SpatialNodeType};

/// Get icon for spatial node type
fn get_node_icon(node: &SpatialNode) -> &'static str {
    match node.node_type {
        SpatialNodeType::Project => "📋",
        SpatialNodeType::Site => "🌍",
        SpatialNodeType::Building => "🏢",
        SpatialNodeType::Storey => "📐",
        SpatialNodeType::Space => "🚪",
        SpatialNodeType::Element => crate::utils::get_entity_icon(&node.entity_type),
    }
}

/// Recursive tree node component
#[derive(Properties, PartialEq)]
pub struct TreeNodeProps {
    pub node: SpatialNode,
    pub depth: usize,
}

#[function_component]
pub fn TreeNode(props: &TreeNodeProps) -> Html {
    let state = use_context::<ViewerStateContext>().expect("ViewerStateContext not found");
    let node = &props.node;
    let depth = props.depth;

    let is_expanded = state.expanded_nodes.contains(&node.id);
    let is_selected = state.selected_ids.contains(&node.id);
    let is_hidden = state.hidden_ids.contains(&node.id);
    let has_children = !node.children.is_empty();
    let is_element = matches!(node.node_type, SpatialNodeType::Element);

    // Filter children based on search query
    let filtered_children: Vec<_> = if state.search_query.is_empty() {
        node.children.clone()
    } else {
        let query = state.search_query.to_lowercase();
        node.children.iter()
            .filter(|child| {
                // Include if name or type matches, or if any descendant matches
                fn matches_query(n: &SpatialNode, q: &str) -> bool {
                    n.name.to_lowercase().contains(q)
                        || n.entity_type.to_lowercase().contains(q)
                        || n.children.iter().any(|c| matches_query(c, q))
                }
                matches_query(child, &query)
            })
            .cloned()
            .collect()
    };

    let child_count = filtered_children.len();

    html! {
        <div class="tree-node" style={format!("--depth: {}", depth)}>
            <div
                class={classes!(
                    "tree-row",
                    is_selected.then_some("selected"),
                    is_hidden.then_some("hidden"),
                    (!node.has_geometry && is_element).then_some("no-geometry")
                )}
            >
                // Expand/collapse toggle
                <span
                    class={classes!("tree-toggle", (!has_children).then_some("empty"))}
                    onclick={
                        let state = state.clone();
                        let node_id = node.id;
                        Callback::from(move |e: MouseEvent| {
                            e.stop_propagation();
                            state.dispatch(ViewerAction::ToggleNodeExpanded(node_id));
                        })
                    }
                >
                    {if has_children {
                        if is_expanded { "▼" } else { "▶" }
                    } else {
                        ""
                    }}
                </span>

                // Icon
                <span class="tree-icon">{get_node_icon(node)}</span>

                // Name (clickable for selection)
                <span
                    class="tree-name"
                    onclick={
                        let state = state.clone();
                        let node_id = node.id;
                        let is_element = is_element;
                        Callback::from(move |_| {
                            if is_element {
                                state.dispatch(ViewerAction::Select(node_id));
                            } else {
                                state.dispatch(ViewerAction::ToggleNodeExpanded(node_id));
                            }
                        })
                    }
                >
                    {&node.name}
                </span>

                // Child count badge
                if child_count > 0 && !is_element {
                    <span class="tree-count">{child_count}</span>
                }

                // Visibility toggle for elements
                if is_element && node.has_geometry {
                    <button
                        class={classes!("visibility-btn", is_hidden.then_some("hidden"))}
                        onclick={
                            let state = state.clone();
                            let node_id = node.id;
                            Callback::from(move |e: MouseEvent| {
                                e.stop_propagation();
                                state.dispatch(ViewerAction::ToggleVisibility(node_id));
                            })
                        }
                        title={if is_hidden { "Show" } else { "Hide" }}
                    >
                        {if is_hidden { "👁‍🗨" } else { "👁" }}
                    </button>
                }
            </div>

            // Children
            if is_expanded && !filtered_children.is_empty() {
                <div class="tree-children">
                    { for filtered_children.iter().map(|child| html! {
                        <TreeNode node={child.clone()} depth={depth + 1} />
                    })}
                </div>
            }
        </div>
    }
}

/// Hierarchy panel component
#[function_component]
pub fn HierarchyPanel() -> Html {
    let state = use_context::<ViewerStateContext>().expect("ViewerStateContext not found");

    html! {
        <div class="hierarchy-panel">
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

            // Expand/collapse all buttons
            if state.spatial_tree.is_some() {
                <div class="tree-controls">
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

            // Entity tree
            <div class="entity-list">
                if state.spatial_tree.is_none() && state.entities.is_empty() {
                    <div class="empty-state">
                        <span class="empty-icon">{"📂"}</span>
                        <span class="empty-text">{"No model loaded"}</span>
                        <span class="empty-hint">{"Open an IFC file to view entities"}</span>
                    </div>
                } else if let Some(ref tree) = state.spatial_tree {
                    <TreeNode node={tree.clone()} depth={0} />
                } else {
                    // Fallback to flat list if no tree
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
                                        {entity.name.as_deref().unwrap_or(&entity.entity_type)}
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
