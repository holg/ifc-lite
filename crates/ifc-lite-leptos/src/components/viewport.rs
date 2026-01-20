//! Viewport component - contains the Bevy canvas

use crate::bridge;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Bevy loading state
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BevyState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
}

/// Viewport component - manages Bevy canvas lifecycle
#[component]
pub fn Viewport() -> impl IntoView {
    let bevy_state = RwSignal::new(BevyState::NotLoaded);

    // Load Bevy on mount
    Effect::new(move |_| {
        // Check if already loaded
        if bridge::is_bevy_loaded() {
            bevy_state.set(BevyState::Loaded);
            return;
        }

        // Check if already loading
        if bridge::is_bevy_loading() {
            bevy_state.set(BevyState::Loading);
            return;
        }

        bevy_state.set(BevyState::Loading);
        bridge::log_info("[Leptos] Starting Bevy viewer...");

        spawn_local(async move {
            match bridge::load_bevy_viewer().await {
                Ok(_) => {
                    bridge::log_info("[Leptos] Bevy viewer started successfully");
                    bevy_state.set(BevyState::Loaded);
                }
                Err(e) => {
                    // Bevy throws an exception for control flow when starting
                    // This is expected behavior - check if it's the control flow exception
                    let error_str = format!("{:?}", e);
                    if error_str.contains("Using exceptions for control flow")
                        || error_str.contains("unreachable")
                    {
                        bridge::log_info("[Leptos] Bevy viewer running (control flow exception is normal)");
                        bevy_state.set(BevyState::Loaded);
                    } else {
                        bridge::log_error(&format!("[Leptos] Failed to start Bevy: {:?}", e));
                        bevy_state.set(BevyState::Error);
                    }
                }
            }
        });
    });

    view! {
        <div class="viewport">
            // Bevy canvas
            <canvas id="bevy-canvas" class="viewport-canvas"></canvas>

            // Loading overlay
            {move || {
                match bevy_state.get() {
                    BevyState::NotLoaded | BevyState::Loading => {
                        view! {
                            <div class="viewport-overlay loading-overlay">
                                <div class="loading-spinner"></div>
                                <span>"Starting 3D viewer..."</span>
                            </div>
                        }.into_any()
                    }
                    BevyState::Error => {
                        view! {
                            <div class="viewport-overlay error-overlay">
                                <span class="error-icon">"⚠️"</span>
                                <span>"Failed to start 3D viewer"</span>
                                <span class="error-hint">"Check browser console for details"</span>
                            </div>
                        }.into_any()
                    }
                    BevyState::Loaded => view! { <div style="display:none"></div> }.into_any(),
                }
            }}
        </div>
    }
}
