//! Mesh system for IFC geometry
//!
//! Handles loading IFC geometry into Bevy meshes with materials.

use bevy::prelude::*;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::asset::RenderAssetUsages;
use serde::{Deserialize, Serialize};
use crate::{IfcSceneData, SceneBounds, ViewerSettings, log};

/// Mesh plugin
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoFitState>()
            .init_resource::<PendingFocus>()
            .add_systems(Update, (
                spawn_meshes_system,
                auto_fit_camera_system,
                update_mesh_visibility_system,
                update_mesh_selection_system,
                poll_focus_command_system,
            ).chain());
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

/// IFC mesh data (serializable for storage)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IfcMesh {
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

impl IfcMesh {
    /// Convert to Bevy mesh
    pub fn to_bevy_mesh(&self) -> Mesh {
        let vertex_count = self.positions.len() / 3;

        // Parse positions
        let positions: Vec<[f32; 3]> = (0..vertex_count)
            .map(|i| {
                let idx = i * 3;
                // Convert from IFC Z-up to Bevy Y-up
                [
                    self.positions[idx],      // X stays
                    self.positions[idx + 2],  // Z -> Y
                    -self.positions[idx + 1], // -Y -> Z
                ]
            })
            .collect();

        // Parse normals (with same coordinate conversion)
        let normals: Vec<[f32; 3]> = if self.normals.len() == self.positions.len() {
            (0..vertex_count)
                .map(|i| {
                    let idx = i * 3;
                    [
                        self.normals[idx],
                        self.normals[idx + 2],
                        -self.normals[idx + 1],
                    ]
                })
                .collect()
        } else {
            // Compute flat normals from triangles if not provided
            compute_flat_normals(&positions, &self.indices)
        };

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_indices(Indices::U32(self.indices.clone()));

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

/// System to spawn meshes when scene data changes
fn spawn_meshes_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene_data: ResMut<IfcSceneData>,
    existing_entities: Query<Entity, With<IfcEntity>>,
) {
    if !scene_data.dirty {
        return;
    }

    log(&format!("[Bevy] Spawning {} meshes", scene_data.meshes.len()));

    // Despawn existing entities
    for entity in existing_entities.iter() {
        commands.entity(entity).despawn();
    }

    // Track bounds
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);

    // Spawn new meshes
    for ifc_mesh in &scene_data.meshes {
        let mesh = ifc_mesh.to_bevy_mesh();
        let transform = ifc_mesh.get_transform();
        let color = ifc_mesh.get_color();

        // Compute entity bounds and update scene bounds
        let mut entity_min = Vec3::splat(f32::INFINITY);
        let mut entity_max = Vec3::splat(f32::NEG_INFINITY);
        for i in (0..ifc_mesh.positions.len()).step_by(3) {
            let pos = Vec3::new(
                ifc_mesh.positions[i],
                ifc_mesh.positions[i + 2],  // Z -> Y
                -ifc_mesh.positions[i + 1], // -Y -> Z
            );
            let world_pos = transform.transform_point(pos);
            entity_min = entity_min.min(world_pos);
            entity_max = entity_max.max(world_pos);
            min = min.min(world_pos);
            max = max.max(world_pos);
        }

        let material = StandardMaterial {
            base_color: color,
            metallic: 0.0,
            perceptual_roughness: 0.5, // Less rough for better light response
            reflectance: 0.3,
            double_sided: true,
            cull_mode: None,
            ..default()
        };

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            transform,
            IfcEntity {
                id: ifc_mesh.entity_id,
                entity_type: ifc_mesh.entity_type.clone(),
                name: ifc_mesh.name.clone(),
            },
            EntityBounds {
                min: entity_min,
                max: entity_max,
            },
        ));
    }

    // Update scene bounds
    if min.x.is_finite() && max.x.is_finite() {
        scene_data.bounds = Some(SceneBounds { min, max });
        log(&format!("[Bevy] Scene bounds: {:?} to {:?}", min, max));
    }

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
        log(&format!("[Bevy] Auto-fitting camera to bounds: {:?} to {:?}", bounds.min, bounds.max));

        // Calculate scene center and size
        let center = bounds.center();
        let diagonal = bounds.diagonal();

        // Set camera to fit the entire scene
        let fov_rad = camera_controller.fov.to_radians();
        let distance = diagonal / (2.0 * (fov_rad / 2.0).tan());

        // Update camera controller directly (no animation for initial fit)
        camera_controller.target = center;
        camera_controller.distance = distance.max(100.0); // Minimum distance of 100mm
        camera_controller.azimuth = 0.785;  // 45 degrees
        camera_controller.elevation = 0.615; // ~35 degrees (isometric)

        log(&format!("[Bevy] Camera set to: target={:?}, distance={}", center, distance));

        auto_fit.has_fit = true;
    }
}

/// System to update mesh visibility based on settings
fn update_mesh_visibility_system(
    settings: Res<ViewerSettings>,
    mut query: Query<(&IfcEntity, &mut Visibility)>,
) {
    if !settings.is_changed() {
        return;
    }

    for (ifc_entity, mut visibility) in query.iter_mut() {
        let should_hide = settings.hidden_entities.contains(&ifc_entity.id);
        let should_isolate = settings.isolated_entities.as_ref()
            .map(|isolated| !isolated.contains(&ifc_entity.id))
            .unwrap_or(false);

        *visibility = if should_hide || should_isolate {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

/// System to update mesh selection highlighting
fn update_mesh_selection_system(
    selection: Res<crate::picking::SelectionState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&IfcEntity, &MeshMaterial3d<StandardMaterial>)>,
) {
    if !selection.is_changed() {
        return;
    }

    for (ifc_entity, material_handle) in query.iter() {
        if let Some(material) = materials.get_mut(material_handle) {
            let is_selected = selection.selected.contains(&ifc_entity.id);
            let is_hovered = selection.hovered == Some(ifc_entity.id);

            if is_selected {
                // Bright selection color
                material.emissive = LinearRgba::new(0.2, 0.4, 0.8, 1.0);
            } else if is_hovered {
                // Subtle hover highlight
                material.emissive = LinearRgba::new(0.1, 0.2, 0.3, 1.0);
            } else {
                // No highlight
                material.emissive = LinearRgba::BLACK;
            }
        }
    }
}

/// System to poll for focus commands from Yew (zoom to entity)
fn poll_focus_command_system(
    mut camera_controller: ResMut<crate::camera::CameraController>,
    entities: Query<(&IfcEntity, &EntityBounds)>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(focus) = crate::storage::load_focus() {
            // Clear the command so we don't process it again
            crate::storage::clear_focus();

            log(&format!("[Bevy] Focus command received for entity #{}", focus.entity_id));

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
    match entity_type {
        // Walls - light gray
        s if s.contains("Wall") => [0.85, 0.85, 0.85, 1.0],
        // Slabs/floors - darker gray
        s if s.contains("Slab") => [0.7, 0.7, 0.7, 1.0],
        // Roofs - dark gray
        s if s.contains("Roof") => [0.5, 0.5, 0.5, 1.0],
        // Beams/columns - steel blue
        s if s.contains("Beam") || s.contains("Column") => [0.6, 0.65, 0.75, 1.0],
        // Doors - brown
        s if s.contains("Door") => [0.6, 0.4, 0.2, 1.0],
        // Windows - light blue (semi-transparent)
        s if s.contains("Window") => [0.7, 0.85, 0.95, 0.5],
        // Stairs - medium gray
        s if s.contains("Stair") => [0.65, 0.65, 0.65, 1.0],
        // Railings - dark gray
        s if s.contains("Railing") => [0.4, 0.4, 0.4, 1.0],
        // Furniture - wood color
        s if s.contains("Furniture") => [0.7, 0.55, 0.35, 1.0],
        // Space - very light, semi-transparent
        s if s.contains("Space") => [0.9, 0.9, 0.95, 0.2],
        // Default - neutral gray
        _ => [0.75, 0.75, 0.75, 1.0],
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
