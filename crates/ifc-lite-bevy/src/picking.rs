//! Picking and selection system
//!
//! Handles raycasting for object selection and hover detection.

use crate::camera::MainCamera;
use crate::mesh::{BatchedMesh, TriangleEntityMapping};
use crate::storage::{save_selection, SelectionStorage};
use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rustc_hash::FxHashSet;

/// Picking plugin
pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionState>()
            .init_resource::<PickingSettings>()
            // Run picking after camera input so we can see just_clicked flag
            .add_systems(
                Update,
                (picking_system, hover_system)
                    .after(crate::camera::CameraPlugin::input_system_set()),
            );
    }
}

/// Current selection state
#[derive(Resource, Default)]
pub struct SelectionState {
    /// Currently selected entity IDs
    pub selected: FxHashSet<u64>,
    /// Currently hovered entity ID
    pub hovered: Option<u64>,
}

impl SelectionState {
    /// Check if entity is selected
    pub fn is_selected(&self, id: u64) -> bool {
        self.selected.contains(&id)
    }

    /// Select single entity (clears previous selection)
    pub fn select(&mut self, id: u64) {
        self.selected.clear();
        self.selected.insert(id);
        self.save();
    }

    /// Toggle selection for entity
    pub fn toggle(&mut self, id: u64) {
        if self.selected.contains(&id) {
            self.selected.remove(&id);
        } else {
            self.selected.insert(id);
        }
        self.save();
    }

    /// Add to selection
    pub fn add(&mut self, id: u64) {
        self.selected.insert(id);
        self.save();
    }

    /// Remove from selection
    pub fn remove(&mut self, id: u64) {
        self.selected.remove(&id);
        self.save();
    }

    /// Clear all selection
    pub fn clear(&mut self) {
        self.selected.clear();
        self.save();
    }

    /// Save to localStorage
    fn save(&self) {
        let storage = SelectionStorage {
            selected_ids: self.selected.iter().copied().collect(),
            hovered_id: self.hovered,
        };
        save_selection(&storage);
    }
}

/// Picking settings
#[derive(Resource)]
pub struct PickingSettings {
    /// Whether picking is enabled
    pub enabled: bool,
    /// Hover detection throttle (frames)
    pub hover_throttle: u32,
}

impl Default for PickingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            hover_throttle: 3, // Check every 3 frames
        }
    }
}

/// Picking system - handles click selection on batched meshes
#[allow(clippy::too_many_arguments)]
fn picking_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    batched_meshes: Query<(&BatchedMesh, &GlobalTransform, &Mesh3d)>,
    triangle_mapping: Res<TriangleEntityMapping>,
    meshes: Res<Assets<Mesh>>,
    mut selection: ResMut<SelectionState>,
    settings: Res<PickingSettings>,
    mut camera_controller: ResMut<crate::camera::CameraController>,
) {
    if !settings.enabled {
        return;
    }

    // Use camera controller's click detection (click = press+release without drag)
    if !camera_controller.just_clicked {
        return;
    }

    // Reset the flag so we only process once
    camera_controller.just_clicked = false;

    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };

    // Use the position where the click started
    let click_pos = camera_controller.drag_start_pos;

    // Create ray from camera through click position
    let Ok(ray) = camera.viewport_to_world(camera_transform, click_pos) else {
        return;
    };

    // Find closest intersection in batched meshes
    let mut closest: Option<(u64, f32)> = None;

    for (batched_mesh, transform, mesh_handle) in batched_meshes.iter() {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            if let Some((distance, triangle_index)) =
                ray_mesh_intersection_with_triangle(&ray, mesh, transform)
            {
                // Look up which entity this triangle belongs to
                if let Some(entity_id) =
                    triangle_mapping.get_entity(batched_mesh.is_transparent, triangle_index)
                {
                    if closest.map(|(_, d)| distance < d).unwrap_or(true) {
                        closest = Some((entity_id, distance));
                    }
                }
            }
        }
    }

    // Update selection based on result
    if let Some((entity_id, _)) = closest {
        let ctrl_pressed = keyboard.pressed(KeyCode::ControlLeft)
            || keyboard.pressed(KeyCode::ControlRight)
            || keyboard.pressed(KeyCode::SuperLeft)
            || keyboard.pressed(KeyCode::SuperRight);

        if ctrl_pressed {
            selection.toggle(entity_id);
        } else {
            selection.select(entity_id);
        }
    } else {
        // Clicked on empty space - clear selection
        if !keyboard.pressed(KeyCode::ControlLeft) && !keyboard.pressed(KeyCode::ControlRight) {
            selection.clear();
        }
    }
}

/// Hover system - detects entity under cursor using batched meshes
#[allow(clippy::too_many_arguments)]
fn hover_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    batched_meshes: Query<(&BatchedMesh, &GlobalTransform, &Mesh3d)>,
    triangle_mapping: Res<TriangleEntityMapping>,
    meshes: Res<Assets<Mesh>>,
    mut selection: ResMut<SelectionState>,
    settings: Res<PickingSettings>,
    mut frame_counter: Local<u32>,
) {
    if !settings.enabled {
        return;
    }

    // Throttle hover detection
    *frame_counter += 1;
    if !(*frame_counter).is_multiple_of(settings.hover_throttle) {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else {
        if selection.hovered.is_some() {
            selection.hovered = None;
        }
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };

    // Create ray from camera through cursor
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    // Find closest intersection in batched meshes
    let mut closest: Option<(u64, f32)> = None;

    for (batched_mesh, transform, mesh_handle) in batched_meshes.iter() {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            if let Some((distance, triangle_index)) =
                ray_mesh_intersection_with_triangle(&ray, mesh, transform)
            {
                // Look up which entity this triangle belongs to
                if let Some(entity_id) =
                    triangle_mapping.get_entity(batched_mesh.is_transparent, triangle_index)
                {
                    if closest.map(|(_, d)| distance < d).unwrap_or(true) {
                        closest = Some((entity_id, distance));
                    }
                }
            }
        }
    }

    // Update hover state
    let new_hovered = closest.map(|(id, _)| id);
    if selection.hovered != new_hovered {
        selection.hovered = new_hovered;
    }
}

/// Ray-mesh intersection with triangle index for batched mesh picking
/// Returns (distance, triangle_index) of the closest hit
fn ray_mesh_intersection_with_triangle(
    ray: &Ray3d,
    mesh: &Mesh,
    transform: &GlobalTransform,
) -> Option<(f32, usize)> {
    // Get vertex positions
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;

    // First do a quick AABB check from vertex positions
    let transform_matrix = transform.affine();
    let (min, max) = compute_world_aabb(positions, &transform_matrix);

    // Quick AABB rejection test
    if !ray_aabb_intersects(ray, min, max) {
        return None;
    }

    // Get indices
    let indices = mesh.indices()?;
    let indices: Vec<usize> = indices.iter().collect();

    let mut closest: Option<(f32, usize)> = None;

    // Iterate through triangles
    for (tri_idx, chunk) in indices.chunks(3).enumerate() {
        if chunk.len() < 3 {
            continue;
        }
        let v0 = transform_matrix.transform_point3(Vec3::from(positions[chunk[0]]));
        let v1 = transform_matrix.transform_point3(Vec3::from(positions[chunk[1]]));
        let v2 = transform_matrix.transform_point3(Vec3::from(positions[chunk[2]]));

        if let Some(t) = ray_triangle_intersection(ray, v0, v1, v2) {
            if t > 0.0 && closest.map(|(d, _)| t < d).unwrap_or(true) {
                closest = Some((t, tri_idx));
            }
        }
    }

    closest
}

/// Compute world-space AABB from vertex positions
fn compute_world_aabb(positions: &[[f32; 3]], transform: &Affine3A) -> (Vec3, Vec3) {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);

    for pos in positions {
        let world_pos = transform.transform_point3(Vec3::from(*pos));
        min = min.min(world_pos);
        max = max.max(world_pos);
    }

    (min, max)
}

/// Möller–Trumbore ray-triangle intersection algorithm
fn ray_triangle_intersection(ray: &Ray3d, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
    const EPSILON: f32 = 1e-7;

    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = ray.direction.cross(edge2);
    let a = edge1.dot(h);

    // Ray is parallel to triangle
    if a.abs() < EPSILON {
        return None;
    }

    let f = 1.0 / a;
    let s = ray.origin - v0;
    let u = f * s.dot(h);

    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * ray.direction.dot(q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);
    if t > EPSILON {
        Some(t)
    } else {
        None
    }
}

/// Quick ray-AABB intersection test
fn ray_aabb_intersects(ray: &Ray3d, min: Vec3, max: Vec3) -> bool {
    let inv_dir = Vec3::new(
        1.0 / ray.direction.x,
        1.0 / ray.direction.y,
        1.0 / ray.direction.z,
    );

    let t1 = (min - ray.origin) * inv_dir;
    let t2 = (max - ray.origin) * inv_dir;

    let tmin = t1.min(t2);
    let tmax = t1.max(t2);

    let t_enter = tmin.x.max(tmin.y).max(tmin.z);
    let t_exit = tmax.x.min(tmax.y).min(tmax.z);

    t_enter <= t_exit && t_exit >= 0.0
}
