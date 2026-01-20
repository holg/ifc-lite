// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! 2D Profile definitions and triangulation

use crate::error::{Error, Result};
use nalgebra::Point2;

/// 2D Profile with optional holes
#[derive(Debug, Clone)]
pub struct Profile2D {
    /// Outer boundary (counter-clockwise)
    pub outer: Vec<Point2<f64>>,
    /// Holes (clockwise)
    pub holes: Vec<Vec<Point2<f64>>>,
}

impl Profile2D {
    /// Create a new profile
    pub fn new(outer: Vec<Point2<f64>>) -> Self {
        Self {
            outer,
            holes: Vec::new(),
        }
    }

    /// Add a hole to the profile
    pub fn add_hole(&mut self, hole: Vec<Point2<f64>>) {
        self.holes.push(hole);
    }

    /// Triangulate the profile using earcutr
    /// Returns triangle indices into the flattened vertex array
    pub fn triangulate(&self) -> Result<Triangulation> {
        if self.outer.len() < 3 {
            return Err(Error::profile(
                "Profile must have at least 3 vertices",
            ));
        }

        // Flatten vertices for earcutr
        let mut vertices = Vec::with_capacity(
            (self.outer.len() + self.holes.iter().map(|h| h.len()).sum::<usize>()) * 2,
        );

        // Add outer boundary
        for p in &self.outer {
            vertices.push(p.x);
            vertices.push(p.y);
        }

        // Add holes
        let mut hole_indices = Vec::with_capacity(self.holes.len());
        for hole in &self.holes {
            hole_indices.push(vertices.len() / 2);
            for p in hole {
                vertices.push(p.x);
                vertices.push(p.y);
            }
        }

        // Triangulate
        let indices = if hole_indices.is_empty() {
            earcutr::earcut(&vertices, &[], 2)
                .map_err(|e| Error::triangulation(format!("{:?}", e)))?
        } else {
            earcutr::earcut(&vertices, &hole_indices, 2)
                .map_err(|e| Error::triangulation(format!("{:?}", e)))?
        };

        // Convert to Point2 array
        let mut points = Vec::with_capacity(vertices.len() / 2);
        for i in (0..vertices.len()).step_by(2) {
            points.push(Point2::new(vertices[i], vertices[i + 1]));
        }

        Ok(Triangulation { points, indices })
    }

    /// Create a rectangular profile centered at origin
    pub fn rectangle(width: f64, height: f64) -> Self {
        let half_w = width / 2.0;
        let half_h = height / 2.0;

        Self::new(vec![
            Point2::new(-half_w, -half_h),
            Point2::new(half_w, -half_h),
            Point2::new(half_w, half_h),
            Point2::new(-half_w, half_h),
        ])
    }

    /// Create a circular profile centered at origin
    pub fn circle(radius: f64, segments: Option<usize>) -> Self {
        let segments = segments.unwrap_or_else(|| calculate_circle_segments(radius));
        let mut outer = Vec::with_capacity(segments);

        for i in 0..segments {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (segments as f64);
            outer.push(Point2::new(radius * angle.cos(), radius * angle.sin()));
        }

        Self::new(outer)
    }
}

/// Triangulated profile result
#[derive(Debug, Clone)]
pub struct Triangulation {
    /// All vertices (outer + holes)
    pub points: Vec<Point2<f64>>,
    /// Triangle indices
    pub indices: Vec<usize>,
}

/// Void metadata for depth-aware extrusion
///
/// Tracks information about a void that has been projected to the 2D profile plane,
/// including its depth range for generating internal caps when the void doesn't
/// extend through the full extrusion depth.
#[derive(Debug, Clone)]
pub struct VoidInfo {
    /// Hole contour in 2D profile space (clockwise winding for holes)
    pub contour: Vec<Point2<f64>>,
    /// Start depth in extrusion space (0.0 = bottom cap)
    pub depth_start: f64,
    /// End depth in extrusion space (extrusion_depth = top cap)
    pub depth_end: f64,
    /// Whether void extends full depth (no internal caps needed)
    pub is_through: bool,
}

impl VoidInfo {
    /// Create a new void info
    pub fn new(
        contour: Vec<Point2<f64>>,
        depth_start: f64,
        depth_end: f64,
        is_through: bool,
    ) -> Self {
        Self {
            contour,
            depth_start,
            depth_end,
            is_through,
        }
    }

    /// Create a through void (extends full depth)
    pub fn through(contour: Vec<Point2<f64>>, depth: f64) -> Self {
        Self {
            contour,
            depth_start: 0.0,
            depth_end: depth,
            is_through: true,
        }
    }
}

/// Profile with void tracking for depth-aware extrusion
///
/// Extends Profile2D with metadata about voids that have been classified as
/// coplanar and can be handled at the profile level.
#[derive(Debug, Clone)]
pub struct Profile2DWithVoids {
    /// Base profile (outer boundary + any existing holes)
    pub profile: Profile2D,
    /// Void metadata for depth-aware extrusion
    pub voids: Vec<VoidInfo>,
}

impl Profile2DWithVoids {
    /// Create a new profile with voids
    pub fn new(profile: Profile2D, voids: Vec<VoidInfo>) -> Self {
        Self { profile, voids }
    }

    /// Create from a base profile with no voids
    pub fn from_profile(profile: Profile2D) -> Self {
        Self {
            profile,
            voids: Vec::new(),
        }
    }

    /// Add a void to the profile
    pub fn add_void(&mut self, void_info: VoidInfo) {
        self.voids.push(void_info);
    }

    /// Get all through voids (can be added as simple holes)
    pub fn through_voids(&self) -> impl Iterator<Item = &VoidInfo> {
        self.voids.iter().filter(|v| v.is_through)
    }

    /// Get all partial-depth voids (need internal caps)
    pub fn partial_voids(&self) -> impl Iterator<Item = &VoidInfo> {
        self.voids.iter().filter(|v| !v.is_through)
    }

    /// Check if there are any voids
    pub fn has_voids(&self) -> bool {
        !self.voids.is_empty()
    }

    /// Get number of voids
    pub fn void_count(&self) -> usize {
        self.voids.len()
    }

    /// Create a profile with through-voids merged as holes
    pub fn profile_with_through_holes(&self) -> Profile2D {
        let mut profile = self.profile.clone();

        for void_info in self.through_voids() {
            profile.add_hole(void_info.contour.clone());
        }

        profile
    }
}

/// Common profile types
#[derive(Debug, Clone)]
pub enum ProfileType {
    Rectangle { width: f64, height: f64 },
    Circle { radius: f64 },
    HollowCircle { outer_radius: f64, inner_radius: f64 },
    Polygon { points: Vec<Point2<f64>> },
}

impl ProfileType {
    /// Convert to Profile2D
    pub fn to_profile(&self) -> Profile2D {
        match self {
            Self::Rectangle { width, height } => Profile2D::rectangle(*width, *height),
            Self::Circle { radius } => Profile2D::circle(*radius, None),
            Self::HollowCircle { outer_radius, inner_radius } => {
                let mut profile = Profile2D::circle(*outer_radius, None);
                let hole_segments = calculate_circle_segments(*inner_radius);
                let mut hole = Vec::with_capacity(hole_segments);
                for i in 0..hole_segments {
                    let angle = 2.0 * std::f64::consts::PI * (i as f64) / (hole_segments as f64);
                    hole.push(Point2::new(inner_radius * angle.cos(), inner_radius * angle.sin()));
                }
                hole.reverse(); // Clockwise for hole
                profile.add_hole(hole);
                profile
            }
            Self::Polygon { points } => Profile2D::new(points.clone()),
        }
    }
}

/// Calculate adaptive number of segments for a circle
#[inline]
pub fn calculate_circle_segments(radius: f64) -> usize {
    let segments = (radius.sqrt() * 8.0).ceil() as usize;
    segments.clamp(8, 32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rectangle_profile() {
        let profile = Profile2D::rectangle(10.0, 5.0);
        assert_eq!(profile.outer.len(), 4);
        assert_eq!(profile.holes.len(), 0);
    }

    #[test]
    fn test_circle_profile() {
        let profile = Profile2D::circle(5.0, None);
        assert!(profile.outer.len() >= 8);
        assert_eq!(profile.holes.len(), 0);
    }

    #[test]
    fn test_triangulate_rectangle() {
        let profile = Profile2D::rectangle(10.0, 5.0);
        let tri = profile.triangulate().unwrap();
        assert_eq!(tri.points.len(), 4);
        assert_eq!(tri.indices.len(), 6);
    }
}
