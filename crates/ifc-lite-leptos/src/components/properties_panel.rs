//! Properties panel - shows selected entity details

use crate::bridge;
use crate::state::use_viewer_state;
use leptos::prelude::*;
use rustc_hash::FxHashSet;

/// Properties panel component
#[component]
pub fn PropertiesPanel() -> impl IntoView {
    let state = use_viewer_state();

    // Get selected entity (first one if multiple selected)
    let selected_entity = Memo::new(move |_| {
        let selected_ids = state.selection.selected_ids.get();
        let entities = state.scene.entities.get();
        selected_ids
            .iter()
            .next()
            .and_then(|id| entities.iter().find(|e| e.id == *id))
            .cloned()
    });

    let selection_count = Memo::new(move |_| state.selection.selected_ids.get().len());

    view! {
        <div class="properties-panel">
            {move || {
                let entity = selected_entity.get();
                let count = selection_count.get();

                if let Some(entity) = entity {
                    // Single entity selected
                    view! {
                        <div>
                            // Entity info section
                            <div class="property-section">
                                <div class="section-header">"Entity Info"</div>

                                <div class="property-row">
                                    <span class="property-label">"Type"</span>
                                    <span class="property-value">{entity.entity_type.clone()}</span>
                                </div>

                                {entity.name.clone().map(|name| view! {
                                    <div class="property-row">
                                        <span class="property-label">"Name"</span>
                                        <span class="property-value">{name}</span>
                                    </div>
                                })}

                                {entity.description.clone().map(|desc| view! {
                                    <div class="property-row">
                                        <span class="property-label">"Description"</span>
                                        <span class="property-value">{desc}</span>
                                    </div>
                                })}

                                {entity.global_id.clone().map(|gid| {
                                    let gid_clone = gid.clone();
                                    view! {
                                        <div class="property-row">
                                            <span class="property-label">"GlobalId"</span>
                                            <span class="property-value global-id">
                                                {gid}
                                                <button
                                                    class="copy-btn"
                                                    on:click=move |_| copy_to_clipboard(&gid_clone)
                                                    title="Copy to clipboard"
                                                >
                                                    "📋"
                                                </button>
                                            </span>
                                        </div>
                                    }
                                })}

                                {entity.storey.clone().map(|storey| view! {
                                    <div class="property-row">
                                        <span class="property-label">"Storey"</span>
                                        <span class="property-value">{storey}</span>
                                    </div>
                                })}

                                {entity.storey_elevation.map(|elev| view! {
                                    <div class="property-row">
                                        <span class="property-label">"Elevation"</span>
                                        <span class="property-value">{format!("{:.2} m", elev)}</span>
                                    </div>
                                })}
                            </div>

                            // Actions section
                            <div class="property-section">
                                <div class="section-header">"Actions"</div>
                                <div class="action-buttons">
                                    <ActionButtons entity_id=entity.id entity_type=entity.entity_type.clone() />
                                </div>
                            </div>

                            // Property sets
                            {if entity.property_sets.is_empty() {
                                view! {
                                    <div class="property-section">
                                        <div class="section-header">"Property Sets"</div>
                                        <div class="empty-state small">
                                            <span class="empty-text">"No property sets"</span>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div>
                                        {entity.property_sets.into_iter().map(|pset| view! {
                                            <div class="property-section">
                                                <div class="section-header">{pset.name}</div>
                                                {pset.properties.into_iter().map(|prop| view! {
                                                    <div class="property-row">
                                                        <span class="property-label">{prop.name}</span>
                                                        <span class="property-value">
                                                            {prop.value}
                                                            {prop.unit.map(|u| view! {
                                                                <span class="property-unit">{format!(" {}", u)}</span>
                                                            })}
                                                        </span>
                                                    </div>
                                                }).collect_view()}
                                            </div>
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }}

                            // Quantities
                            {if entity.quantities.is_empty() {
                                view! {
                                    <div class="property-section">
                                        <div class="section-header">"Quantities"</div>
                                        <div class="empty-state small">
                                            <span class="empty-text">"No quantities"</span>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="property-section">
                                        <div class="section-header">"Quantities"</div>
                                        {entity.quantities.into_iter().map(|qty| view! {
                                            <div class="property-row">
                                                <span class="property-label">{qty.name}</span>
                                                <span class="property-value">
                                                    {format!("{:.3}", qty.value)}
                                                    {if !qty.unit.is_empty() {
                                                        Some(view! {
                                                            <span class="property-unit">{format!(" {}", qty.unit)}</span>
                                                        })
                                                    } else {
                                                        None
                                                    }}
                                                </span>
                                            </div>
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }}
                        </div>
                    }.into_any()
                } else if count > 1 {
                    // Multiple selection
                    view! {
                        <div class="multi-selection">
                            <span class="selection-icon">"📑"</span>
                            <span class="selection-count">{format!("{} entities selected", count)}</span>
                            <MultiSelectionActions />
                        </div>
                    }.into_any()
                } else {
                    // No selection
                    view! {
                        <div class="empty-state">
                            <span class="empty-icon">"👆"</span>
                            <span class="empty-text">"No entity selected"</span>
                            <span class="empty-hint">"Click an entity to view its properties"</span>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Action buttons for single entity
#[component]
fn ActionButtons(entity_id: u64, entity_type: String) -> impl IntoView {
    let state = use_viewer_state();

    view! {
        <button
            class="action-btn"
            on:click=move |_| {
                bridge::save_focus(&bridge::FocusData { entity_id });
                bridge::log(&format!("Zoom to entity #{}", entity_id));
            }
            title="Zoom to entity"
        >
            "🔍 Zoom to"
        </button>

        <button
            class="action-btn"
            on:click=move |_| {
                state.visibility.isolate(entity_id);
            }
            title="Isolate entity"
        >
            "🎯 Isolate"
        </button>

        <button
            class="action-btn"
            on:click=move |_| {
                state.visibility.hide(entity_id);
            }
            title="Hide entity"
        >
            "👁‍🗨 Hide"
        </button>

        <button
            class="action-btn"
            on:click={
                let entity_type = entity_type.clone();
                move |_| {
                    // Select all entities of the same type
                    let same_type_ids: FxHashSet<u64> = state.scene.entities.get()
                        .iter()
                        .filter(|e| e.entity_type == entity_type)
                        .map(|e| e.id)
                        .collect();
                    for id in same_type_ids {
                        state.selection.add_to_selection(id);
                    }
                }
            }
            title="Select all of this type"
        >
            "📑 Select Similar"
        </button>
    }
}

/// Action buttons for multiple selection
#[component]
fn MultiSelectionActions() -> impl IntoView {
    let state = use_viewer_state();

    view! {
        <div class="action-buttons">
            <button
                class="action-btn"
                on:click=move |_| {
                    let ids = state.selection.selected_ids.get();
                    state.visibility.isolate_many(ids);
                }
            >
                "🎯 Isolate All"
            </button>

            <button
                class="action-btn"
                on:click=move |_| {
                    for id in state.selection.selected_ids.get().iter() {
                        state.visibility.hide(*id);
                    }
                }
            >
                "👁‍🗨 Hide All"
            </button>

            <button
                class="action-btn"
                on:click=move |_| {
                    state.selection.clear();
                }
            >
                "✖ Clear Selection"
            </button>
        </div>
    }
}

/// Copy text to clipboard using JS
fn copy_to_clipboard(text: &str) {
    let js_code = format!(
        "navigator.clipboard.writeText('{}').catch(e => console.warn('Copy failed:', e))",
        text.replace('\'', "\\'")
    );
    let _ = js_sys::eval(&js_code);
}
