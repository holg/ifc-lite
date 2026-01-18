//! Apple FFI functions for iOS and macOS
//!
//! These functions are called from Swift to control the Bevy app.

use crate::{
    mesh::{IfcMesh, IfcMeshSerialized},
    native_view::AppViews,
    EntityInfo, IfcSceneData, IfcViewerPlugin, ViewerSettings,
};
use bevy::prelude::*;
use std::ffi::c_void;

/// Opaque app handle for FFI
pub struct BevyApp {
    app: App,
}

/// Create a new Bevy app attached to a native view
///
/// # Safety
/// - `view_ptr` must be a valid pointer to a UIView (iOS) or NSView (macOS)
/// - The view must have a CAMetalLayer as its backing layer
/// - The caller must ensure the view outlives the BevyApp
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_bevy_app(
    view_ptr: *mut c_void,
    _max_fps: i32,
    scale_factor: f32,
) -> *mut BevyApp {
    // Initialize logging
    #[cfg(debug_assertions)]
    {
        std::env::set_var("RUST_LOG", "info");
    }

    let mut app = App::new();

    // Create view object based on platform
    #[cfg(target_os = "ios")]
    let view_obj = crate::native_view::IOSViewObj {
        view: view_ptr,
        scale_factor,
    };

    #[cfg(target_os = "macos")]
    let view_obj = crate::native_view::MacOSViewObj {
        view: view_ptr,
        scale_factor,
    };

    // Initialize app views manager
    let mut app_views = AppViews::new();

    // Create initial window entity
    let window_entity = app.world_mut().spawn_empty().id();

    // Register the view
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        app_views.create_window(view_obj, window_entity);
    }

    // Insert resources before plugins
    app.insert_resource(IfcSceneData::default());
    app.insert_resource(ViewerSettings::default());
    app.insert_non_send_resource(app_views);

    // Add default plugins with custom window settings
    // Note: We don't use WinitPlugin since we have our own window management
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "IFC-Lite Viewer".to_string(),
                    resolution: (800u32, 600u32).into(),
                    present_mode: bevy::window::PresentMode::AutoVsync,
                    ..default()
                }),
                ..default()
            })
            .build(),
    );

    // Add IFC viewer plugin
    app.add_plugins(IfcViewerPlugin);
    app.add_plugins(crate::native_view::AppViewPlugin);

    let bevy_app = Box::new(BevyApp { app });
    Box::into_raw(bevy_app)
}

/// Process a single frame update
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn enter_frame(bevy_app: *mut BevyApp) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;
    app.update();
}

/// Release the Bevy app and free memory
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
/// - After calling this function, the pointer is invalid
#[unsafe(no_mangle)]
pub unsafe extern "C" fn release_bevy_app(bevy_app: *mut BevyApp) {
    if bevy_app.is_null() {
        return;
    }

    let _ = Box::from_raw(bevy_app);
}

/// Load IFC geometry into the viewer
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
/// - `meshes_json` must be a valid null-terminated JSON string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_geometry(
    bevy_app: *mut BevyApp,
    meshes_json: *const std::ffi::c_char,
) -> bool {
    if bevy_app.is_null() || meshes_json.is_null() {
        return false;
    }

    let json_str = match std::ffi::CStr::from_ptr(meshes_json).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Deserialize to serialized format, then convert to Arc-based IfcMesh
    let serialized: Vec<IfcMeshSerialized> = match serde_json::from_str(json_str) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to parse meshes JSON: {}", e);
            return false;
        }
    };
    let meshes: Vec<IfcMesh> = serialized.into_iter().map(IfcMesh::from).collect();

    let app = &mut (*bevy_app).app;

    if let Some(mut scene_data) = app.world_mut().get_resource_mut::<IfcSceneData>() {
        scene_data.meshes = meshes;
        scene_data.dirty = true;
        true
    } else {
        false
    }
}

/// Load entity metadata
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
/// - `entities_json` must be a valid null-terminated JSON string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_entities(
    bevy_app: *mut BevyApp,
    entities_json: *const std::ffi::c_char,
) -> bool {
    if bevy_app.is_null() || entities_json.is_null() {
        return false;
    }

    let json_str = match std::ffi::CStr::from_ptr(entities_json).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let entities: Vec<EntityInfo> = match serde_json::from_str(json_str) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to parse entities JSON: {}", e);
            return false;
        }
    };

    let app = &mut (*bevy_app).app;

    if let Some(mut scene_data) = app.world_mut().get_resource_mut::<IfcSceneData>() {
        scene_data.entities = entities;
        true
    } else {
        false
    }
}

/// Select an entity by ID
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn select_entity(bevy_app: *mut BevyApp, entity_id: u64) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;

    if let Some(mut selection) = app.world_mut().get_resource_mut::<crate::SelectionState>() {
        selection.selected.clear();
        selection.selected.insert(entity_id);
    }
}

/// Clear selection
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clear_selection(bevy_app: *mut BevyApp) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;

    if let Some(mut selection) = app.world_mut().get_resource_mut::<crate::SelectionState>() {
        selection.selected.clear();
        selection.hovered = None;
    }
}

/// Hide an entity
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hide_entity(bevy_app: *mut BevyApp, entity_id: u64) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;

    if let Some(mut settings) = app.world_mut().get_resource_mut::<ViewerSettings>() {
        settings.hidden_entities.insert(entity_id);
    }
}

/// Show an entity
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn show_entity(bevy_app: *mut BevyApp, entity_id: u64) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;

    if let Some(mut settings) = app.world_mut().get_resource_mut::<ViewerSettings>() {
        settings.hidden_entities.remove(&entity_id);
    }
}

/// Show all entities
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn show_all(bevy_app: *mut BevyApp) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;

    if let Some(mut settings) = app.world_mut().get_resource_mut::<ViewerSettings>() {
        settings.hidden_entities.clear();
        settings.isolated_entities = None;
    }
}

/// Isolate entities (hide all others)
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
/// - `entity_ids` must be a valid array of `count` u64 values
#[unsafe(no_mangle)]
pub unsafe extern "C" fn isolate_entities(
    bevy_app: *mut BevyApp,
    entity_ids: *const u64,
    count: usize,
) {
    if bevy_app.is_null() || entity_ids.is_null() {
        return;
    }

    let ids = std::slice::from_raw_parts(entity_ids, count);
    let app = &mut (*bevy_app).app;

    if let Some(mut settings) = app.world_mut().get_resource_mut::<ViewerSettings>() {
        settings.isolated_entities = Some(ids.iter().copied().collect());
    }
}

/// Set camera home view
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn camera_home(bevy_app: *mut BevyApp) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;

    if let Some(mut controller) = app
        .world_mut()
        .get_resource_mut::<crate::CameraController>()
    {
        controller.home();
    }
}

/// Fit camera to show all geometry
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn camera_fit_all(bevy_app: *mut BevyApp) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;
    let world = app.world_mut();

    // Get bounds from scene data
    let bounds = world
        .get_resource::<IfcSceneData>()
        .and_then(|data| data.bounds.clone());

    if let (Some(bounds), Some(mut controller)) =
        (bounds, world.get_resource_mut::<crate::CameraController>())
    {
        controller.fit_bounds(bounds.min, bounds.max);
    }
}

/// Focus camera on a specific entity
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn camera_focus_entity(bevy_app: *mut BevyApp, entity_id: u64) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;

    // Set pending focus - the system will handle it
    if let Some(mut pending) = app
        .world_mut()
        .get_resource_mut::<crate::mesh::PendingFocus>()
    {
        pending.entity_id = Some(entity_id);
    }
}

/// Handle touch started event
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn touch_started(bevy_app: *mut BevyApp, x: f32, y: f32) {
    touch_event(bevy_app, x, y, TouchPhase::Started);
}

/// Handle touch moved event
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn touch_moved(bevy_app: *mut BevyApp, x: f32, y: f32) {
    touch_event(bevy_app, x, y, TouchPhase::Moved);
}

/// Handle touch ended event
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn touch_ended(bevy_app: *mut BevyApp, x: f32, y: f32) {
    touch_event(bevy_app, x, y, TouchPhase::Ended);
}

/// Handle touch cancelled event
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn touch_cancelled(bevy_app: *mut BevyApp, x: f32, y: f32) {
    touch_event(bevy_app, x, y, TouchPhase::Canceled);
}

/// Internal touch event handler
unsafe fn touch_event(bevy_app: *mut BevyApp, x: f32, y: f32, phase: TouchPhase) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;
    let world = app.world_mut();

    // Get the primary window entity
    let window_entity = world
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .iter(world)
        .next();

    if let Some(window_entity) = window_entity {
        let touch = TouchInput {
            phase,
            position: Vec2::new(x, y),
            window: window_entity,
            force: None,
            id: 0,
        };

        if let Some(mut messages) = world.get_resource_mut::<Messages<TouchInput>>() {
            messages.write(touch);
        }
    }
}

/// Set theme (dark/light)
///
/// # Safety
/// - `bevy_app` must be a valid pointer returned by `create_bevy_app`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_theme(bevy_app: *mut BevyApp, dark: bool) {
    if bevy_app.is_null() {
        return;
    }

    let app = &mut (*bevy_app).app;

    if let Some(mut settings) = app.world_mut().get_resource_mut::<ViewerSettings>() {
        settings.theme = if dark {
            crate::Theme::Dark
        } else {
            crate::Theme::Light
        };
    }
}

use bevy::ecs::message::Messages;
use bevy::input::touch::{TouchInput, TouchPhase};
