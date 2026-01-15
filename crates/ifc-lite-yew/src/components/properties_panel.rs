//! Properties panel - shows selected entity details

use std::collections::HashSet;
use yew::prelude::*;
use crate::state::{ViewerAction, ViewerStateContext};

/// Properties panel component
#[function_component]
pub fn PropertiesPanel() -> Html {
    let state = use_context::<ViewerStateContext>().expect("ViewerStateContext not found");

    // Get selected entity
    let selected_entity = state.selected_ids.iter().next()
        .and_then(|id| state.entities.iter().find(|e| e.id == *id));

    html! {
        <div class="properties-panel">
            if let Some(entity) = selected_entity {
                // Entity info section
                <div class="property-section">
                    <div class="section-header">{"Entity Info"}</div>

                    <div class="property-row">
                        <span class="property-label">{"Type"}</span>
                        <span class="property-value">{&entity.entity_type}</span>
                    </div>

                    if let Some(ref name) = entity.name {
                        <div class="property-row">
                            <span class="property-label">{"Name"}</span>
                            <span class="property-value">{name}</span>
                        </div>
                    }

                    if let Some(ref global_id) = entity.global_id {
                        <div class="property-row">
                            <span class="property-label">{"GlobalId"}</span>
                            <span class="property-value global-id">
                                {global_id}
                                <button
                                    class="copy-btn"
                                    onclick={
                                        let gid = global_id.clone();
                                        Callback::from(move |_| {
                                            copy_to_clipboard(&gid);
                                        })
                                    }
                                    title="Copy to clipboard"
                                >
                                    {"📋"}
                                </button>
                            </span>
                        </div>
                    }

                    if let Some(ref storey) = entity.storey {
                        <div class="property-row">
                            <span class="property-label">{"Storey"}</span>
                            <span class="property-value">{storey}</span>
                        </div>
                    }

                    if let Some(elevation) = entity.storey_elevation {
                        <div class="property-row">
                            <span class="property-label">{"Elevation"}</span>
                            <span class="property-value">{format!("{:.2} m", elevation)}</span>
                        </div>
                    }
                </div>

                // Actions section
                <div class="property-section">
                    <div class="section-header">{"Actions"}</div>

                    <div class="action-buttons">
                        <button
                            class="action-btn"
                            onclick={
                                let entity_id = entity.id;
                                Callback::from(move |_| {
                                    crate::bridge::save_focus(&crate::bridge::FocusData { entity_id });
                                    crate::bridge::log(&format!("Zoom to entity #{}", entity_id));
                                })
                            }
                            title="Zoom to entity"
                        >
                            {"🔍 Zoom to"}
                        </button>

                        <button
                            class="action-btn"
                            onclick={
                                let state = state.clone();
                                let entity_id = entity.id;
                                Callback::from(move |_| {
                                    state.dispatch(ViewerAction::IsolateEntity(entity_id));
                                })
                            }
                            title="Isolate entity"
                        >
                            {"🎯 Isolate"}
                        </button>

                        <button
                            class="action-btn"
                            onclick={
                                let state = state.clone();
                                let entity_id = entity.id;
                                Callback::from(move |_| {
                                    state.dispatch(ViewerAction::HideEntity(entity_id));
                                })
                            }
                            title="Hide entity"
                        >
                            {"👁‍🗨 Hide"}
                        </button>

                        <button
                            class="action-btn"
                            onclick={
                                let state = state.clone();
                                let entity_type = entity.entity_type.clone();
                                Callback::from(move |_| {
                                    // Select all entities of the same type
                                    let same_type_ids: HashSet<u64> = state.entities.iter()
                                        .filter(|e| e.entity_type == entity_type)
                                        .map(|e| e.id)
                                        .collect();
                                    for id in same_type_ids {
                                        state.dispatch(ViewerAction::AddToSelection(id));
                                    }
                                })
                            }
                            title="Select all of this type"
                        >
                            {"📑 Select Similar"}
                        </button>
                    </div>
                </div>

                // Property Sets
                if !entity.property_sets.is_empty() {
                    { for entity.property_sets.iter().map(|pset| html! {
                        <div class="property-section">
                            <div class="section-header">{&pset.name}</div>
                            { for pset.properties.iter().map(|prop| html! {
                                <div class="property-row">
                                    <span class="property-label">{&prop.name}</span>
                                    <span class="property-value">
                                        {&prop.value}
                                        if let Some(ref unit) = prop.unit {
                                            <span class="property-unit">{format!(" {}", unit)}</span>
                                        }
                                    </span>
                                </div>
                            })}
                        </div>
                    })}
                } else {
                    <div class="property-section">
                        <div class="section-header">{"Property Sets"}</div>
                        <div class="empty-state small">
                            <span class="empty-text">{"No property sets"}</span>
                        </div>
                    </div>
                }

                // Quantities
                if !entity.quantities.is_empty() {
                    <div class="property-section">
                        <div class="section-header">{"Quantities"}</div>
                        { for entity.quantities.iter().map(|qty| html! {
                            <div class="property-row">
                                <span class="property-label">{&qty.name}</span>
                                <span class="property-value">
                                    {format!("{:.3}", qty.value)}
                                    if !qty.unit.is_empty() {
                                        <span class="property-unit">{format!(" {}", qty.unit)}</span>
                                    }
                                </span>
                            </div>
                        })}
                    </div>
                } else {
                    <div class="property-section">
                        <div class="section-header">{"Quantities"}</div>
                        <div class="empty-state small">
                            <span class="empty-text">{"No quantities"}</span>
                        </div>
                    </div>
                }
            } else if state.selected_ids.len() > 1 {
                // Multiple selection
                <div class="multi-selection">
                    <span class="selection-icon">{"📑"}</span>
                    <span class="selection-count">
                        {format!("{} entities selected", state.selected_ids.len())}
                    </span>

                    <div class="action-buttons">
                        <button
                            class="action-btn"
                            onclick={
                                let state = state.clone();
                                Callback::from(move |_| {
                                    let ids = state.selected_ids.clone();
                                    state.dispatch(ViewerAction::IsolateEntities(ids));
                                })
                            }
                        >
                            {"🎯 Isolate All"}
                        </button>

                        <button
                            class="action-btn"
                            onclick={
                                let state = state.clone();
                                Callback::from(move |_| {
                                    for id in state.selected_ids.iter() {
                                        state.dispatch(ViewerAction::HideEntity(*id));
                                    }
                                })
                            }
                        >
                            {"👁‍🗨 Hide All"}
                        </button>

                        <button
                            class="action-btn"
                            onclick={
                                let state = state.clone();
                                Callback::from(move |_| {
                                    state.dispatch(ViewerAction::ClearSelection);
                                })
                            }
                        >
                            {"✖ Clear Selection"}
                        </button>
                    </div>
                </div>
            } else {
                // No selection
                <div class="empty-state">
                    <span class="empty-icon">{"👆"}</span>
                    <span class="empty-text">{"No entity selected"}</span>
                    <span class="empty-hint">{"Click an entity to view its properties"}</span>
                </div>
            }
        </div>
    }
}

/// Copy text to clipboard using JS eval
fn copy_to_clipboard(text: &str) {
    // Simple approach using JS eval
    let js_code = format!(
        "navigator.clipboard.writeText('{}').catch(e => console.warn('Copy failed:', e))",
        text.replace('\'', "\\'")
    );
    let _ = js_sys::eval(&js_code);
}
