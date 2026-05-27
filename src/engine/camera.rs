//! Camera system — orbit camera with mouse input

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

/// Orbit camera component
#[derive(Component)]
pub struct OrbitCamera {
    /// Distance from target
    pub distance: f32,
    /// Horizontal rotation (yaw) in radians
    pub yaw: f32,
    /// Vertical rotation (pitch) in radians
    pub pitch: f32,
    /// Target point to orbit around
    pub target: Vec3,
    /// Rotation speed (radians per pixel)
    pub rotation_speed: f32,
    /// Zoom speed (distance units per scroll tick)
    pub zoom_speed: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            distance: 60.0,
            yaw: 0.0,
            pitch: FRAC_PI_4,
            target: Vec3::ZERO,
            rotation_speed: 0.005,
            zoom_speed: 5.0,
        }
    }
}

impl OrbitCamera {
    /// Compute camera position from orbit parameters
    pub fn position(&self) -> Vec3 {
        let x = self.target.x + self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.target.y + self.distance * self.pitch.sin();
        let z = self.target.z + self.distance * self.pitch.cos() * self.yaw.cos();
        Vec3::new(x, y, z)
    }
}

/// Spawn the main camera
pub fn spawn_orbit_camera(mut commands: Commands) {
    let camera = OrbitCamera::default();
    let position = camera.position();

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(position).looking_at(camera.target, Vec3::Y),
        camera,
    ));
}

/// Handle mouse input for camera rotation and zoom
pub fn handle_camera_input(
    mut mouse_motion: EventReader<MouseMotion>,
    mut scroll: EventReader<MouseWheel>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut camera_query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    let (mut camera, mut transform) = match camera_query.get_single_mut() {
        Ok(q) => q,
        Err(_) => return,
    };

    // Rotate on left mouse drag
    if mouse_button.pressed(MouseButton::Left) {
        for MouseMotion { delta } in mouse_motion.read() {
            camera.yaw -= delta.x * camera.rotation_speed;
            camera.pitch += delta.y * camera.rotation_speed;

            camera.pitch = camera.pitch.clamp(0.1, FRAC_PI_2 - 0.05);
        }
    }

    for scroll_event in scroll.read() {
        let unit_scale = match scroll_event.unit {
            MouseScrollUnit::Line => 1.0,
            MouseScrollUnit::Pixel => 0.05,
        };
        camera.distance -= scroll_event.y * unit_scale * camera.zoom_speed;
        camera.distance = camera.distance.clamp(10.0, 200.0);
    }

    let position = camera.position();
    *transform = Transform::from_translation(position).looking_at(camera.target, Vec3::Y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_orbits_world_origin() {
        let camera = OrbitCamera::default();
        let position = camera.position();

        assert!((position.x - 0.0).abs() < f32::EPSILON);
        assert!(position.y > 0.0);
        assert!(position.z > 0.0);
    }
}
