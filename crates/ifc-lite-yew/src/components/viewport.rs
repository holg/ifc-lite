//! Viewport component - embeds Bevy canvas

use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::state::ViewerStateContext;
use crate::bridge::{load_bevy_viewer, is_bevy_loaded, log, log_error};

/// Bevy loading state
#[derive(Clone, Copy, PartialEq)]
enum BevyState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
}

/// Viewport component
#[function_component]
pub fn Viewport() -> Html {
    let _state = use_context::<ViewerStateContext>().expect("ViewerStateContext not found");
    let bevy_state = use_state(|| BevyState::NotLoaded);
    let error_msg = use_state(String::new);

    // Load Bevy on mount
    {
        let bevy_state = bevy_state.clone();
        let error_msg = error_msg.clone();

        use_effect_with((), move |_| {
            // Check if already loaded
            if is_bevy_loaded() {
                bevy_state.set(BevyState::Loaded);
                return;
            }

            bevy_state.set(BevyState::Loading);
            log("[Yew] Loading Bevy viewer...");

            spawn_local(async move {
                match load_bevy_viewer().await {
                    Ok(_) => {
                        log("[Yew] Bevy viewer loaded successfully");
                        bevy_state.set(BevyState::Loaded);
                    }
                    Err(e) => {
                        // Bevy uses exceptions for control flow - check if this is one
                        let error_str = format!("{:?}", e);
                        if error_str.contains("Using exceptions for control flow") {
                            // This is normal Bevy behavior, not an error
                            log("[Yew] Bevy event loop started");
                            bevy_state.set(BevyState::Loaded);
                        } else {
                            log_error(&format!("[Yew] Failed to load Bevy: {}", error_str));
                            error_msg.set(error_str);
                            bevy_state.set(BevyState::Error);
                        }
                    }
                }
            });
        });
    }

    html! {
        <div class="viewport">
            // Bevy canvas
            <canvas
                id="bevy-canvas"
                class="viewport-canvas"
            />

            // Loading overlay
            if *bevy_state == BevyState::Loading {
                <div class="viewport-overlay loading-overlay">
                    <div class="loading-content">
                        <span class="loading-spinner large" />
                        <span class="loading-text">{"Loading 3D viewer..."}</span>
                    </div>
                </div>
            }

            // Error overlay
            if *bevy_state == BevyState::Error {
                <div class="viewport-overlay error-overlay">
                    <div class="error-content">
                        <span class="error-icon">{"⚠️"}</span>
                        <span class="error-title">{"Failed to load 3D viewer"}</span>
                        <span class="error-message">{&*error_msg}</span>
                        <button
                            class="retry-btn"
                            onclick={
                                let bevy_state = bevy_state.clone();
                                Callback::from(move |_| {
                                    bevy_state.set(BevyState::NotLoaded);
                                    // Re-trigger effect by changing state
                                })
                            }
                        >
                            {"Retry"}
                        </button>
                    </div>
                </div>
            }

            // Not loaded overlay (before effect runs)
            if *bevy_state == BevyState::NotLoaded {
                <div class="viewport-overlay">
                    <div class="loading-content">
                        <span class="loading-text">{"Initializing..."}</span>
                    </div>
                </div>
            }
        </div>
    }
}
