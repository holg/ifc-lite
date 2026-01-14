//! Status bar component

use yew::prelude::*;
use crate::state::ViewerStateContext;

/// Status bar component
#[function_component]
pub fn StatusBar() -> Html {
    let state = use_context::<ViewerStateContext>().expect("ViewerStateContext not found");

    // Calculate visible entity count
    let visible_count = state.entities.iter()
        .filter(|e| {
            // Not hidden
            if state.hidden_ids.contains(&e.id) {
                return false;
            }
            // Not isolated out
            if let Some(ref isolated) = state.isolated_ids {
                if !isolated.contains(&e.id) {
                    return false;
                }
            }
            // Not filtered by storey
            if let Some(ref storey) = state.storey_filter {
                if e.storey.as_ref() != Some(storey) {
                    return false;
                }
            }
            true
        })
        .count();

    let total_count = state.entities.len();

    html! {
        <div class="status-bar">
            // Left: Status/errors
            <div class="status-left">
                if let Some(ref error) = state.error {
                    <span class="status-error" title={error.clone()}>
                        {"⚠️ "}{error}
                    </span>
                } else if state.loading {
                    if let Some(ref progress) = state.progress {
                        <span class="status-loading">
                            {&progress.phase}
                        </span>
                    } else {
                        <span class="status-loading">{"Loading..."}</span>
                    }
                } else if state.file_name.is_some() {
                    <span class="status-ready">{"Ready"}</span>
                } else {
                    <span class="status-idle">{"No file loaded"}</span>
                }
            </div>

            // Center: Counts
            <div class="status-center">
                if total_count > 0 {
                    <span class="status-count" title="Visible entities">
                        {format!("{} / {} entities", visible_count, total_count)}
                    </span>
                }

                // Storey filter indicator
                if let Some(ref storey) = state.storey_filter {
                    <span class="status-filter" title="Storey filter active">
                        {"🏢 "}{storey}
                    </span>
                }

                // Selection count
                if !state.selected_ids.is_empty() {
                    <span class="status-selection">
                        {format!("{} selected", state.selected_ids.len())}
                    </span>
                }
            </div>

            // Right: File info
            <div class="status-right">
                if let Some(ref file_name) = state.file_name {
                    <span class="status-filename" title={file_name.clone()}>
                        {file_name}
                    </span>
                }
            </div>
        </div>
    }
}
