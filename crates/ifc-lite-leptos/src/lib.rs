//! IFC-Lite Leptos UI Components
//!
//! Leptos-based UI components for the IFC-Lite viewer.
//! Provides a reactive UI that integrates with the Bevy 3D renderer.

pub mod bridge;
pub mod components;
pub mod state;
pub mod utils;

// Re-exports
pub use bridge::{
    init_debug_from_url, is_bevy_loaded, is_bevy_loading, is_unified_mode, load_bevy_viewer,
    save_camera_cmd, save_focus, save_geometry, save_palette, save_section, save_selection,
    save_visibility, CameraCommand, ColorPalette, EntityData, FocusData, GeometryData,
    SectionData, SelectionData, VisibilityData,
    // Cache exports
    compute_file_hash, is_model_cached, load_cached_model, save_model_to_cache,
    clear_model_cache, CachedModel, CacheEntry, CacheIndex,
};
pub use components::{App, ViewerLayout};
pub use state::{
    provide_viewer_state, use_viewer_state, EntityInfo, MeasurePoint, Measurement, Progress,
    PropertySet, PropertyValue, QuantityValue, SectionAxis, SpatialNode, SpatialNodeType,
    StoreyInfo, Theme, Tool, ViewerState,
};
