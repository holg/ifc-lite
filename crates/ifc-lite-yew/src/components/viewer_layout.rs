//! Main viewer layout component
//!
//! Three-panel layout: hierarchy (left), viewport (center), properties (right)

use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::state::{ViewerAction, ViewerStateContext, Progress, use_viewer_state};
use crate::bridge::{self, VisibilityData};
use crate::utils::{get_file_param, build_ifc_url, fetch_ifc_file};
use super::{HierarchyPanel, PropertiesPanel, StatusBar, Toolbar, Viewport, parse_and_process_ifc};

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
                                    bridge::log_error(&format!("[Yew] Failed to process IFC: {}", e));
                                    state.dispatch(ViewerAction::SetError(e));
                                }
                            }
                        }
                        Err(e) => {
                            bridge::log_error(&format!("[Yew] Failed to fetch IFC: {}", e));
                            state.dispatch(ViewerAction::SetError(format!("Failed to load file: {}", e)));
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

    // Sync selection state to Bevy
    {
        let selected_ids = state.selected_ids.clone();
        let hovered_id = state.hovered_id;

        use_effect_with(
            (selected_ids.len(), hovered_id),
            move |_| {
                let selection = bridge::SelectionData {
                    selected_ids: selected_ids.iter().copied().collect(),
                    hovered_id,
                };
                bridge::save_selection(&selection);
                || ()
            },
        );
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
