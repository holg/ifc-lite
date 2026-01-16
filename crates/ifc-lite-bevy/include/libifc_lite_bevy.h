// IFC-Lite Bevy Viewer FFI Header
// Generated for iOS/macOS Swift integration

#ifndef LIBIFC_LITE_BEVY_H
#define LIBIFC_LITE_BEVY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque pointer to the Bevy application
typedef struct bevy_app bevy_app;

// ============================================================================
// App Lifecycle
// ============================================================================

/// Create a new Bevy app attached to a native Metal view
/// @param view_ptr Pointer to the UIView (iOS) or NSView (macOS) with CAMetalLayer backing
/// @param max_fps Maximum frames per second (use 60 or 120)
/// @param scale_factor Display scale factor (e.g., 2.0 for Retina)
/// @return Pointer to the Bevy app, or NULL on failure
bevy_app* create_bevy_app(void* view_ptr, int32_t max_fps, float scale_factor);

/// Process a single frame update. Call this from your display link callback.
/// @param app The Bevy app instance
void enter_frame(bevy_app* app);

/// Release the Bevy app and free all resources
/// @param app The Bevy app instance
void release_bevy_app(bevy_app* app);

// ============================================================================
// Data Loading
// ============================================================================

/// Load IFC geometry from JSON
/// @param app The Bevy app instance
/// @param meshes_json Null-terminated JSON string containing mesh data
/// @return true on success, false on failure
bool load_geometry(bevy_app* app, const char* meshes_json);

/// Load entity metadata from JSON
/// @param app The Bevy app instance
/// @param entities_json Null-terminated JSON string containing entity data
/// @return true on success, false on failure
bool load_entities(bevy_app* app, const char* entities_json);

// ============================================================================
// Selection
// ============================================================================

/// Select an entity by ID
/// @param app The Bevy app instance
/// @param entity_id The entity ID to select
void select_entity(bevy_app* app, uint64_t entity_id);

/// Clear the current selection
/// @param app The Bevy app instance
void clear_selection(bevy_app* app);

// ============================================================================
// Visibility
// ============================================================================

/// Hide an entity
/// @param app The Bevy app instance
/// @param entity_id The entity ID to hide
void hide_entity(bevy_app* app, uint64_t entity_id);

/// Show a hidden entity
/// @param app The Bevy app instance
/// @param entity_id The entity ID to show
void show_entity(bevy_app* app, uint64_t entity_id);

/// Show all hidden entities
/// @param app The Bevy app instance
void show_all(bevy_app* app);

/// Isolate entities (hide all others)
/// @param app The Bevy app instance
/// @param entity_ids Array of entity IDs to isolate
/// @param count Number of entity IDs in the array
void isolate_entities(bevy_app* app, const uint64_t* entity_ids, size_t count);

// ============================================================================
// Camera Control
// ============================================================================

/// Set camera to home (isometric) view
/// @param app The Bevy app instance
void camera_home(bevy_app* app);

/// Fit camera to show all geometry
/// @param app The Bevy app instance
void camera_fit_all(bevy_app* app);

/// Focus camera on a specific entity
/// @param app The Bevy app instance
/// @param entity_id The entity ID to focus on
void camera_focus_entity(bevy_app* app, uint64_t entity_id);

// ============================================================================
// Touch Input
// ============================================================================

/// Handle touch started event
/// @param app The Bevy app instance
/// @param x X coordinate in view coordinates
/// @param y Y coordinate in view coordinates
void touch_started(bevy_app* app, float x, float y);

/// Handle touch moved event
/// @param app The Bevy app instance
/// @param x X coordinate in view coordinates
/// @param y Y coordinate in view coordinates
void touch_moved(bevy_app* app, float x, float y);

/// Handle touch ended event
/// @param app The Bevy app instance
/// @param x X coordinate in view coordinates
/// @param y Y coordinate in view coordinates
void touch_ended(bevy_app* app, float x, float y);

/// Handle touch cancelled event
/// @param app The Bevy app instance
/// @param x X coordinate in view coordinates
/// @param y Y coordinate in view coordinates
void touch_cancelled(bevy_app* app, float x, float y);

// ============================================================================
// Theme
// ============================================================================

/// Set the viewer theme
/// @param app The Bevy app instance
/// @param dark true for dark theme, false for light theme
void set_theme(bevy_app* app, bool dark);

#ifdef __cplusplus
}
#endif

#endif // LIBIFC_LITE_BEVY_H
