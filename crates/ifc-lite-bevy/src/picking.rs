//! Picking and selection system
//!
//! Handles raycasting for object selection and hover detection.

use bevy::prelude::*;
use bevy::camera::primitives::MeshAabb;
use bevy::window::PrimaryWindow;
use rustc_hash::FxHashSet;
use crate::camera::MainCamera;
use crate::mesh::IfcEntity;
use crate::storage::{SelectionStorage, save_selection};

/// Picking plugin
pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionState>()
            .init_resource::<PickingSettings>()
            .add_systems(Update, (
                picking_system,
                hover_system,
            ));
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

/// Picking system - handles click selection
fn picking_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    entities: Query<(&IfcEntity, &GlobalTransform, &Mesh3d)>,
    meshes: Res<Assets<Mesh>>,
    mut selection: ResMut<SelectionState>,
    settings: Res<PickingSettings>,
) {
    if !settings.enabled {
        return;
    }

    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_transform)) = cameras.single() else { return };

    // Create ray from camera through cursor
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else { return };

    // Find closest intersection
    let mut closest: Option<(u64, f32)> = None;

    for (ifc_entity, transform, mesh_handle) in entities.iter() {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            if let Some(distance) = ray_mesh_intersection(&ray, mesh, transform) {
                if closest.map(|(_, d)| distance < d).unwrap_or(true) {
                    closest = Some((ifc_entity.id, distance));
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
        if !keyboard.pressed(KeyCode::ControlLeft)
            && !keyboard.pressed(KeyCode::ControlRight) {
            selection.clear();
        }
    }
}

/// Hover system - detects entity under cursor
fn hover_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    entities: Query<(&IfcEntity, &GlobalTransform, &Mesh3d)>,
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
    if *frame_counter % settings.hover_throttle != 0 {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else {
        if selection.hovered.is_some() {
            selection.hovered = None;
        }
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else { return };

    // Create ray from camera through cursor
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else { return };

    // Find closest intersection
    let mut closest: Option<(u64, f32)> = None;

    for (ifc_entity, transform, mesh_handle) in entities.iter() {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            if let Some(distance) = ray_mesh_intersection(&ray, mesh, transform) {
                if closest.map(|(_, d)| distance < d).unwrap_or(true) {
                    closest = Some((ifc_entity.id, distance));
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

/// Simple ray-mesh intersection using bounding box (fast approximation)
/// For more accurate picking, use bevy_mod_raycast or similar
fn ray_mesh_intersection(
    ray: &Ray3d,
    mesh: &Mesh,
    transform: &GlobalTransform,
) -> Option<f32> {
    // Get mesh AABB
    let aabb = mesh.compute_aabb()?;

    // Transform AABB to world space (approximate)
    // Convert Vec3A to Vec3 for multiplication with scale
    let center: Vec3 = aabb.center.into();
    let half_extents: Vec3 = aabb.half_extents.into();

    let world_center = transform.transform_point(center);
    let scale = transform.to_scale_rotation_translation().0;
    let world_half_extents = half_extents * scale;

    let min: Vec3 = world_center - world_half_extents;
    let max: Vec3 = world_center + world_half_extents;

    // Ray-AABB intersection (slab method)
    let inv_dir = Vec3::new(
        1.0 / ray.direction.x,
        1.0 / ray.direction.y,
        1.0 / ray.direction.z,
    );

    let t1: Vec3 = (min - ray.origin) * inv_dir;
    let t2: Vec3 = (max - ray.origin) * inv_dir;

    let tmin = t1.min(t2);
    let tmax = t1.max(t2);

    let t_enter = tmin.x.max(tmin.y).max(tmin.z);
    let t_exit = tmax.x.min(tmax.y).min(tmax.z);

    if t_enter <= t_exit && t_exit >= 0.0 {
        Some(t_enter.max(0.0))
    } else {
        None
    }
}
