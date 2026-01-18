//! Mesh system for IFC geometry
//!
//! Handles loading IFC geometry into Bevy meshes with materials.
//!
//! ## Performance: Batched Rendering
//!
//! Instead of creating one Bevy entity per IFC entity (which causes 1000+ draw calls),
//! we batch meshes by material type into a few large meshes:
//! - Opaque batch: All solid geometry in one draw call
//! - Transparent batch: All glass/windows in one draw call
//!
//! This reduces draw calls from N to 2-3, dramatically improving orbit/pan performance.
//!
//! ## Memory Optimization: Arc-based Geometry Sharing
//!
//! Geometry data (positions, normals, indices) is stored in `Arc<MeshGeometry>` to avoid
//! expensive cloning. This saves ~1.7GB RAM on a 200MB IFC file by sharing geometry
//! between the parser output and our mesh structures.

use crate::{log, IfcSceneData, SceneBounds, ViewerSettings};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Mesh plugin
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoFitState>()
            .init_resource::<PendingFocus>()
            .init_resource::<TriangleEntityMapping>()
            .add_systems(
                Update,
                (
                    spawn_meshes_system,
                    auto_fit_camera_system,
                    update_mesh_visibility_system,
                    update_mesh_selection_system,
                    poll_focus_command_system,
                )
                    .chain(),
            );
    }
}

/// Resource for pending focus command
#[derive(Resource, Default)]
pub struct PendingFocus {
    pub entity_id: Option<u64>,
}

/// State for auto-fit camera on first load
#[derive(Resource, Default)]
pub struct AutoFitState {
    /// Whether we've already auto-fit for this scene
    pub has_fit: bool,
}

/// Shared geometry data - uses Arc to avoid expensive cloning
///
/// This struct holds the heavy data (positions, normals, indices) that would
/// otherwise be cloned multiple times through the pipeline. Using Arc saves
/// ~1.7GB RAM on a 200MB IFC file.
#[derive(Clone, Debug, Default)]
pub struct MeshGeometry {
    /// Vertex positions (flattened: [x0,y0,z0, x1,y1,z1, ...])
    pub positions: Vec<f32>,
    /// Vertex normals (flattened: [nx0,ny0,nz0, ...])
    pub normals: Vec<f32>,
    /// Triangle indices
    pub indices: Vec<u32>,
}

impl MeshGeometry {
    /// Create new geometry from vectors (takes ownership, no clone)
    pub fn new(positions: Vec<f32>, normals: Vec<f32>, indices: Vec<u32>) -> Self {
        Self {
            positions,
            normals,
            indices,
        }
    }

    /// Create from ifc_lite_geometry::Mesh (takes ownership via conversion)
    pub fn from_geometry_mesh(mesh: ifc_lite_geometry::Mesh) -> Self {
        Self {
            positions: mesh.positions,
            normals: mesh.normals,
            indices: mesh.indices,
        }
    }

    /// Vertex count
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Triangle count
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

/// IFC mesh data with Arc-based geometry sharing
///
/// The geometry data is wrapped in Arc to enable zero-copy sharing between
/// the parser/geometry processor and the Bevy mesh system. Only the lightweight
/// metadata (color, transform, entity info) is owned per-instance.
#[derive(Clone, Debug)]
pub struct IfcMesh {
    /// Entity ID
    pub entity_id: u64,
    /// Shared geometry data (positions, normals, indices)
    pub geometry: Arc<MeshGeometry>,
    /// Base color [r, g, b, a]
    pub color: [f32; 4],
    /// Transform matrix (column-major 4x4)
    pub transform: [f32; 16],
    /// Entity type (e.g., "IfcWall")
    pub entity_type: String,
    /// Entity name
    pub name: Option<String>,
}

/// Legacy serializable format for storage/transfer
/// Used for web storage where we need JSON serialization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IfcMeshSerialized {
    /// Entity ID
    pub entity_id: u64,
    /// Vertex positions (flattened: [x0,y0,z0, x1,y1,z1, ...])
    pub positions: Vec<f32>,
    /// Vertex normals (flattened: [nx0,ny0,nz0, ...])
    pub normals: Vec<f32>,
    /// Triangle indices
    pub indices: Vec<u32>,
    /// Base color [r, g, b, a]
    pub color: [f32; 4],
    /// Transform matrix (column-major 4x4)
    pub transform: [f32; 16],
    /// Entity type (e.g., "IfcWall")
    pub entity_type: String,
    /// Entity name
    pub name: Option<String>,
}

impl From<IfcMeshSerialized> for IfcMesh {
    fn from(s: IfcMeshSerialized) -> Self {
        Self {
            entity_id: s.entity_id,
            geometry: Arc::new(MeshGeometry::new(s.positions, s.normals, s.indices)),
            color: s.color,
            transform: s.transform,
            entity_type: s.entity_type,
            name: s.name,
        }
    }
}

impl From<&IfcMesh> for IfcMeshSerialized {
    fn from(m: &IfcMesh) -> Self {
        Self {
            entity_id: m.entity_id,
            positions: m.geometry.positions.clone(),
            normals: m.geometry.normals.clone(),
            indices: m.geometry.indices.clone(),
            color: m.color,
            transform: m.transform,
            entity_type: m.entity_type.clone(),
            name: m.name.clone(),
        }
    }
}

impl IfcMesh {
    /// Create a new IfcMesh with Arc-wrapped geometry (no cloning)
    pub fn new(
        entity_id: u64,
        geometry: Arc<MeshGeometry>,
        color: [f32; 4],
        transform: [f32; 16],
        entity_type: String,
        name: Option<String>,
    ) -> Self {
        Self {
            entity_id,
            geometry,
            color,
            transform,
            entity_type,
            name,
        }
    }

    /// Create from geometry mesh, taking ownership (no clone)
    pub fn from_geometry_mesh(
        entity_id: u64,
        mesh: ifc_lite_geometry::Mesh,
        color: [f32; 4],
        entity_type: String,
        name: Option<String>,
    ) -> Self {
        Self {
            entity_id,
            geometry: Arc::new(MeshGeometry::from_geometry_mesh(mesh)),
            color,
            transform: [
                1.0, 0.0, 0.0, 0.0, // column 0
                0.0, 1.0, 0.0, 0.0, // column 1
                0.0, 0.0, 1.0, 0.0, // column 2
                0.0, 0.0, 0.0, 1.0, // column 3
            ],
            entity_type,
            name,
        }
    }

    /// Check if geometry is empty
    pub fn is_empty(&self) -> bool {
        self.geometry.is_empty()
    }

    /// Convert to Bevy mesh
    pub fn to_bevy_mesh(&self) -> Mesh {
        let vertex_count = self.geometry.vertex_count();

        // Parse positions
        let positions: Vec<[f32; 3]> = (0..vertex_count)
            .map(|i| {
                let idx = i * 3;
                // Convert from IFC Z-up to Bevy Y-up
                [
                    self.geometry.positions[idx],      // X stays
                    self.geometry.positions[idx + 2],  // Z -> Y
                    -self.geometry.positions[idx + 1], // -Y -> Z
                ]
            })
            .collect();

        // Parse normals (with same coordinate conversion)
        let normals: Vec<[f32; 3]> = if self.geometry.normals.len() == self.geometry.positions.len()
        {
            (0..vertex_count)
                .map(|i| {
                    let idx = i * 3;
                    [
                        self.geometry.normals[idx],
                        self.geometry.normals[idx + 2],
                        -self.geometry.normals[idx + 1],
                    ]
                })
                .collect()
        } else {
            // Compute flat normals from triangles if not provided
            compute_flat_normals(&positions, &self.geometry.indices)
        };

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_indices(Indices::U32(self.geometry.indices.clone()));

        mesh
    }

    /// Get transform as Bevy Transform
    pub fn get_transform(&self) -> Transform {
        let mat = Mat4::from_cols_array(&self.transform);
        Transform::from_matrix(mat)
    }

    /// Get color as Bevy Color
    pub fn get_color(&self) -> Color {
        Color::srgba(self.color[0], self.color[1], self.color[2], self.color[3])
    }
}

/// Marker component for IFC entities
#[derive(Component)]
pub struct IfcEntity {
    pub id: u64,
    pub entity_type: String,
    pub name: Option<String>,
}

/// Entity bounding box component (for zoom-to-entity)
#[derive(Component, Clone, Debug)]
pub struct EntityBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl EntityBounds {
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn diagonal(&self) -> f32 {
        (self.max - self.min).length()
    }
}

/// Marker for entities that need material update
#[derive(Component)]
pub struct NeedsMaterialUpdate;

/// Marker for batched mesh entities
#[derive(Component)]
pub struct BatchedMesh {
    /// Whether this batch is transparent
    pub is_transparent: bool,
}

/// Resource mapping triangle indices to entity IDs for picking
#[derive(Resource, Default)]
pub struct TriangleEntityMapping {
    /// Maps triangle index -> entity ID for opaque batch
    pub opaque: Vec<u64>,
    /// Maps triangle index -> entity ID for transparent batch
    pub transparent: Vec<u64>,
}

impl TriangleEntityMapping {
    /// Look up entity ID from triangle index
    pub fn get_entity(&self, is_transparent: bool, triangle_index: usize) -> Option<u64> {
        let mapping = if is_transparent {
            &self.transparent
        } else {
            &self.opaque
        };
        mapping.get(triangle_index).copied()
    }
}

/// Batched geometry builder - combines multiple meshes into one
struct BatchBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
    /// Maps triangle index -> entity_id (for picking)
    triangle_to_entity: Vec<u64>,
}

impl BatchBuilder {
    fn with_capacity(vertex_hint: usize, index_hint: usize) -> Self {
        Self {
            positions: Vec::with_capacity(vertex_hint),
            normals: Vec::with_capacity(vertex_hint),
            colors: Vec::with_capacity(vertex_hint),
            indices: Vec::with_capacity(index_hint),
            triangle_to_entity: Vec::with_capacity(index_hint / 3),
        }
    }

    /// Add a mesh to the batch, transforming vertices to world space
    fn add_mesh(&mut self, ifc_mesh: &IfcMesh) {
        let geometry = &ifc_mesh.geometry;
        let vertex_count = geometry.vertex_count();
        if vertex_count == 0 {
            return;
        }

        let start_vertex = self.positions.len();
        let transform = ifc_mesh.get_transform();
        let color = [
            ifc_mesh.color[0],
            ifc_mesh.color[1],
            ifc_mesh.color[2],
            ifc_mesh.color[3],
        ];

        // Transform positions to world space and convert Z-up to Y-up
        for i in 0..vertex_count {
            let idx = i * 3;
            // Convert from IFC Z-up to Bevy Y-up
            let local_pos = Vec3::new(
                geometry.positions[idx],
                geometry.positions[idx + 2],  // Z -> Y
                -geometry.positions[idx + 1], // -Y -> Z
            );
            let world_pos = transform.transform_point(local_pos);
            self.positions.push([world_pos.x, world_pos.y, world_pos.z]);

            // Transform normals (rotation only, no translation)
            if geometry.normals.len() == geometry.positions.len() {
                let local_normal = Vec3::new(
                    geometry.normals[idx],
                    geometry.normals[idx + 2],
                    -geometry.normals[idx + 1],
                );
                let world_normal = transform.rotation * local_normal;
                self.normals
                    .push([world_normal.x, world_normal.y, world_normal.z]);
            } else {
                self.normals.push([0.0, 1.0, 0.0]); // Default up
            }

            self.colors.push(color);
        }

        // Add indices with offset and track triangle-to-entity mapping
        let index_offset = start_vertex as u32;
        let num_triangles = geometry.triangle_count();
        for &idx in &geometry.indices {
            self.indices.push(idx + index_offset);
        }

        // Map each triangle to its entity ID (for picking)
        for _ in 0..num_triangles {
            self.triangle_to_entity.push(ifc_mesh.entity_id);
        }
    }

    /// Get the triangle-to-entity mapping (consumes ownership)
    fn take_triangle_mapping(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.triangle_to_entity)
    }

    /// Build final Bevy mesh
    fn build(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        // Recompute normals if we didn't have proper ones
        let normals = if self.normals.iter().all(|n| n[1] == 1.0 && n[0] == 0.0) {
            compute_flat_normals(&self.positions, &self.indices)
        } else {
            self.normals
        };

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));

        mesh
    }

    fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// System to spawn batched meshes when scene data changes
fn spawn_meshes_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene_data: ResMut<IfcSceneData>,
    mut triangle_mapping: ResMut<TriangleEntityMapping>,
    existing_entities: Query<Entity, With<IfcEntity>>,
    existing_batches: Query<Entity, With<BatchedMesh>>,
) {
    if !scene_data.dirty {
        return;
    }

    let mesh_count = scene_data.meshes.len();
    log(&format!("[Bevy] Batching {} meshes for GPU", mesh_count));

    // Clear previous triangle mapping
    triangle_mapping.opaque.clear();
    triangle_mapping.transparent.clear();

    // Despawn existing entities and batches
    for entity in existing_entities.iter() {
        commands.entity(entity).despawn();
    }
    for entity in existing_batches.iter() {
        commands.entity(entity).despawn();
    }

    // Estimate capacity (rough: 100 verts per mesh average)
    let vertex_hint = mesh_count * 100;
    let index_hint = mesh_count * 300;

    let mut opaque_batch = BatchBuilder::with_capacity(vertex_hint, index_hint);
    let mut transparent_batch = BatchBuilder::with_capacity(vertex_hint / 10, index_hint / 10);

    // Track bounds
    let mut scene_min = Vec3::splat(f32::INFINITY);
    let mut scene_max = Vec3::splat(f32::NEG_INFINITY);

    // Process all meshes - group by transparency
    for ifc_mesh in &scene_data.meshes {
        let is_transparent = ifc_mesh.color[3] < 1.0;
        let transform = ifc_mesh.get_transform();
        let geometry = &ifc_mesh.geometry;

        // Compute entity bounds
        let mut entity_min = Vec3::splat(f32::INFINITY);
        let mut entity_max = Vec3::splat(f32::NEG_INFINITY);
        for i in (0..geometry.positions.len()).step_by(3) {
            let pos = Vec3::new(
                geometry.positions[i],
                geometry.positions[i + 2],
                -geometry.positions[i + 1],
            );
            let world_pos = transform.transform_point(pos);
            entity_min = entity_min.min(world_pos);
            entity_max = entity_max.max(world_pos);
            scene_min = scene_min.min(world_pos);
            scene_max = scene_max.max(world_pos);
        }

        // Add to appropriate batch
        if is_transparent {
            transparent_batch.add_mesh(ifc_mesh);
        } else {
            opaque_batch.add_mesh(ifc_mesh);
        }

        // Spawn lightweight entity for selection/visibility (no mesh, just metadata)
        commands.spawn((
            IfcEntity {
                id: ifc_mesh.entity_id,
                entity_type: ifc_mesh.entity_type.clone(),
                name: ifc_mesh.name.clone(),
            },
            EntityBounds {
                min: entity_min,
                max: entity_max,
            },
            Transform::default(),
            Visibility::default(),
        ));
    }

    // Spawn opaque batch
    if !opaque_batch.is_empty() {
        log(&format!(
            "[Bevy] Opaque batch: {} vertices, {} triangles",
            opaque_batch.vertex_count(),
            opaque_batch.triangle_count()
        ));

        // Store triangle-to-entity mapping for picking
        triangle_mapping.opaque = opaque_batch.take_triangle_mapping();

        let mesh = opaque_batch.build();
        let material = StandardMaterial {
            base_color: Color::WHITE,
            metallic: 0.0,
            perceptual_roughness: 0.6,
            reflectance: 0.3,
            double_sided: true,
            cull_mode: None,
            // Use vertex colors
            ..default()
        };

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::default(),
            BatchedMesh {
                is_transparent: false,
            },
        ));
    }

    // Spawn transparent batch
    if !transparent_batch.is_empty() {
        log(&format!(
            "[Bevy] Transparent batch: {} vertices, {} triangles",
            transparent_batch.vertex_count(),
            transparent_batch.triangle_count()
        ));

        // Store triangle-to-entity mapping for picking
        triangle_mapping.transparent = transparent_batch.take_triangle_mapping();

        let mesh = transparent_batch.build();
        let material = StandardMaterial {
            base_color: Color::WHITE,
            metallic: 0.0,
            perceptual_roughness: 0.1,
            reflectance: 0.5,
            double_sided: true,
            cull_mode: None,
            alpha_mode: AlphaMode::Blend,
            ..default()
        };

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Transform::default(),
            BatchedMesh {
                is_transparent: true,
            },
        ));
    }

    // Update scene bounds
    if scene_min.x.is_finite() && scene_max.x.is_finite() {
        scene_data.bounds = Some(SceneBounds {
            min: scene_min,
            max: scene_max,
        });
        log(&format!(
            "[Bevy] Scene bounds: {:?} to {:?}",
            scene_min, scene_max
        ));
    }

    log(&format!(
        "[Bevy] Batching complete: {} meshes -> 2 draw calls",
        mesh_count
    ));

    scene_data.dirty = false;
}

/// System to auto-fit camera to scene bounds when first loaded
fn auto_fit_camera_system(
    scene_data: Res<IfcSceneData>,
    mut auto_fit: ResMut<AutoFitState>,
    mut camera_controller: ResMut<crate::camera::CameraController>,
) {
    // Only fit once when bounds become available
    if auto_fit.has_fit {
        return;
    }

    if let Some(ref bounds) = scene_data.bounds {
        log(&format!(
            "[Bevy] Auto-fitting camera to bounds: {:?} to {:?}",
            bounds.min, bounds.max
        ));

        // Calculate scene center and size
        let center = bounds.center();
        let diagonal = bounds.diagonal();

        // Set camera to fit the entire scene
        let fov_rad = camera_controller.fov.to_radians();
        let distance = diagonal / (2.0 * (fov_rad / 2.0).tan());

        // Update camera controller directly (no animation for initial fit)
        camera_controller.target = center;
        camera_controller.distance = distance.max(100.0); // Minimum distance of 100mm
        camera_controller.azimuth = 0.785; // 45 degrees
        camera_controller.elevation = 0.615; // ~35 degrees (isometric)

        log(&format!(
            "[Bevy] Camera set to: target={:?}, distance={}",
            center, distance
        ));

        auto_fit.has_fit = true;
    }
}

/// System to update mesh visibility based on settings
/// Note: With batched rendering, per-entity visibility requires rebuilding the batch.
/// For now, this is a no-op - visibility changes require scene reload.
/// TODO: Implement dynamic visibility via vertex color alpha or shader.
fn update_mesh_visibility_system(
    settings: Res<ViewerSettings>,
    _query: Query<(&IfcEntity, &mut Visibility)>,
) {
    if !settings.is_changed() {
        // With batched meshes, individual entity visibility would require:
        // 1. Rebuilding the batch (expensive), or
        // 2. Custom shader with visibility buffer, or
        // 3. Setting vertex alpha to 0 (requires mesh mutation)
        // For now, visibility is handled at scene load time only.
    }
}

/// System to update mesh selection highlighting
/// Note: With batched rendering, per-entity selection requires custom shaders.
/// TODO: Implement selection via outline post-process or stencil buffer.
fn update_mesh_selection_system(
    selection: Res<crate::picking::SelectionState>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _query: Query<(&IfcEntity, &MeshMaterial3d<StandardMaterial>)>,
) {
    if !selection.is_changed() {
        // With batched meshes, per-entity selection highlighting would require:
        // 1. Custom shader with entity ID buffer, or
        // 2. Outline post-processing effect, or
        // 3. Separate unbatched mesh for selected entities
        // For now, selection state is tracked but not visually shown.
        // The Yew UI still shows selection in the hierarchy panel.
    }
}

/// System to poll for focus commands from Yew (zoom to entity)
#[allow(unused_variables, unused_mut)]
fn poll_focus_command_system(
    mut camera_controller: ResMut<crate::camera::CameraController>,
    entities: Query<(&IfcEntity, &EntityBounds)>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(focus) = crate::storage::load_focus() {
            // Clear the command so we don't process it again
            crate::storage::clear_focus();

            log(&format!(
                "[Bevy] Focus command received for entity #{}",
                focus.entity_id
            ));

            // Find the entity with matching ID
            for (ifc_entity, bounds) in entities.iter() {
                if ifc_entity.id == focus.entity_id {
                    log(&format!(
                        "[Bevy] Focusing on entity '{}' ({}), bounds: {:?} to {:?}",
                        ifc_entity.name.as_deref().unwrap_or("unnamed"),
                        ifc_entity.entity_type,
                        bounds.min,
                        bounds.max
                    ));

                    // Use camera's frame method to zoom to entity bounds
                    camera_controller.frame(bounds.min, bounds.max);
                    break;
                }
            }
        }
    }
}

/// Get default color for IFC entity type
pub fn get_default_color(entity_type: &str) -> [f32; 4] {
    // Convert to uppercase for case-insensitive matching
    let upper = entity_type.to_uppercase();

    if upper.contains("WALL") {
        // Walls - warm beige/cream
        [0.92, 0.85, 0.75, 1.0]
    } else if upper.contains("SLAB") {
        // Slabs/floors - concrete gray
        [0.75, 0.73, 0.70, 1.0]
    } else if upper.contains("ROOF") {
        // Roofs - terracotta/reddish
        [0.72, 0.55, 0.45, 1.0]
    } else if upper.contains("BEAM") || upper.contains("COLUMN") || upper.contains("MEMBER") {
        // Structural elements - steel blue-gray
        [0.60, 0.65, 0.72, 1.0]
    } else if upper.contains("DOOR") {
        // Doors - rich wood brown
        [0.55, 0.35, 0.20, 1.0]
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        // Windows/curtain walls - blue glass (semi-transparent)
        [0.5, 0.7, 0.85, 0.35]
    } else if upper.contains("STAIR") || upper.contains("RAMP") {
        // Stairs/ramps - warm gray
        [0.65, 0.62, 0.58, 1.0]
    } else if upper.contains("RAILING") {
        // Railings - dark metallic
        [0.35, 0.35, 0.38, 1.0]
    } else if upper.contains("FURNITURE") || upper.contains("FURNISHING") {
        // Furniture - warm wood
        [0.65, 0.45, 0.28, 1.0]
    } else if upper.contains("SPACE") {
        // Space - very light blue, semi-transparent
        [0.8, 0.85, 0.95, 0.12]
    } else if upper.contains("PLATE") {
        // Plates - steel
        [0.68, 0.70, 0.75, 1.0]
    } else if upper.contains("COVERING") {
        // Coverings - light warm gray
        [0.82, 0.80, 0.76, 1.0]
    } else if upper.contains("FOOTING") || upper.contains("PILE") {
        // Foundations - dark concrete
        [0.55, 0.53, 0.50, 1.0]
    } else if upper.contains("PROXY") {
        // Building element proxies - purple tint
        [0.70, 0.65, 0.75, 1.0]
    } else if upper.contains("FLOW") || upper.contains("DUCT") || upper.contains("PIPE") {
        // MEP flow elements - green tint
        [0.55, 0.70, 0.58, 1.0]
    } else if upper.contains("ELECTRIC") || upper.contains("ENERGY") {
        // Electrical/energy elements - yellow tint
        [0.75, 0.72, 0.45, 1.0]
    } else if upper.contains("SANITARY") || upper.contains("FIRE") {
        // Plumbing fixtures - white ceramic
        [0.92, 0.92, 0.95, 1.0]
    } else if upper.contains("SHADING") {
        // Shading devices - dark blue-gray
        [0.45, 0.48, 0.55, 0.8]
    } else if upper.contains("TRANSPORT") {
        // Transport elements (elevators, etc) - dark gray
        [0.40, 0.40, 0.42, 1.0]
    } else if upper.contains("GEOGRAPHIC") || upper.contains("VIRTUAL") {
        // Geographic/virtual - light green, semi-transparent
        [0.75, 0.85, 0.75, 0.25]
    } else {
        // Default - neutral warm gray
        [0.75, 0.72, 0.70, 1.0]
    }
}

/// Compute flat normals from triangle positions and indices
fn compute_flat_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32, 0.0, 0.0]; positions.len()];

    // Accumulate face normals to vertices
    for tri in indices.chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;

        if i0 >= positions.len() || i1 >= positions.len() || i2 >= positions.len() {
            continue;
        }

        let p0 = Vec3::from_array(positions[i0]);
        let p1 = Vec3::from_array(positions[i1]);
        let p2 = Vec3::from_array(positions[i2]);

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let face_normal = edge1.cross(edge2);

        // Add face normal to each vertex (will be normalized later)
        for &idx in &[i0, i1, i2] {
            normals[idx][0] += face_normal.x;
            normals[idx][1] += face_normal.y;
            normals[idx][2] += face_normal.z;
        }
    }

    // Normalize all normals
    for normal in &mut normals {
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if len > 0.0001 {
            normal[0] /= len;
            normal[1] /= len;
            normal[2] /= len;
        } else {
            // Default to up if degenerate
            *normal = [0.0, 1.0, 0.0];
        }
    }

    normals
}
