//! AppView plugin for Bevy
//!
//! Replaces WinitPlugin for embedded native views.

use super::AppViews;
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window, WindowResized};

/// Plugin that manages native views instead of using winit
pub struct AppViewPlugin;

impl Plugin for AppViewPlugin {
    fn build(&self, app: &mut App) {
        // Note: AppViews is already inserted by the FFI create_bevy_app function
        // We only add the resize system here
        app.add_systems(Last, update_window_size);
    }
}

/// System to update window size when the native view resizes
fn update_window_size(
    app_views: Option<NonSend<AppViews>>,
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut resize_events: MessageWriter<WindowResized>,
) {
    let Some(app_views) = app_views else { return };

    if let Some(view_window) = app_views.first_view() {
        let (width, height) = view_window.view.logical_resolution();
        let scale = view_window.view.scale_factor();

        for (entity, mut window) in windows.iter_mut() {
            // Check if size changed
            let current_width = window.resolution.width();
            let current_height = window.resolution.height();

            if (current_width - width).abs() > 1.0 || (current_height - height).abs() > 1.0 {
                window.resolution.set(width, height);
                window.resolution.set_scale_factor(scale);

                resize_events.write(WindowResized {
                    window: entity,
                    width,
                    height,
                });
            }
        }
    }
}
