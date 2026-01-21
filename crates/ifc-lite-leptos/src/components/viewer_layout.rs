//! Main viewer layout - three-panel layout with state management

use crate::bridge::{self, SelectionData, VisibilityData};
use crate::components::{HierarchyPanel, PropertiesPanel, StatusBar, Toolbar, Viewport};
use crate::state::{provide_viewer_state, use_viewer_state, Progress, Theme};
use crate::utils::{build_ifc_url, fetch_ifc_file, get_file_param};
use gloo_timers::callback::Interval;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Main App component - entry point for the viewer
#[component]
pub fn App() -> impl IntoView {
    // Provide state context to entire app
    provide_viewer_state();

    view! {
        <ViewerLayout />
    }
}

/// Main viewer layout component
#[component]
pub fn ViewerLayout() -> impl IntoView {
    let state = use_viewer_state();

    // Theme class for styling
    let theme_class = Memo::new(move |_| {
        match state.ui.theme.get() {
            Theme::Dark => "viewer-layout theme-dark",
            Theme::Light => "viewer-layout theme-light",
        }
    });

    view! {
        <div class=move || theme_class.get()>
            // Left panel - Hierarchy
            {move || {
                let collapsed = state.ui.left_panel_collapsed.get();
                if collapsed {
                    view! {
                        <button
                            class="panel-expand-btn panel-expand-left"
                            on:click=move |_| state.ui.toggle_left_panel()
                            title="Expand hierarchy panel"
                        >
                            "▶"
                        </button>
                    }.into_any()
                } else {
                    view! {
                        <div class="panel panel-left">
                            <div class="panel-header">
                                <span class="panel-title">"Model"</span>
                                <button
                                    class="panel-collapse-btn"
                                    on:click=move |_| state.ui.toggle_left_panel()
                                    title="Collapse panel"
                                >
                                    "◀"
                                </button>
                            </div>
                            <HierarchyPanel />
                        </div>
                    }.into_any()
                }
            }}

            // Center - Viewport with toolbar and status bar
            <div class="viewport-container">
                <Toolbar />
                <Viewport />
                <StatusBar />
            </div>

            // Right panel - Properties
            {move || {
                let collapsed = state.ui.right_panel_collapsed.get();
                if collapsed {
                    view! {
                        <button
                            class="panel-expand-btn panel-expand-right"
                            on:click=move |_| state.ui.toggle_right_panel()
                            title="Expand properties panel"
                        >
                            "◀"
                        </button>
                    }.into_any()
                } else {
                    view! {
                        <div class="panel panel-right">
                            <div class="panel-header">
                                <span class="panel-title">"Properties"</span>
                                <button
                                    class="panel-collapse-btn"
                                    on:click=move |_| state.ui.toggle_right_panel()
                                    title="Collapse panel"
                                >
                                    "▶"
                                </button>
                            </div>
                            <PropertiesPanel />
                        </div>
                    }.into_any()
                }
            }}

            // URL parameter loader
            <UrlLoader />

            // State bridge for Bevy sync
            <StateBridge />

            // Shortcuts dialog
            {move || {
                if state.ui.show_shortcuts_dialog.get() {
                    Some(view! { <ShortcutsDialog /> })
                } else {
                    None
                }
            }}
        </div>
    }
}

/// URL parameter loader - loads IFC file from ?file= parameter
#[component]
fn UrlLoader() -> impl IntoView {
    let state = use_viewer_state();

    // Load file from URL on mount
    Effect::new(move |_| {
        if let Some(file_param) = get_file_param() {
            let url = build_ifc_url(&file_param);
            bridge::log_info(&format!("[Leptos] Loading file from URL: {}", url));

            // Extract filename from URL
            let file_name = url.rsplit('/').next().unwrap_or(&file_param).to_string();
            state.scene.set_file_name(file_name);
            state.loading.set_loading(true);
            state.loading.set_progress(Progress {
                phase: "Fetching file".to_string(),
                percent: 0.0,
            });

            spawn_local(async move {
                match fetch_ifc_file(&url).await {
                    Ok(content) => {
                        state.loading.set_progress(Progress {
                            phase: "Parsing IFC".to_string(),
                            percent: 10.0,
                        });

                        match crate::components::toolbar::parse_and_process_ifc(&content, state) {
                            Ok(_) => {
                                bridge::log_info("IFC file loaded from URL");
                                state.loading.set_loading(false);
                                state.loading.clear_progress();
                                bridge::save_camera_cmd(&bridge::CameraCommand {
                                    cmd: "fit_all".to_string(),
                                    mode: None,
                                });
                            }
                            Err(e) => {
                                bridge::log_error(&format!("Failed to process IFC: {}", e));
                                state.loading.set_error(format!("Parse error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        bridge::log_error(&format!("Failed to fetch file: {}", e));
                        state.loading.set_error(format!("Fetch error: {}", e));
                    }
                }
            });
        }
    });

    view! {}
}

/// State bridge - syncs state between Leptos and Bevy
#[component]
fn StateBridge() -> impl IntoView {
    let state = use_viewer_state();
    let interval_started = RwSignal::new(false);

    // Sync visibility to Bevy when it changes
    Effect::new(move |_| {
        let hidden = state.visibility.hidden_ids.get();
        let isolated = state.visibility.isolated_ids.get();

        let visibility = VisibilityData {
            hidden: hidden.iter().copied().collect(),
            isolated: isolated.map(|ids| ids.iter().copied().collect()),
        };
        bridge::save_visibility(&visibility);
    });

    // Sync selection to Bevy when it changes
    Effect::new(move |_| {
        let selected = state.selection.selected_ids.get();
        let hovered = state.selection.hovered_id.get();

        let selection = SelectionData {
            selected_ids: selected.iter().copied().collect(),
            hovered_id: hovered,
        };
        bridge::save_selection(&selection);
    });

    // Sync palette to Bevy when it changes
    Effect::new(move |_| {
        let palette = state.ui.color_palette.get();
        bridge::save_palette(palette);
    });

    // Poll selection from Bevy - only start interval once
    Effect::new(move |_| {
        if interval_started.get_untracked() {
            return;
        }
        interval_started.set(true);

        let poll_interval = Interval::new(100, move || {
            // Only process if source is "bevy"
            if bridge::get_selection_source().as_deref() != Some("bevy") {
                return;
            }

            if let Some(bevy_selection) = bridge::load_selection() {
                // Update selection from Bevy
                let current = state.selection.selected_ids.get_untracked();
                let new_ids: rustc_hash::FxHashSet<u64> =
                    bevy_selection.selected_ids.into_iter().collect();

                if current != new_ids {
                    state.selection.selected_ids.set(new_ids);
                }

                if bevy_selection.hovered_id != state.selection.hovered_id.get_untracked() {
                    state.selection.hovered_id.set(bevy_selection.hovered_id);
                }
            }
        });

        // Leak interval to keep it alive - it runs for the lifetime of the app
        std::mem::forget(poll_interval);
    });

    view! {}
}

/// Keyboard shortcuts dialog
#[component]
fn ShortcutsDialog() -> impl IntoView {
    let state = use_viewer_state();

    view! {
        <div class="dialog-overlay" on:click=move |_| state.ui.toggle_shortcuts_dialog()>
            <div class="dialog shortcuts-dialog" on:click=|ev| ev.stop_propagation()>
                <div class="dialog-header">
                    <span class="dialog-title">"Keyboard Shortcuts"</span>
                    <button
                        class="dialog-close"
                        on:click=move |_| state.ui.toggle_shortcuts_dialog()
                    >
                        "×"
                    </button>
                </div>
                <div class="dialog-content">
                    <div class="shortcut-group">
                        <div class="shortcut-group-title">"Tools"</div>
                        <ShortcutRow key="V" action="Select" />
                        <ShortcutRow key="P" action="Pan" />
                        <ShortcutRow key="O" action="Orbit" />
                        <ShortcutRow key="C" action="Walk" />
                        <ShortcutRow key="M" action="Measure" />
                        <ShortcutRow key="X" action="Section" />
                        <ShortcutRow key="B" action="Box Select" />
                    </div>
                    <div class="shortcut-group">
                        <div class="shortcut-group-title">"Visibility"</div>
                        <ShortcutRow key="A" action="Show All" />
                        <ShortcutRow key="I" action="Isolate Selection" />
                        <ShortcutRow key="Del" action="Hide Selection" />
                    </div>
                    <div class="shortcut-group">
                        <div class="shortcut-group-title">"View"</div>
                        <ShortcutRow key="H" action="Home View" />
                        <ShortcutRow key="F" action="Fit All" />
                        <ShortcutRow key="T" action="Toggle Theme" />
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Single shortcut row
#[component]
fn ShortcutRow(key: &'static str, action: &'static str) -> impl IntoView {
    view! {
        <div class="shortcut-row">
            <span class="shortcut-key">{key}</span>
            <span class="shortcut-action">{action}</span>
        </div>
    }
}
