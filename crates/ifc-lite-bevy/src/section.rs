//! Section plane system
//!
//! Provides clipping plane functionality for viewing building cross-sections.

#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use crate::storage::load_section;
use crate::storage::SectionStorage;
use bevy::prelude::*;

/// Section plane plugin
pub struct SectionPlanePlugin;

impl Plugin for SectionPlanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SectionPlane>()
            .add_systems(Update, poll_section_settings);
    }
}

/// Section plane axis
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SectionAxis {
    X,
    #[default]
    Y,
    Z,
}

impl SectionAxis {
    /// Get plane normal vector
    pub fn normal(&self, flipped: bool) -> Vec3 {
        let base = match self {
            SectionAxis::X => Vec3::X,
            SectionAxis::Y => Vec3::Y,
            SectionAxis::Z => Vec3::Z,
        };
        if flipped {
            -base
        } else {
            base
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "x" => SectionAxis::X,
            "y" => SectionAxis::Y,
            _ => SectionAxis::Z,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            SectionAxis::X => "x",
            SectionAxis::Y => "y",
            SectionAxis::Z => "z",
        }
    }
}

/// Section plane state
#[derive(Resource)]
pub struct SectionPlane {
    /// Whether section plane is enabled
    pub enabled: bool,
    /// Section axis
    pub axis: SectionAxis,
    /// Position along axis (0.0 to 1.0 of scene bounds)
    pub position: f32,
    /// Whether plane normal is flipped
    pub flipped: bool,
    /// Cached plane equation (normal.xyz, distance)
    pub plane: Vec4,
}

impl Default for SectionPlane {
    fn default() -> Self {
        Self {
            enabled: false,
            axis: SectionAxis::Y,
            position: 0.5,
            flipped: false,
            plane: Vec4::new(0.0, 1.0, 0.0, 0.0),
        }
    }
}

impl SectionPlane {
    /// Set axis
    pub fn set_axis(&mut self, axis: SectionAxis) {
        self.axis = axis;
        self.update_plane();
    }

    /// Set position (0.0 to 1.0)
    pub fn set_position(&mut self, position: f32) {
        self.position = position.clamp(0.0, 1.0);
        self.update_plane();
    }

    /// Toggle flip
    pub fn toggle_flip(&mut self) {
        self.flipped = !self.flipped;
        self.update_plane();
    }

    /// Toggle enabled
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Update plane equation from current settings
    pub fn update_plane(&mut self) {
        let normal = self.axis.normal(self.flipped);
        // Distance is calculated based on position - will be updated with scene bounds
        self.plane = Vec4::new(normal.x, normal.y, normal.z, 0.0);
    }

    /// Update plane with scene bounds
    pub fn update_with_bounds(&mut self, min: Vec3, max: Vec3) {
        let normal = self.axis.normal(self.flipped);
        let axis_min = match self.axis {
            SectionAxis::X => min.x,
            SectionAxis::Y => min.y,
            SectionAxis::Z => min.z,
        };
        let axis_max = match self.axis {
            SectionAxis::X => max.x,
            SectionAxis::Y => max.y,
            SectionAxis::Z => max.z,
        };
        let distance = axis_min + (axis_max - axis_min) * self.position;
        self.plane = Vec4::new(normal.x, normal.y, normal.z, distance);
    }

    /// Load from storage
    pub fn from_storage(&mut self, storage: &SectionStorage) {
        self.enabled = storage.enabled;
        self.axis = SectionAxis::parse(&storage.axis);
        self.position = storage.position;
        self.flipped = storage.flipped;
        self.update_plane();
    }

    /// Convert to storage
    pub fn to_storage(&self) -> SectionStorage {
        SectionStorage {
            enabled: self.enabled,
            axis: self.axis.as_str().to_string(),
            position: self.position,
            flipped: self.flipped,
        }
    }
}

/// Poll section settings from localStorage
#[allow(unused_mut)]
fn poll_section_settings(mut section: ResMut<SectionPlane>) {
    #[cfg(target_arch = "wasm32")]
    {
        // Only poll occasionally
        static mut POLL_COUNTER: u32 = 0;
        unsafe {
            POLL_COUNTER += 1;
            if POLL_COUNTER % 30 == 0 {
                if let Some(storage) = load_section() {
                    if storage.enabled != section.enabled
                        || storage.axis != section.axis.as_str()
                        || storage.position != section.position
                        || storage.flipped != section.flipped
                    {
                        section.from_storage(&storage);
                    }
                }
            }
        }
    }

    // Suppress unused warning for native builds
    let _ = &section;
}

// Note: Actual clipping would require custom shaders.
// For a simpler approach, we can use Bevy's built-in clipping planes
// or implement material-based clipping in a custom shader.
//
// For now, this module provides the data structures and settings.
// The actual clipping can be implemented using:
// 1. Custom material with clip plane uniform
// 2. Bevy's ClipPlane component (if available)
// 3. Post-processing effect

/// Custom material with section plane support (placeholder)
/// To implement actual clipping, create a custom material:
///
/// ```glsl
/// // In fragment shader:
/// if (section_enabled) {
///     float d = dot(world_position.xyz, section_plane.xyz) - section_plane.w;
///     if (d > 0.0) discard;
/// }
/// ```
#[derive(Clone, Debug)]
pub struct SectionPlaneMaterial {
    pub base_color: Color,
    pub section_plane: Vec4,
    pub section_enabled: bool,
}
