//! Main viewer layout component
//!
//! Three-panel layout: hierarchy (left), viewport (center), properties (right)

use super::{parse_and_process_ifc, HierarchyPanel, PropertiesPanel, StatusBar, Toolbar, Viewport};
use crate::bridge::{self, VisibilityData};
use crate::state::{use_viewer_state, Progress, ViewerAction, ViewerStateContext};
use crate::utils::{build_ifc_url, fetch_ifc_file, get_file_param};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// Component that loads IFC file from URL parameter on mount
#[function_component]
fn UrlLoader() -> Html {
    let state = use_context::<ViewerStateContext>().expect("ViewerStateContext not found");

    // Run once on mount
    use_effect_with((), {
        let state = state.clone();
        move |_| {
            // Check for ?file= parameter
            if let Some(file_param) = get_file_param() {
                let url = build_ifc_url(&file_param);
                bridge::log(&format!("[Yew] Loading IFC from URL: {}", url));

                // Extract filename for display
                let file_name = file_param
                    .rsplit('/')
                    .next()
                    .unwrap_or(&file_param)
                    .to_string();

                state.dispatch(ViewerAction::SetFileName(file_name));
                state.dispatch(ViewerAction::SetLoading(true));
                state.dispatch(ViewerAction::SetProgress(Progress {
                    phase: "Fetching file".to_string(),
                    percent: 0.0,
                }));

                // Fetch and parse
                spawn_local(async move {
                    match fetch_ifc_file(&url).await {
                        Ok(content) => {
                            bridge::log(&format!("[Yew] Fetched {} bytes", content.len()));
                            state.dispatch(ViewerAction::SetProgress(Progress {
                                phase: "Parsing IFC".to_string(),
                                percent: 10.0,
                            }));

                            match parse_and_process_ifc(&content, &state) {
                                Ok(_) => {
                                    bridge::log("[Yew] IFC file processed successfully");
                                    state.dispatch(ViewerAction::SetLoading(false));
                                    state.dispatch(ViewerAction::ClearProgress);
                                }
                                Err(e) => {
                                    bridge::log_error(&format!(
                                        "[Yew] Failed to process IFC: {}",
                                        e
                                    ));
                                    state.dispatch(ViewerAction::SetError(e));
                                }
                            }
                        }
                        Err(e) => {
                            bridge::log_error(&format!("[Yew] Failed to fetch IFC: {}", e));
                            state.dispatch(ViewerAction::SetError(format!(
                                "Failed to load file: {}",
                                e
                            )));
                        }
                    }
                });
            }

            || ()
        }
    });

    html! {}
}

/// Component that syncs Yew state to Bevy via localStorage bridge
#[function_component]
fn StateBridge() -> Html {
    let state = use_context::<ViewerStateContext>().expect("ViewerStateContext not found");

    // Track last known selection to avoid infinite loops
    let last_bevy_selection = use_state(std::collections::HashSet::<u64>::new);

    // Sync visibility state to Bevy when hidden_ids or isolated_ids change
    {
        let hidden_ids = state.hidden_ids.clone();
        let isolated_ids = state.isolated_ids.clone();

        use_effect_with(
            (hidden_ids.len(), isolated_ids.as_ref().map(|s| s.len())),
            move |_| {
                let visibility = VisibilityData {
                    hidden: hidden_ids.iter().copied().collect(),
                    isolated: isolated_ids.map(|ids| ids.iter().copied().collect()),
                };
                bridge::save_visibility(&visibility);
                bridge::log(&format!(
                    "[Yew] Synced visibility: {} hidden, {} isolated",
                    visibility.hidden.len(),
                    visibility.isolated.as_ref().map(|v| v.len()).unwrap_or(0)
                ));
                || ()
            },
        );
    }

    // Poll selection from Bevy (Bevy -> Yew)
    // Only applies when selection source is "bevy" to avoid race conditions
    {
        let state = state.clone();
        let last_bevy_selection = last_bevy_selection.clone();

        use_effect_with((), move |_| {
            let interval = gloo::timers::callback::Interval::new(100, move || {
                // Only apply selection if it came from Bevy, not from Yew
                let source = bridge::get_selection_source();
                if source.as_deref() != Some("bevy") {
                    return;
                }

                if let Some(bevy_selection) = bridge::load_selection() {
                    let bevy_ids: std::collections::HashSet<u64> =
                        bevy_selection.selected_ids.into_iter().collect();

                    // Only update if Bevy's selection differs from what we last saw
                    if bevy_ids != *last_bevy_selection {
                        // Check if this is different from current Yew state
                        if bevy_ids != state.selected_ids {
                            bridge::log(&format!(
                                "[Yew] Bevy selection changed: {:?}",
                                bevy_ids.iter().take(3).collect::<Vec<_>>()
                            ));

                            // Update Yew state to match Bevy
                            if bevy_ids.is_empty() {
                                state.dispatch(crate::state::ViewerAction::ClearSelection);
                            } else if bevy_ids.len() == 1 {
                                let id = *bevy_ids.iter().next().unwrap();
                                state.dispatch(crate::state::ViewerAction::Select(id));
                            } else {
                                // Multi-select: clear and add each
                                state.dispatch(crate::state::ViewerAction::ClearSelection);
                                for id in &bevy_ids {
                                    state.dispatch(crate::state::ViewerAction::AddToSelection(*id));
                                }
                            }
                        }
                        last_bevy_selection.set(bevy_ids);
                    }
                }
            });

            move || drop(interval)
        });
    }

    // Sync selection state to Bevy (Yew -> Bevy) - only when Yew initiates the change
    {
        let selected_ids = state.selected_ids.clone();
        let hovered_id = state.hovered_id;

        use_effect_with((selected_ids.len(), hovered_id), move |_| {
            let selection = bridge::SelectionData {
                selected_ids: selected_ids.iter().copied().collect(),
                hovered_id,
            };
            bridge::save_selection(&selection);
            || ()
        });
    }

    html! {}
}

/// Main viewer layout properties
#[derive(Properties, PartialEq)]
pub struct ViewerLayoutProps {
    #[prop_or_default]
    pub class: Classes,
}

/// Main viewer layout component
#[function_component]
pub fn ViewerLayout(props: &ViewerLayoutProps) -> Html {
    let state = use_viewer_state();

    // Theme class
    let theme_class = match state.theme {
        crate::state::Theme::Dark => "theme-dark",
        crate::state::Theme::Light => "theme-light",
    };

    html! {
        <ContextProvider<ViewerStateContext> context={state.clone()}>
            // URL loader handles ?file= parameter on mount
            <UrlLoader />
            // State bridge syncs Yew state to Bevy via localStorage
            <StateBridge />
            <div class={classes!("viewer-layout", theme_class, props.class.clone())}>
                // Left panel (hierarchy)
                if !state.left_panel_collapsed {
                    <div class="panel panel-left">
                        <div class="panel-header">
                            <span class="panel-title">{"Model"}</span>
                            <button
                                class="panel-collapse-btn"
                                onclick={
                                    let state = state.clone();
                                    Callback::from(move |_| {
                                        state.dispatch(crate::state::ViewerAction::SetLeftPanelCollapsed(true));
                                    })
                                }
                                title="Collapse panel"
                            >
                                {"◀"}
                            </button>
                        </div>
                        <HierarchyPanel />
                    </div>
                } else {
                    <button
                        class="panel-expand-btn panel-expand-left"
                        onclick={
                            let state = state.clone();
                            Callback::from(move |_| {
                                state.dispatch(crate::state::ViewerAction::SetLeftPanelCollapsed(false));
                            })
                        }
                        title="Expand hierarchy panel"
                    >
                        {"▶"}
                    </button>
                }

                // Center (viewport)
                <div class="viewport-container">
                    <Toolbar />
                    <Viewport />
                    <StatusBar />
                </div>

                // Right panel (properties)
                if !state.right_panel_collapsed {
                    <div class="panel panel-right">
                        <div class="panel-header">
                            <span class="panel-title">{"Properties"}</span>
                            <button
                                class="panel-collapse-btn"
                                onclick={
                                    let state = state.clone();
                                    Callback::from(move |_| {
                                        state.dispatch(crate::state::ViewerAction::SetRightPanelCollapsed(true));
                                    })
                                }
                                title="Collapse panel"
                            >
                                {"▶"}
                            </button>
                        </div>
                        <PropertiesPanel />
                    </div>
                } else {
                    <button
                        class="panel-expand-btn panel-expand-right"
                        onclick={
                            let state = state.clone();
                            Callback::from(move |_| {
                                state.dispatch(crate::state::ViewerAction::SetRightPanelCollapsed(false));
                            })
                        }
                        title="Expand properties panel"
                    >
                        {"◀"}
                    </button>
                }
            </div>
        </ContextProvider<ViewerStateContext>>
    }
}
