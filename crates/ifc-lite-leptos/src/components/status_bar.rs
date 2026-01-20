//! Status bar component - shows loading/error status and entity counts

use crate::state::use_viewer_state;
use leptos::prelude::*;

/// Status bar component
#[component]
pub fn StatusBar() -> impl IntoView {
    let state = use_viewer_state();

    // Computed: visible entity count
    let visible_count = Memo::new(move |_| {
        let entities = state.scene.entities.get();
        let hidden = state.visibility.hidden_ids.get();
        let isolated = state.visibility.isolated_ids.get();
        let storey_filter = state.visibility.storey_filter.get();

        entities
            .iter()
            .filter(|e| {
                // Check if hidden
                if hidden.contains(&e.id) {
                    return false;
                }
                // Check if isolated (only show isolated entities)
                if let Some(ref iso) = isolated {
                    if !iso.contains(&e.id) {
                        return false;
                    }
                }
                // Check storey filter
                if let Some(ref filter) = storey_filter {
                    if e.storey.as_ref() != Some(filter) {
                        return false;
                    }
                }
                true
            })
            .count()
    });

    let total_count = Memo::new(move |_| state.scene.entities.get().len());

    view! {
        <div class="status-bar">
            // Left section: status
            <div class="status-left">
                {move || {
                    let error = state.loading.error.get();
                    let loading = state.loading.loading.get();
                    let progress = state.loading.progress.get();

                    if let Some(err) = error {
                        view! {
                            <span class="status-error">{err}</span>
                        }.into_any()
                    } else if loading {
                        if let Some(p) = progress {
                            view! {
                                <span class="status-loading">
                                    {format!("{} {}%", p.phase, p.percent as i32)}
                                </span>
                            }.into_any()
                        } else {
                            view! {
                                <span class="status-loading">"Loading..."</span>
                            }.into_any()
                        }
                    } else if total_count.get() > 0 {
                        view! {
                            <span class="status-ready">"Ready"</span>
                        }.into_any()
                    } else {
                        view! {
                            <span class="status-idle">"No model loaded"</span>
                        }.into_any()
                    }
                }}
            </div>

            // Center section: entity counts
            <div class="status-center">
                {move || {
                    let total = total_count.get();
                    let visible = visible_count.get();
                    if total > 0 {
                        if visible != total {
                            view! {
                                <span class="status-count">
                                    {format!("{} / {} entities", visible, total)}
                                </span>
                            }.into_any()
                        } else {
                            view! {
                                <span class="status-count">
                                    {format!("{} entities", total)}
                                </span>
                            }.into_any()
                        }
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}
            </div>

            // Right section: filters and selection
            <div class="status-right">
                // Storey filter indicator
                {move || {
                    state.visibility.storey_filter.get().map(|storey| {
                        view! {
                            <span class="status-filter">
                                {format!("Storey: {}", storey)}
                            </span>
                        }
                    })
                }}

                // Selection count
                {move || {
                    let count = state.selection.selected_ids.get().len();
                    if count > 0 {
                        Some(view! {
                            <span class="status-selection">
                                {format!("{} selected", count)}
                            </span>
                        })
                    } else {
                        None
                    }
                }}

                // File name
                {move || {
                    state.scene.file_name.get().map(|name| {
                        view! {
                            <span class="status-file">{name}</span>
                        }
                    })
                }}
            </div>
        </div>
    }
}
