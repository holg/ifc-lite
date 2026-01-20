// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! # IFC-Lite Geometry Processing (Trait-Based)
//!
//! Efficient geometry processing for IFC models using trait-based architecture.
//! This crate uses the `EntityResolver` trait from `ifc-lite-model` for entity
//! lookup, making it independent of any specific parser implementation.
//!
//! ## Overview
//!
//! This crate transforms IFC geometry representations into GPU-ready triangle meshes:
//!
//! - **Profile Handling**: Extract and process 2D profiles (rectangle, circle, arbitrary)
//! - **Extrusion**: Generate 3D meshes from extruded profiles
//! - **Triangulation**: Polygon triangulation with hole support via earcutr
//! - **Mesh Processing**: Normal calculation and coordinate transformations
//!
//! ## Architecture
//!
//! The crate uses traits for abstraction:
//!
//! - `GeometryProcessor`: Trait for individual geometry type processors
//! - `EntityResolver`: Trait from ifc-lite-model for entity lookup
//! - `GeometrySource`: Trait from ifc-lite-model for geometry access
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use ifc_lite_geometry::{
//!     Profile2D, extrude_profile,
//!     Point2, Point3, Vector3
//! };
//!
//! // Create a rectangular profile
//! let profile = Profile2D::rectangle(2.0, 1.0);
//!
//! // Extrude to 3D
//! let mesh = extrude_profile(&profile, 3.0, None)?;
//!
//! println!("Generated {} triangles", mesh.triangle_count());
//! ```

pub mod error;
pub mod extrusion;
pub mod mesh;
pub mod processors;
pub mod profile;
pub mod router;
pub mod triangulation;

// Re-export nalgebra types for convenience
pub use nalgebra::{Point2, Point3, Vector2, Vector3};

// Re-export main types
pub use error::{Error, Result};
pub use extrusion::{apply_transform, extrude_profile, extrude_profile_with_voids};
pub use mesh::Mesh;
pub use profile::{
    calculate_circle_segments, Profile2D, Profile2DWithVoids, ProfileType, Triangulation, VoidInfo,
};
pub use router::{GeometryProcessor, GeometryRouter};
pub use triangulation::{
    calculate_polygon_normal, project_to_2d, project_to_2d_with_basis, triangulate_polygon,
    triangulate_polygon_with_holes,
};

// Re-export processors
pub use processors::{
    ExtrudedAreaSolidProcessor, FacetedBrepProcessor, RevolvedAreaSolidProcessor,
    SweptDiskSolidProcessor, TriangulatedFaceSetProcessor,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_extrusion() {
        let profile = Profile2D::rectangle(10.0, 5.0);
        let mesh = extrude_profile(&profile, 20.0, None).unwrap();

        assert!(mesh.vertex_count() > 0);
        assert!(mesh.triangle_count() > 0);
    }

    #[test]
    fn test_mesh_to_mesh_data() {
        let profile = Profile2D::rectangle(1.0, 1.0);
        let mesh = extrude_profile(&profile, 1.0, None).unwrap();
        let mesh_data = mesh.to_mesh_data();

        assert_eq!(mesh_data.positions.len(), mesh.positions.len());
        assert_eq!(mesh_data.normals.len(), mesh.normals.len());
        assert_eq!(mesh_data.indices.len(), mesh.indices.len());
    }
}
