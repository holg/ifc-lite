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
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
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
            .init_resource::<CurrentPalette>()
            .init_resource::<EntityColorMapping>()
            .init_resource::<PreviousSelection>()
            .add_systems(
                Update,
                (
                    poll_palette_change_system,
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

/// Current color palette state
#[cfg(feature = "color-palette")]
#[derive(Resource, Default)]
pub struct CurrentPalette {
    pub palette: ColorPalette,
}

#[cfg(not(feature = "color-palette"))]
#[derive(Resource, Default)]
pub struct CurrentPalette;

/// Lightweight entity info for palette switching and selection highlighting (no geometry data)
#[cfg(feature = "color-palette")]
#[derive(Clone, Debug)]
pub struct EntityColorInfo {
    pub entity_id: u64,
    pub entity_type: String,
    pub original_color: [f32; 4],
    pub start_vertex: u32,
    pub vertex_count: u32,
}

/// Maps entity color info for palette switching without keeping geometry
#[cfg(feature = "color-palette")]
#[derive(Resource, Default)]
pub struct EntityColorMapping {
    /// Opaque mesh entity mappings
    pub opaque: Vec<EntityColorInfo>,
    /// Transparent mesh entity mappings
    pub transparent: Vec<EntityColorInfo>,
}

#[cfg(not(feature = "color-palette"))]
#[derive(Resource, Default)]
pub struct EntityColorMapping;

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

    /// Create from ifc_lite_geometry_new::Mesh (takes ownership via conversion)
    pub fn from_geometry_mesh(mesh: ifc_lite_geometry_new::Mesh) -> Self {
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
        mesh: ifc_lite_geometry_new::Mesh,
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
    /// Lightweight entity info for palette switching (only when feature enabled)
    #[cfg(feature = "color-palette")]
    entity_color_info: Vec<EntityColorInfo>,
}

impl BatchBuilder {
    fn with_capacity(vertex_hint: usize, index_hint: usize) -> Self {
        Self {
            positions: Vec::with_capacity(vertex_hint),
            normals: Vec::with_capacity(vertex_hint),
            colors: Vec::with_capacity(vertex_hint),
            indices: Vec::with_capacity(index_hint),
            triangle_to_entity: Vec::with_capacity(index_hint / 3),
            #[cfg(feature = "color-palette")]
            entity_color_info: Vec::new(),
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

        // Track lightweight entity info for palette switching and selection (no geometry!)
        #[cfg(feature = "color-palette")]
        self.entity_color_info.push(EntityColorInfo {
            entity_id: ifc_mesh.entity_id,
            entity_type: ifc_mesh.entity_type.clone(),
            original_color: color,
            start_vertex: start_vertex as u32,
            vertex_count: vertex_count as u32,
        });
    }

    /// Get the triangle-to-entity mapping (consumes ownership)
    fn take_triangle_mapping(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.triangle_to_entity)
    }

    /// Get the entity color info (consumes ownership)
    #[cfg(feature = "color-palette")]
    fn take_color_info(&mut self) -> Vec<EntityColorInfo> {
        std::mem::take(&mut self.entity_color_info)
    }

    /// Build final Bevy mesh
    fn build(self) -> Mesh {
        // Always use MAIN_WORLD | RENDER_WORLD to allow picking to access vertex positions
        // The picking system needs to read positions for ray-mesh intersection
        let usage = RenderAssetUsages::default(); // MAIN_WORLD | RENDER_WORLD

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, usage);

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

/// Get current time in milliseconds (WASM)
#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// System to spawn batched meshes when scene data changes
fn spawn_meshes_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene_data: ResMut<IfcSceneData>,
    mut triangle_mapping: ResMut<TriangleEntityMapping>,
    mut color_mapping: ResMut<EntityColorMapping>,
    existing_entities: Query<Entity, With<IfcEntity>>,
    existing_batches: Query<Entity, With<BatchedMesh>>,
) {
    if !scene_data.dirty {
        return;
    }

    let batch_start = now_ms();
    let mesh_count = scene_data.meshes.len();
    crate::log_info(&format!("[Bevy] Batching {} meshes for GPU...", mesh_count));

    // Clear previous mappings
    triangle_mapping.opaque.clear();
    triangle_mapping.transparent.clear();
    #[cfg(feature = "color-palette")]
    {
        color_mapping.opaque.clear();
        color_mapping.transparent.clear();
    }
    let _ = &color_mapping; // Silence unused warning when feature disabled

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

        // Store mappings for picking
        triangle_mapping.opaque = opaque_batch.take_triangle_mapping();
        #[cfg(feature = "color-palette")]
        {
            color_mapping.opaque = opaque_batch.take_color_info();
        }

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

        // Store mappings for picking
        triangle_mapping.transparent = transparent_batch.take_triangle_mapping();
        #[cfg(feature = "color-palette")]
        {
            color_mapping.transparent = transparent_batch.take_color_info();
        }

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

    // Calculate totals for logging
    let total_vertices = scene_data.meshes.iter().map(|m| m.geometry.vertex_count()).sum::<usize>();
    let total_triangles = scene_data.meshes.iter().map(|m| m.geometry.triangle_count()).sum::<usize>();
    let geometry_size: usize = scene_data
        .meshes
        .iter()
        .map(|m| {
            m.geometry.positions.len() * 4
                + m.geometry.normals.len() * 4
                + m.geometry.indices.len() * 4
        })
        .sum();

    let batch_time = now_ms() - batch_start;
    crate::log_info(&format!(
        "[Bevy] ✓ GPU upload: {:.0}ms | {} vertices, {} triangles | {:.1} MB geometry",
        batch_time,
        total_vertices,
        total_triangles,
        geometry_size as f64 / (1024.0 * 1024.0)
    ));

    // FREE MEMORY: Clear heavy geometry data now that it's on GPU
    for mesh in &mut scene_data.meshes {
        mesh.geometry = Arc::new(MeshGeometry::default());
    }

    log(&format!(
        "[Bevy] Freed {}MB of geometry data from IfcSceneData",
        geometry_size / (1024 * 1024)
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

/// Selection highlight color (light blue / hellblau)
#[cfg(feature = "color-palette")]
const SELECTION_COLOR: [f32; 4] = [0.3, 0.7, 1.0, 1.0];

/// Resource to track previous selection for efficient updates
#[derive(Resource, Default)]
pub struct PreviousSelection {
    #[cfg(feature = "color-palette")]
    pub selected_ids: rustc_hash::FxHashSet<u64>,
}

/// System to update mesh selection highlighting via vertex colors
#[cfg(feature = "color-palette")]
fn update_mesh_selection_system(
    selection: Res<crate::picking::SelectionState>,
    mut previous_selection: ResMut<PreviousSelection>,
    color_mapping: Res<EntityColorMapping>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    batched_meshes: Query<(&Mesh3d, &BatchedMesh)>,
) {
    if !selection.is_changed() {
        return;
    }

    let current_selection = &selection.selected;

    // Find entities that changed selection state
    let newly_selected: Vec<u64> = current_selection
        .difference(&previous_selection.selected_ids)
        .copied()
        .collect();
    let newly_deselected: Vec<u64> = previous_selection.selected_ids
        .difference(current_selection)
        .copied()
        .collect();

    if newly_selected.is_empty() && newly_deselected.is_empty() {
        return;
    }

    // Update vertex colors in batched meshes
    for (mesh_handle, batched_mesh) in batched_meshes.iter() {
        let Some(mesh) = mesh_assets.get_mut(&mesh_handle.0) else {
            continue;
        };

        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) else {
            continue;
        };

        let color_infos = if batched_mesh.is_transparent {
            &color_mapping.transparent
        } else {
            &color_mapping.opaque
        };

        // Apply highlight color to newly selected
        for &entity_id in &newly_selected {
            for info in color_infos.iter().filter(|i| i.entity_id == entity_id) {
                let start = info.start_vertex as usize;
                let end = start + info.vertex_count as usize;
                for color in colors[start..end].iter_mut() {
                    *color = SELECTION_COLOR;
                }
            }
        }

        // Restore original color for newly deselected
        for &entity_id in &newly_deselected {
            for info in color_infos.iter().filter(|i| i.entity_id == entity_id) {
                let start = info.start_vertex as usize;
                let end = start + info.vertex_count as usize;
                for color in colors[start..end].iter_mut() {
                    *color = info.original_color;
                }
            }
        }
    }

    // Update previous selection state
    previous_selection.selected_ids = current_selection.clone();
}

#[cfg(not(feature = "color-palette"))]
fn update_mesh_selection_system(
    _selection: Res<crate::picking::SelectionState>,
) {
    // Selection highlighting requires color-palette feature for vertex color updates
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

/// System to poll for palette change commands from Yew
#[cfg(feature = "color-palette")]
#[allow(unused_variables, unused_mut)]
fn poll_palette_change_system(
    mut current_palette: ResMut<CurrentPalette>,
    color_mapping: Res<EntityColorMapping>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    batched_meshes: Query<(&Mesh3d, &BatchedMesh)>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(palette_str) = crate::storage::load_palette() {
            // Clear the command so we don't process it again
            crate::storage::clear_palette();

            // Parse palette string
            let new_palette = match palette_str.as_str() {
                "vibrant" => ColorPalette::Vibrant,
                "realistic" => ColorPalette::Realistic,
                "high_contrast" => ColorPalette::HighContrast,
                "monochrome" => ColorPalette::Monochrome,
                _ => {
                    log(&format!("[Bevy] Unknown palette: {}", palette_str));
                    return;
                }
            };

            // Only update if palette changed
            if current_palette.palette != new_palette {
                log(&format!(
                    "[Bevy] Palette changed to {:?}, updating vertex colors in-place",
                    new_palette
                ));

                current_palette.palette = new_palette;

                // Update vertex colors directly in GPU meshes (no geometry rebuild!)
                for (mesh_handle, batched) in batched_meshes.iter() {
                    let mapping = if batched.is_transparent {
                        &color_mapping.transparent
                    } else {
                        &color_mapping.opaque
                    };

                    if let Some(mesh) = mesh_assets.get_mut(&mesh_handle.0) {
                        // Get mutable access to vertex colors
                        if let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
                            mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
                        {
                            // Update colors based on entity mapping
                            for info in mapping {
                                let new_color = get_color_for_palette(&info.entity_type, new_palette);
                                let start = info.start_vertex as usize;
                                let end = start + info.vertex_count as usize;
                                for i in start..end.min(colors.len()) {
                                    colors[i] = new_color;
                                }
                            }
                            log(&format!(
                                "[Bevy] Updated {} entity colors in {} batch",
                                mapping.len(),
                                if batched.is_transparent { "transparent" } else { "opaque" }
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// No-op system when color-palette feature is disabled
#[cfg(not(feature = "color-palette"))]
fn poll_palette_change_system() {
    // Color palette switching disabled - no entity color info stored
}

/// Color palette for IFC visualization
#[cfg(feature = "color-palette")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorPalette {
    /// Vibrant - saturated, vivid colors (default)
    #[default]
    Vibrant,
    /// Realistic - muted architectural colors
    Realistic,
    /// High Contrast - bold colors for visibility
    HighContrast,
    /// Monochrome - grayscale for technical views
    Monochrome,
}

#[cfg(feature = "color-palette")]
impl ColorPalette {
    /// Get all available palettes
    pub fn all() -> &'static [ColorPalette] {
        &[
            ColorPalette::Vibrant,
            ColorPalette::Realistic,
            ColorPalette::HighContrast,
            ColorPalette::Monochrome,
        ]
    }

    /// Get palette name for display
    pub fn name(&self) -> &'static str {
        match self {
            ColorPalette::Vibrant => "Vibrant",
            ColorPalette::Realistic => "Realistic",
            ColorPalette::HighContrast => "High Contrast",
            ColorPalette::Monochrome => "Monochrome",
        }
    }
}

/// Get color for IFC entity type using the specified palette
#[cfg(feature = "color-palette")]
pub fn get_color_for_palette(entity_type: &str, palette: ColorPalette) -> [f32; 4] {
    match palette {
        ColorPalette::Vibrant => get_vibrant_color(entity_type),
        ColorPalette::Realistic => get_realistic_color(entity_type),
        ColorPalette::HighContrast => get_high_contrast_color(entity_type),
        ColorPalette::Monochrome => get_monochrome_color(entity_type),
    }
}

/// Get default color for IFC entity type (uses Vibrant palette)
pub fn get_default_color(entity_type: &str) -> [f32; 4] {
    get_vibrant_color(entity_type)
}

/// Vibrant color palette - saturated, vivid colors
fn get_vibrant_color(entity_type: &str) -> [f32; 4] {
    let upper = entity_type.to_uppercase();

    if upper.contains("WALL") {
        [0.95, 0.90, 0.80, 1.0] // Warm cream
    } else if upper.contains("SLAB") {
        [0.85, 0.82, 0.78, 1.0] // Light concrete
    } else if upper.contains("ROOF") {
        [0.85, 0.45, 0.35, 1.0] // Terracotta red
    } else if upper.contains("BEAM") || upper.contains("COLUMN") || upper.contains("MEMBER") {
        [0.45, 0.55, 0.75, 1.0] // Steel blue
    } else if upper.contains("DOOR") {
        [0.65, 0.40, 0.25, 1.0] // Rich wood brown
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        [0.4, 0.7, 0.9, 0.4] // Sky blue glass
    } else if upper.contains("STAIR") || upper.contains("RAMP") {
        [0.75, 0.70, 0.65, 1.0] // Warm stone
    } else if upper.contains("RAILING") {
        [0.30, 0.30, 0.35, 1.0] // Dark metal
    } else if upper.contains("FURNITURE") || upper.contains("FURNISHING") {
        [0.70, 0.50, 0.30, 1.0] // Warm wood
    } else if upper.contains("SPACE") {
        [0.7, 0.85, 0.95, 0.15] // Light blue space
    } else if upper.contains("PLATE") {
        [0.70, 0.72, 0.78, 1.0] // Steel plate
    } else if upper.contains("COVERING") {
        [0.88, 0.85, 0.80, 1.0] // Light finish
    } else if upper.contains("FOOTING") || upper.contains("PILE") {
        [0.60, 0.58, 0.55, 1.0] // Dark concrete
    } else if upper.contains("PROXY") {
        [0.75, 0.60, 0.80, 1.0] // Purple accent
    } else if upper.contains("FLOW") || upper.contains("DUCT") || upper.contains("PIPE") {
        [0.40, 0.75, 0.50, 1.0] // Green MEP
    } else if upper.contains("ELECTRIC") || upper.contains("ENERGY") {
        [0.90, 0.80, 0.30, 1.0] // Yellow electrical
    } else if upper.contains("SANITARY") || upper.contains("FIRE") {
        [0.95, 0.95, 0.98, 1.0] // White ceramic
    } else if upper.contains("SHADING") {
        [0.40, 0.45, 0.55, 0.85] // Blue-gray shade
    } else if upper.contains("TRANSPORT") {
        [0.45, 0.45, 0.50, 1.0] // Dark gray
    } else if upper.contains("GEOGRAPHIC") || upper.contains("VIRTUAL") {
        [0.65, 0.85, 0.65, 0.3] // Light green
    } else {
        [0.80, 0.78, 0.75, 1.0] // Neutral gray
    }
}

/// Realistic color palette - muted architectural colors
#[cfg(feature = "color-palette")]
fn get_realistic_color(entity_type: &str) -> [f32; 4] {
    let upper = entity_type.to_uppercase();

    if upper.contains("WALL") {
        [0.92, 0.85, 0.75, 1.0] // Warm beige
    } else if upper.contains("SLAB") {
        [0.75, 0.73, 0.70, 1.0] // Concrete gray
    } else if upper.contains("ROOF") {
        [0.72, 0.55, 0.45, 1.0] // Terracotta
    } else if upper.contains("BEAM") || upper.contains("COLUMN") || upper.contains("MEMBER") {
        [0.60, 0.65, 0.72, 1.0] // Steel blue-gray
    } else if upper.contains("DOOR") {
        [0.55, 0.35, 0.20, 1.0] // Wood brown
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        [0.5, 0.7, 0.85, 0.35] // Blue glass
    } else if upper.contains("STAIR") || upper.contains("RAMP") {
        [0.65, 0.62, 0.58, 1.0] // Warm gray
    } else if upper.contains("RAILING") {
        [0.35, 0.35, 0.38, 1.0] // Dark metallic
    } else if upper.contains("FURNITURE") || upper.contains("FURNISHING") {
        [0.65, 0.45, 0.28, 1.0] // Warm wood
    } else if upper.contains("SPACE") {
        [0.8, 0.85, 0.95, 0.12] // Light blue
    } else if upper.contains("PLATE") {
        [0.68, 0.70, 0.75, 1.0] // Steel
    } else if upper.contains("COVERING") {
        [0.82, 0.80, 0.76, 1.0] // Light warm gray
    } else if upper.contains("FOOTING") || upper.contains("PILE") {
        [0.55, 0.53, 0.50, 1.0] // Dark concrete
    } else if upper.contains("PROXY") {
        [0.70, 0.65, 0.75, 1.0] // Purple tint
    } else if upper.contains("FLOW") || upper.contains("DUCT") || upper.contains("PIPE") {
        [0.55, 0.70, 0.58, 1.0] // Green tint
    } else if upper.contains("ELECTRIC") || upper.contains("ENERGY") {
        [0.75, 0.72, 0.45, 1.0] // Yellow tint
    } else if upper.contains("SANITARY") || upper.contains("FIRE") {
        [0.92, 0.92, 0.95, 1.0] // White ceramic
    } else if upper.contains("SHADING") {
        [0.45, 0.48, 0.55, 0.8] // Dark blue-gray
    } else if upper.contains("TRANSPORT") {
        [0.40, 0.40, 0.42, 1.0] // Dark gray
    } else if upper.contains("GEOGRAPHIC") || upper.contains("VIRTUAL") {
        [0.75, 0.85, 0.75, 0.25] // Light green
    } else {
        [0.75, 0.72, 0.70, 1.0] // Neutral warm gray
    }
}

/// High contrast color palette - bold colors for visibility
#[cfg(feature = "color-palette")]
fn get_high_contrast_color(entity_type: &str) -> [f32; 4] {
    let upper = entity_type.to_uppercase();

    if upper.contains("WALL") {
        [1.0, 0.95, 0.85, 1.0] // Bright cream
    } else if upper.contains("SLAB") {
        [0.7, 0.7, 0.7, 1.0] // Medium gray
    } else if upper.contains("ROOF") {
        [0.9, 0.3, 0.2, 1.0] // Bright red
    } else if upper.contains("BEAM") || upper.contains("COLUMN") || upper.contains("MEMBER") {
        [0.2, 0.4, 0.8, 1.0] // Bright blue
    } else if upper.contains("DOOR") {
        [0.6, 0.3, 0.1, 1.0] // Dark brown
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        [0.3, 0.7, 1.0, 0.5] // Bright cyan glass
    } else if upper.contains("STAIR") || upper.contains("RAMP") {
        [0.9, 0.7, 0.5, 1.0] // Orange-tan
    } else if upper.contains("RAILING") {
        [0.2, 0.2, 0.2, 1.0] // Near black
    } else if upper.contains("FURNITURE") || upper.contains("FURNISHING") {
        [0.8, 0.5, 0.2, 1.0] // Bright orange
    } else if upper.contains("SPACE") {
        [0.6, 0.8, 1.0, 0.2] // Light cyan
    } else if upper.contains("PLATE") {
        [0.5, 0.5, 0.6, 1.0] // Blue-gray
    } else if upper.contains("COVERING") {
        [0.95, 0.9, 0.85, 1.0] // Off-white
    } else if upper.contains("FOOTING") || upper.contains("PILE") {
        [0.4, 0.4, 0.35, 1.0] // Dark brown-gray
    } else if upper.contains("PROXY") {
        [0.8, 0.4, 0.9, 1.0] // Bright purple
    } else if upper.contains("FLOW") || upper.contains("DUCT") || upper.contains("PIPE") {
        [0.2, 0.9, 0.4, 1.0] // Bright green
    } else if upper.contains("ELECTRIC") || upper.contains("ENERGY") {
        [1.0, 0.9, 0.2, 1.0] // Bright yellow
    } else if upper.contains("SANITARY") || upper.contains("FIRE") {
        [1.0, 1.0, 1.0, 1.0] // Pure white
    } else if upper.contains("SHADING") {
        [0.3, 0.35, 0.5, 0.9] // Dark blue
    } else if upper.contains("TRANSPORT") {
        [0.3, 0.3, 0.3, 1.0] // Dark gray
    } else if upper.contains("GEOGRAPHIC") || upper.contains("VIRTUAL") {
        [0.5, 1.0, 0.5, 0.35] // Bright green
    } else {
        [0.85, 0.85, 0.85, 1.0] // Light gray
    }
}

/// Monochrome color palette - grayscale for technical views
#[cfg(feature = "color-palette")]
fn get_monochrome_color(entity_type: &str) -> [f32; 4] {
    let upper = entity_type.to_uppercase();

    // Use different gray levels based on element type for visual hierarchy
    if upper.contains("WALL") {
        [0.85, 0.85, 0.85, 1.0] // Light gray
    } else if upper.contains("SLAB") {
        [0.70, 0.70, 0.70, 1.0] // Medium gray
    } else if upper.contains("ROOF") {
        [0.60, 0.60, 0.60, 1.0] // Medium-dark gray
    } else if upper.contains("BEAM") || upper.contains("COLUMN") || upper.contains("MEMBER") {
        [0.50, 0.50, 0.50, 1.0] // Mid gray
    } else if upper.contains("DOOR") {
        [0.40, 0.40, 0.40, 1.0] // Dark gray
    } else if upper.contains("WINDOW") || upper.contains("CURTAINWALL") {
        [0.75, 0.75, 0.75, 0.4] // Transparent light gray
    } else if upper.contains("STAIR") || upper.contains("RAMP") {
        [0.65, 0.65, 0.65, 1.0] // Medium-light gray
    } else if upper.contains("RAILING") {
        [0.30, 0.30, 0.30, 1.0] // Very dark gray
    } else if upper.contains("FURNITURE") || upper.contains("FURNISHING") {
        [0.55, 0.55, 0.55, 1.0] // Medium gray
    } else if upper.contains("SPACE") {
        [0.90, 0.90, 0.90, 0.15] // Very light gray, transparent
    } else if upper.contains("PLATE") {
        [0.60, 0.60, 0.60, 1.0] // Medium-dark gray
    } else if upper.contains("COVERING") {
        [0.80, 0.80, 0.80, 1.0] // Light gray
    } else if upper.contains("FOOTING") || upper.contains("PILE") {
        [0.45, 0.45, 0.45, 1.0] // Dark gray
    } else if upper.contains("PROXY") {
        [0.70, 0.70, 0.70, 1.0] // Medium gray
    } else if upper.contains("FLOW") || upper.contains("DUCT") || upper.contains("PIPE") {
        [0.55, 0.55, 0.55, 1.0] // Medium gray
    } else if upper.contains("ELECTRIC") || upper.contains("ENERGY") {
        [0.65, 0.65, 0.65, 1.0] // Medium-light gray
    } else if upper.contains("SANITARY") || upper.contains("FIRE") {
        [0.95, 0.95, 0.95, 1.0] // Near white
    } else if upper.contains("SHADING") {
        [0.35, 0.35, 0.35, 0.85] // Dark gray, slightly transparent
    } else if upper.contains("TRANSPORT") {
        [0.40, 0.40, 0.40, 1.0] // Dark gray
    } else if upper.contains("GEOGRAPHIC") || upper.contains("VIRTUAL") {
        [0.85, 0.85, 0.85, 0.25] // Light gray, transparent
    } else {
        [0.75, 0.75, 0.75, 1.0] // Default gray
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
