// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Geometry source trait for 3D rendering

use crate::{EntityId, IfcType, MeshData};
use std::sync::Arc;

/// Entity geometry with mesh, color, and transform
#[derive(Clone, Debug)]
pub struct EntityGeometry {
    /// Processed mesh data (shared via Arc)
    pub mesh: Arc<MeshData>,
    /// RGBA color [r, g, b, a] where values are 0.0-1.0
    pub color: [f32; 4],
    /// 4x4 transformation matrix (column-major order)
    pub transform: [f32; 16],
}

impl EntityGeometry {
    /// Create new entity geometry
    pub fn new(mesh: Arc<MeshData>, color: [f32; 4], transform: [f32; 16]) -> Self {
        Self {
            mesh,
            color,
            transform,
        }
    }

    /// Create geometry with identity transform
    pub fn with_identity_transform(mesh: Arc<MeshData>, color: [f32; 4]) -> Self {
        Self {
            mesh,
            color,
            transform: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// Check if geometry is empty
    pub fn is_empty(&self) -> bool {
        self.mesh.is_empty()
    }

    /// Get triangle count
    pub fn triangle_count(&self) -> usize {
        self.mesh.triangle_count()
    }
}

impl Default for EntityGeometry {
    fn default() -> Self {
        Self {
            mesh: Arc::new(MeshData::default()),
            color: [0.8, 0.8, 0.8, 1.0], // Light gray default
            transform: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

/// Geometry source for rendering
///
/// Provides access to processed geometry data ready for GPU rendering.
/// Implementations handle geometry processing, caching, and color assignment.
///
/// # Example
///
/// ```ignore
/// use ifc_lite_model::{GeometrySource, EntityId};
///
/// fn render_model(geometry: &dyn GeometrySource) {
///     // Get all entities with geometry
///     let entities = geometry.entities_with_geometry();
///     println!("Rendering {} entities", entities.len());
///
///     // Process each entity
///     for id in entities {
///         if let Some(geom) = geometry.get_geometry(id) {
///             println!("Entity {:?}: {} triangles", id, geom.triangle_count());
///             // Submit to GPU...
///         }
///     }
///
///     // Or batch process for efficiency
///     let all_geom = geometry.batch_geometry(&entities);
///     for (id, geom) in all_geom {
///         // Submit to GPU...
///     }
/// }
/// ```
pub trait GeometrySource: Send + Sync {
    /// Get all entity IDs that have processable geometry
    ///
    /// # Returns
    /// A vector of entity IDs that have geometry representations
    fn entities_with_geometry(&self) -> Vec<EntityId>;

    /// Check if an entity has processable geometry
    ///
    /// # Arguments
    /// * `id` - The entity ID to check
    ///
    /// # Returns
    /// `true` if the entity has geometry that can be processed
    fn has_geometry(&self, id: EntityId) -> bool;

    /// Get processed geometry for a single entity
    ///
    /// # Arguments
    /// * `id` - The entity ID to get geometry for
    ///
    /// # Returns
    /// The processed geometry if available
    fn get_geometry(&self, id: EntityId) -> Option<EntityGeometry>;

    /// Batch process geometry for multiple entities
    ///
    /// This is more efficient than calling `get_geometry` repeatedly
    /// as it can leverage caching and parallel processing.
    ///
    /// # Arguments
    /// * `ids` - The entity IDs to process
    ///
    /// # Returns
    /// A vector of (entity_id, geometry) pairs
    fn batch_geometry(&self, ids: &[EntityId]) -> Vec<(EntityId, EntityGeometry)> {
        ids.iter()
            .filter_map(|id| self.get_geometry(*id).map(|g| (*id, g)))
            .collect()
    }

    /// Get default color for an entity type
    ///
    /// Returns a color based on the IFC type for consistent visualization.
    ///
    /// # Arguments
    /// * `ifc_type` - The IFC type
    ///
    /// # Returns
    /// RGBA color array [r, g, b, a]
    fn default_color(&self, ifc_type: &IfcType) -> [f32; 4] {
        get_default_color(ifc_type)
    }

    /// Get total triangle count for all geometry
    fn total_triangle_count(&self) -> usize {
        self.entities_with_geometry()
            .iter()
            .filter_map(|id| self.get_geometry(*id))
            .map(|g| g.triangle_count())
            .sum()
    }
}

/// Get default color for an IFC type
///
/// Provides consistent colors for different element types.
pub fn get_default_color(ifc_type: &IfcType) -> [f32; 4] {
    match ifc_type {
        // Walls - light beige/tan
        IfcType::IfcWall | IfcType::IfcWallStandardCase => [0.85, 0.80, 0.70, 1.0],

        // Curtain walls - blue-gray glass
        IfcType::IfcCurtainWall => [0.6, 0.7, 0.8, 0.7],

        // Slabs/floors - light gray concrete
        IfcType::IfcSlab => [0.75, 0.75, 0.75, 1.0],

        // Roofs - terracotta/clay
        IfcType::IfcRoof => [0.72, 0.45, 0.35, 1.0],

        // Beams - structural blue-gray
        IfcType::IfcBeam => [0.55, 0.60, 0.65, 1.0],

        // Columns - structural gray
        IfcType::IfcColumn => [0.60, 0.60, 0.60, 1.0],

        // Doors - wood brown
        IfcType::IfcDoor => [0.55, 0.40, 0.25, 1.0],

        // Windows - light blue glass
        IfcType::IfcWindow => [0.7, 0.85, 0.95, 0.5],

        // Stairs - warm gray
        IfcType::IfcStair | IfcType::IfcStairFlight => [0.70, 0.68, 0.65, 1.0],

        // Ramps
        IfcType::IfcRamp | IfcType::IfcRampFlight => [0.70, 0.68, 0.65, 1.0],

        // Railings - metallic gray
        IfcType::IfcRailing => [0.50, 0.50, 0.55, 1.0],

        // Coverings - white
        IfcType::IfcCovering => [0.95, 0.95, 0.95, 1.0],

        // Plates - steel blue
        IfcType::IfcPlate => [0.60, 0.65, 0.70, 1.0],

        // Members - structural
        IfcType::IfcMember => [0.58, 0.58, 0.58, 1.0],

        // Footings - concrete gray
        IfcType::IfcFooting => [0.65, 0.65, 0.65, 1.0],

        // Piles - dark concrete
        IfcType::IfcPile => [0.55, 0.55, 0.55, 1.0],

        // Furniture - wood tones
        IfcType::IfcFurnishingElement | IfcType::IfcFurniture => [0.65, 0.50, 0.35, 1.0],

        // MEP elements - various colors
        IfcType::IfcDistributionElement | IfcType::IfcDistributionFlowElement => {
            [0.5, 0.7, 0.5, 1.0]
        }
        IfcType::IfcFlowTerminal => [0.7, 0.7, 0.5, 1.0],
        IfcType::IfcFlowSegment => [0.5, 0.5, 0.7, 1.0],
        IfcType::IfcFlowFitting => [0.6, 0.5, 0.6, 1.0],

        // Openings - transparent red (usually not rendered)
        IfcType::IfcOpeningElement | IfcType::IfcOpeningStandardCase => [1.0, 0.3, 0.3, 0.3],

        // Building element proxy - purple (catch-all)
        IfcType::IfcBuildingElementProxy => [0.7, 0.5, 0.8, 1.0],

        // Infrastructure elements
        IfcType::IfcRoad | IfcType::IfcRoadPart => [0.4, 0.4, 0.4, 1.0],
        IfcType::IfcBridge | IfcType::IfcBridgePart => [0.6, 0.6, 0.55, 1.0],
        IfcType::IfcRailway | IfcType::IfcRailwayPart => [0.5, 0.45, 0.4, 1.0],
        IfcType::IfcPavement => [0.35, 0.35, 0.35, 1.0],

        // Default - medium gray
        _ => [0.7, 0.7, 0.7, 1.0],
    }
}

/// Geometry processing options
#[derive(Clone, Debug, Default)]
pub struct GeometryOptions {
    /// Whether to compute normals if not provided
    pub compute_normals: bool,
    /// Whether to deduplicate identical meshes
    pub deduplicate: bool,
    /// Whether to merge small meshes
    pub merge_small_meshes: bool,
    /// Minimum triangle count for merging
    pub merge_threshold: usize,
}

impl GeometryOptions {
    /// Create options for fast processing (less optimization)
    pub fn fast() -> Self {
        Self {
            compute_normals: true,
            deduplicate: false,
            merge_small_meshes: false,
            merge_threshold: 0,
        }
    }

    /// Create options for optimized output (slower processing)
    pub fn optimized() -> Self {
        Self {
            compute_normals: true,
            deduplicate: true,
            merge_small_meshes: true,
            merge_threshold: 100,
        }
    }
}
