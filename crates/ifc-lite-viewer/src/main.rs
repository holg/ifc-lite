//! IFC-Lite Viewer Application
//!
//! Main entry point for the web-based IFC viewer.

use ifc_lite_yew::{bridge, ViewerLayout};
use yew::prelude::*;

/// Main application component
#[function_component]
fn App() -> Html {
    html! {
        <ViewerLayout />
    }
}

/// Main entry point
fn main() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

    // Initialize debug mode from URL (?debug=1)
    bridge::init_debug_from_url();

    // Start the Yew application
    yew::Renderer::<App>::new().render();
}
