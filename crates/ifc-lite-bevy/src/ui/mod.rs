//! Bevy UI components for IFC-Lite viewer
//!
//! Pure Bevy UI implementation - works on both web and native.

mod hierarchy;
mod layout;
mod properties;
mod styles;
mod toolbar;

pub use hierarchy::*;
pub use layout::*;
pub use properties::*;
pub use styles::*;
pub use toolbar::{ButtonAction, ToolbarButton, ToolbarPlugin};

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, ScrollPosition};

/// Main UI plugin - combines all UI components
pub struct IfcUiPlugin;

impl Plugin for IfcUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .add_plugins((
                LayoutPlugin,
                ToolbarPlugin,
                HierarchyPlugin,
                PropertiesPlugin,
            ))
            .add_systems(Update, ui_scroll_system);
    }
}

/// Marker for scrollable panels that need manual scroll handling
#[derive(Component)]
pub struct ScrollablePanel;

/// System to handle mouse wheel scrolling in UI panels
/// Uses cursor position to check if within scrollable panel bounds
fn ui_scroll_system(
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut scrollable_query: Query<
        (&mut ScrollPosition, &ComputedNode, &GlobalTransform),
        With<ScrollablePanel>,
    >,
    windows: Query<&Window>,
    mut logged: Local<bool>,
) {
    const LINE_HEIGHT: f32 = 40.0; // Larger scroll step

    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Debug: log panel info once
    if !*logged {
        *logged = true;
        let count = scrollable_query.iter().count();
        println!("[Scroll] Found {} scrollable panels", count);
        for (_scroll_pos, computed, global_transform) in scrollable_query.iter() {
            let node_pos = global_transform.translation().truncate();
            let node_size = computed.size();
            println!("[Scroll] Panel at {:?}, size {:?}", node_pos, node_size);
        }
    }

    for ev in mouse_wheel.read() {
        let delta_y = -ev.y * LINE_HEIGHT;

        // Find scrollable panels under cursor
        for (mut scroll_pos, computed, global_transform) in scrollable_query.iter_mut() {
            // Get node position and size
            let node_pos = global_transform.translation().truncate();
            let node_size = computed.size();
            let half_size = node_size / 2.0;

            let min_x = node_pos.x - half_size.x;
            let max_x = node_pos.x + half_size.x;
            let min_y = node_pos.y - half_size.y;
            let max_y = node_pos.y + half_size.y;

            // Check if cursor is within bounds
            if cursor_pos.x >= min_x
                && cursor_pos.x <= max_x
                && cursor_pos.y >= min_y
                && cursor_pos.y <= max_y
            {
                scroll_pos.y += delta_y;
                scroll_pos.y = scroll_pos.y.max(0.0);
                println!("[Scroll] Scrolling to y={}", scroll_pos.y);
                // Only scroll one panel per event
                break;
            }
        }
    }
}

/// Global UI state
#[derive(Resource)]
pub struct UiState {
    /// Left panel (hierarchy) visible
    pub show_hierarchy: bool,
    /// Right panel (properties) visible
    pub show_properties: bool,
    /// Current search filter
    pub search_filter: String,
    /// Selected storey filter
    pub storey_filter: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_hierarchy: true,
            show_properties: true,
            search_filter: String::new(),
            storey_filter: None,
        }
    }
}
