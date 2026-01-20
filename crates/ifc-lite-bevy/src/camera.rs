//! Camera system with orbit, pan, and zoom controls
//!
//! Provides a flexible camera controller similar to the TypeScript version.

#[cfg(target_arch = "wasm32")]
use crate::storage::save_camera;
use crate::storage::CameraStorage;
use bevy::ecs::message::MessageReader;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

/// System set for camera input (for ordering)
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CameraInputSet;

/// Camera controller plugin
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraController>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (
                    poll_camera_commands_system,
                    camera_input_system,
                    camera_update_system,
                    camera_keyboard_system,
                )
                    .chain()
                    .in_set(CameraInputSet),
            );
    }
}

impl CameraPlugin {
    /// Get the system set for camera input (for ordering picking after camera)
    pub fn input_system_set() -> CameraInputSet {
        CameraInputSet
    }
}

/// Camera operating mode
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CameraMode {
    #[default]
    Orbit,
    Pan,
    Walk,
}

/// Camera controller resource
#[derive(Resource)]
pub struct CameraController {
    /// Current mode
    pub mode: CameraMode,
    /// Target point to orbit around
    pub target: Vec3,
    /// Distance from target
    pub distance: f32,
    /// Azimuth angle (horizontal rotation)
    pub azimuth: f32,
    /// Elevation angle (vertical rotation)
    pub elevation: f32,
    /// Damping factor for smooth movement (0.0 = instant, 1.0 = never moves)
    pub damping: f32,
    /// Velocity for inertia
    pub velocity: Vec3,
    /// Angular velocity for orbit inertia
    pub angular_velocity: Vec2,
    /// Whether camera is currently animating
    pub is_animating: bool,
    /// Animation target (for preset views)
    pub animation_target: Option<CameraAnimationTarget>,
    /// Field of view in degrees
    pub fov: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Walk mode speed
    pub walk_speed: f32,
    /// Orbit sensitivity
    pub orbit_sensitivity: f32,
    /// Pan sensitivity
    pub pan_sensitivity: f32,
    /// Zoom sensitivity
    pub zoom_sensitivity: f32,
    /// Is dragging (mouse down)
    pub is_dragging: bool,
    /// Last mouse position
    pub last_mouse_pos: Vec2,
    /// Mouse position when drag started (for click detection)
    pub drag_start_pos: Vec2,
    /// Did actual dragging occur (mouse moved significantly)?
    pub did_drag: bool,
    /// Was this a click (released without dragging)?
    pub just_clicked: bool,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            mode: CameraMode::Orbit,
            target: Vec3::ZERO,
            distance: 100.0,  // Start further back for IFC models (in mm)
            azimuth: 0.785,   // 45 degrees
            elevation: 0.615, // ~35 degrees (isometric)
            damping: 0.92,
            velocity: Vec3::ZERO,
            angular_velocity: Vec2::ZERO,
            is_animating: false,
            animation_target: None,
            fov: 45.0,
            near: 1.0,         // 1mm near plane for IFC-scale models
            far: 1000000.0,    // 1km far plane for large IFC models
            walk_speed: 500.0, // 0.5m per frame for walking in mm-scale
            orbit_sensitivity: 0.005,
            pan_sensitivity: 0.01,
            zoom_sensitivity: 0.02,
            is_dragging: false,
            last_mouse_pos: Vec2::ZERO,
            drag_start_pos: Vec2::ZERO,
            did_drag: false,
            just_clicked: false,
        }
    }
}

impl CameraController {
    /// Get camera position from spherical coordinates
    pub fn get_position(&self) -> Vec3 {
        let x = self.distance * self.elevation.cos() * self.azimuth.sin();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.elevation.cos() * self.azimuth.cos();
        self.target + Vec3::new(x, y, z)
    }

    /// Set preset view
    pub fn set_preset_view(&mut self, azimuth: f32, elevation: f32) {
        self.animation_target = Some(CameraAnimationTarget {
            azimuth,
            elevation,
            distance: self.distance,
            target: self.target,
            duration: 0.5,
            elapsed: 0.0,
        });
        self.is_animating = true;
    }

    /// Set home/isometric view
    pub fn home(&mut self) {
        self.set_preset_view(0.785, 0.615); // 45°, 35.264°
    }

    /// Fit all - zoom to show entire scene
    pub fn fit_bounds(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) * 0.5;
        let size = max - min;
        let diagonal = size.length();

        // Calculate distance to fit the entire model
        let fov_rad = self.fov.to_radians();
        let distance = diagonal / (2.0 * (fov_rad / 2.0).tan());

        self.animation_target = Some(CameraAnimationTarget {
            azimuth: self.azimuth,
            elevation: self.elevation,
            distance: distance.max(1.0),
            target: center,
            duration: 0.5,
            elapsed: 0.0,
        });
        self.is_animating = true;
    }

    /// Frame selection - zoom to specific bounds
    pub fn frame(&mut self, min: Vec3, max: Vec3) {
        self.fit_bounds(min, max);
    }

    /// Zoom in
    pub fn zoom_in(&mut self) {
        self.distance = (self.distance * 0.8).max(1.0);
    }

    /// Zoom out
    pub fn zoom_out(&mut self) {
        self.distance = (self.distance * 1.25).min(500000.0);
    }

    /// Convert to storage format
    pub fn to_storage(&self) -> CameraStorage {
        CameraStorage {
            azimuth: self.azimuth,
            elevation: self.elevation,
            distance: self.distance,
            target: [self.target.x, self.target.y, self.target.z],
        }
    }

    /// Load from storage format
    pub fn from_storage(&mut self, storage: &CameraStorage) {
        self.azimuth = storage.azimuth;
        self.elevation = storage.elevation;
        self.distance = storage.distance;
        self.target = Vec3::new(storage.target[0], storage.target[1], storage.target[2]);
    }
}

/// Animation target for smooth camera transitions
#[derive(Clone, Debug)]
pub struct CameraAnimationTarget {
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
    pub target: Vec3,
    pub duration: f32,
    pub elapsed: f32,
}

/// Marker component for the main camera
#[derive(Component)]
pub struct MainCamera;

/// System to poll for camera commands from Yew UI
#[allow(unused_variables, unused_mut)]
fn poll_camera_commands_system(
    mut controller: ResMut<CameraController>,
    scene_data: Res<crate::IfcSceneData>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(cmd) = crate::storage::load_camera_cmd() {
            crate::storage::clear_camera_cmd();

            match cmd.cmd.as_str() {
                "home" => {
                    controller.home();
                }
                "fit_all" => {
                    if let Some(ref bounds) = scene_data.bounds {
                        controller.fit_bounds(bounds.min, bounds.max);
                    }
                }
                "set_mode" => {
                    if let Some(mode) = cmd.mode {
                        controller.mode = match mode.as_str() {
                            "pan" => CameraMode::Pan,
                            "walk" => CameraMode::Walk,
                            _ => CameraMode::Orbit,
                        };
                    }
                }
                _ => {}
            }
        }
    }
}

/// Setup the 3D camera
fn setup_camera(mut commands: Commands, controller: Res<CameraController>) {
    use bevy::render::view::Msaa;

    let position = controller.get_position();

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(position).looking_at(controller.target, Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: controller.fov.to_radians(),
            near: controller.near,
            far: controller.far,
            ..default()
        }),
        MainCamera,
        // Enable 4x MSAA for smoother edges
        Msaa::Sample4,
    ));

    // Ambient light - lower for more contrast (like original viewer)
    commands.spawn(AmbientLight {
        color: Color::WHITE,
        brightness: 80.0, // Much lower ambient for better contrast
        affects_lightmapped_meshes: true,
    });

    // Key directional light - main light from top-right-front
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.99, 0.97), // Slightly warm
            illuminance: 25000.0,                // Strong key light
            shadows_enabled: false,
            affects_lightmapped_mesh_diffuse: true,
            ..default()
        },
        Transform::from_xyz(0.5, 1.0, 0.3).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Fill light from opposite side - subtle
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.85, 0.9, 1.0), // Cool fill
            illuminance: 8000.0,                // Moderate fill
            shadows_enabled: false,
            affects_lightmapped_mesh_diffuse: true,
            ..default()
        },
        Transform::from_xyz(-0.5, 0.3, -0.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Rim/back light for edge definition
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.9, 0.95, 1.0),
            illuminance: 5000.0,
            shadows_enabled: false,
            affects_lightmapped_mesh_diffuse: true,
            ..default()
        },
        Transform::from_xyz(-0.3, 0.8, -0.8).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Handle mouse input for camera control
#[allow(unused_variables)]
fn camera_input_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut controller: ResMut<CameraController>,
    windows: Query<&Window>,
    // Check if mouse is over any UI element with Interaction (only when bevy-ui feature is enabled)
    #[cfg(feature = "bevy-ui")] ui_interactions: Query<&Interaction, With<Node>>,
) {
    let Ok(window) = windows.single() else { return };

    // Check if mouse is over any UI element (hovered or pressed)
    #[cfg(feature = "bevy-ui")]
    let mouse_over_ui = ui_interactions
        .iter()
        .any(|interaction| matches!(interaction, Interaction::Hovered | Interaction::Pressed));
    #[cfg(not(feature = "bevy-ui"))]
    let mouse_over_ui = false;

    // Handle mouse button state - only start drag if not over UI
    if mouse_button.just_pressed(MouseButton::Left) && !mouse_over_ui {
        controller.is_dragging = true;
        controller.did_drag = false;
        controller.just_clicked = false; // Reset on press
        if let Some(pos) = window.cursor_position() {
            controller.last_mouse_pos = pos;
            controller.drag_start_pos = pos;
        }
    }
    if mouse_button.just_released(MouseButton::Left) {
        // Check if this was a click (no significant drag)
        if !controller.did_drag {
            controller.just_clicked = true;
        }
        controller.is_dragging = false;
    }

    // Handle mouse motion
    if controller.is_dragging {
        for ev in mouse_motion.read() {
            // Mark as drag if mouse moved significantly (more than 3 pixels)
            if ev.delta.length() > 3.0 {
                controller.did_drag = true;
            }

            match controller.mode {
                CameraMode::Orbit => {
                    controller.azimuth -= ev.delta.x * controller.orbit_sensitivity;
                    controller.elevation -= ev.delta.y * controller.orbit_sensitivity;
                    // Clamp elevation to avoid gimbal lock
                    controller.elevation = controller.elevation.clamp(-1.5, 1.5);
                    // Store angular velocity for inertia
                    controller.angular_velocity = ev.delta * controller.orbit_sensitivity;
                }
                CameraMode::Pan => {
                    // Calculate pan in camera space
                    let right = Vec3::new(controller.azimuth.cos(), 0.0, -controller.azimuth.sin());
                    let up = Vec3::Y;
                    let pan = right
                        * ev.delta.x
                        * controller.pan_sensitivity
                        * controller.distance
                        * 0.01
                        - up * ev.delta.y * controller.pan_sensitivity * controller.distance * 0.01;
                    controller.target += pan;
                }
                CameraMode::Walk => {
                    // First-person look
                    controller.azimuth -= ev.delta.x * controller.orbit_sensitivity * 0.5;
                    controller.elevation -= ev.delta.y * controller.orbit_sensitivity * 0.5;
                    controller.elevation = controller.elevation.clamp(-1.5, 1.5);
                }
            }
        }
    } else {
        // Apply damping to angular velocity when not dragging
        let damping = controller.damping;
        controller.angular_velocity *= damping;
        if controller.angular_velocity.length() > 0.0001 {
            controller.azimuth -= controller.angular_velocity.x;
            controller.elevation -= controller.angular_velocity.y;
            controller.elevation = controller.elevation.clamp(-1.5, 1.5);
        }
    }

    // Handle mouse wheel for zoom - only when NOT over UI
    if !mouse_over_ui {
        for ev in mouse_wheel.read() {
            let zoom_delta = ev.y * controller.zoom_sensitivity;
            controller.distance = (controller.distance * (1.0 - zoom_delta)).clamp(1.0, 500000.0);
        }
    }
}

/// Handle keyboard input for camera control
fn camera_keyboard_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut controller: ResMut<CameraController>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // Walk mode movement (WASD)
    if controller.mode == CameraMode::Walk {
        let forward = Vec3::new(
            -controller.azimuth.sin() * controller.elevation.cos(),
            controller.elevation.sin(),
            -controller.azimuth.cos() * controller.elevation.cos(),
        )
        .normalize();
        let right = Vec3::new(controller.azimuth.cos(), 0.0, -controller.azimuth.sin());

        let mut movement = Vec3::ZERO;

        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            movement += forward;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            movement -= forward;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            movement -= right;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            movement += right;
        }
        if keyboard.pressed(KeyCode::KeyQ) {
            movement -= Vec3::Y;
        }
        if keyboard.pressed(KeyCode::KeyE) {
            movement += Vec3::Y;
        }

        if movement.length() > 0.0 {
            let walk_speed = controller.walk_speed;
            controller.target += movement.normalize() * walk_speed * dt;
        }
    }

    // Preset views (number keys)
    if keyboard.just_pressed(KeyCode::Digit1) {
        controller.set_preset_view(0.0, 0.0); // Front
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        controller.set_preset_view(std::f32::consts::PI, 0.0); // Back
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        controller.set_preset_view(-std::f32::consts::FRAC_PI_2, 0.0); // Left
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        controller.set_preset_view(std::f32::consts::FRAC_PI_2, 0.0); // Right
    }
    if keyboard.just_pressed(KeyCode::Digit5) {
        controller.set_preset_view(0.0, std::f32::consts::FRAC_PI_2 - 0.001); // Top
    }
    if keyboard.just_pressed(KeyCode::Digit6) {
        controller.set_preset_view(0.0, -std::f32::consts::FRAC_PI_2 + 0.001); // Bottom
    }
    if keyboard.just_pressed(KeyCode::KeyH) {
        controller.home(); // Isometric
    }
}

/// Update camera transform
fn camera_update_system(
    mut controller: ResMut<CameraController>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // Handle animation
    if controller.animation_target.is_some() {
        // Extract animation target data to avoid borrow conflicts
        let animation_data = {
            let target = controller.animation_target.as_mut().unwrap();
            target.elapsed += dt;
            let t = (target.elapsed / target.duration).min(1.0);
            // Ease out cubic
            let t = 1.0 - (1.0 - t).powi(3);
            let completed = target.elapsed >= target.duration;
            (
                target.azimuth,
                target.elevation,
                target.distance,
                target.target,
                t,
                completed,
            )
        };

        let (target_azimuth, target_elevation, target_distance, target_pos, t, completed) =
            animation_data;

        controller.azimuth = lerp(controller.azimuth, target_azimuth, t);
        controller.elevation = lerp(controller.elevation, target_elevation, t);
        controller.distance = lerp(controller.distance, target_distance, t);
        controller.target = controller.target.lerp(target_pos, t);

        if completed {
            controller.animation_target = None;
            controller.is_animating = false;
        }
    }

    // Update camera transform
    if let Ok(mut transform) = camera.single_mut() {
        let position = controller.get_position();

        // Apply damping for smooth movement
        transform.translation = transform
            .translation
            .lerp(position, 1.0 - controller.damping.powi(2));
        transform.look_at(controller.target, Vec3::Y);
    }

    // Save camera state periodically (WASM)
    #[cfg(target_arch = "wasm32")]
    {
        // Only save occasionally to avoid flooding localStorage
        static mut SAVE_COUNTER: u32 = 0;
        unsafe {
            SAVE_COUNTER += 1;
            if SAVE_COUNTER % 30 == 0 {
                save_camera(&controller.to_storage());
            }
        }
    }
}

/// Linear interpolation
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
