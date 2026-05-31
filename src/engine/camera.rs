//! Camera system — orbit camera with mouse input

use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::engine::render::{WORLD_PLANE_CENTER, WORLD_PLANE_SIZE};

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
    /// Pan speed as a fraction of orbit distance per mouse pixel.
    pub pan_speed: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            distance: 68.0,
            yaw: -0.55,
            pitch: 0.62,
            target: WORLD_PLANE_CENTER,
            rotation_speed: 0.005,
            zoom_speed: 5.0,
            pan_speed: 0.0015,
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

/// Handle mouse input for camera rotation, panning, and zoom.
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

    let pointer_delta = mouse_motion
        .read()
        .fold(Vec2::ZERO, |total, MouseMotion { delta }| total + *delta);

    if mouse_button.pressed(MouseButton::Left) {
        camera.yaw -= pointer_delta.x * camera.rotation_speed;
        camera.pitch += pointer_delta.y * camera.rotation_speed;
        camera.pitch = camera.pitch.clamp(0.1, FRAC_PI_2 - 0.05);
    }

    if mouse_button.pressed(MouseButton::Right) || mouse_button.pressed(MouseButton::Middle) {
        pan_camera_target(&mut camera, pointer_delta);
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

fn pan_camera_target(camera: &mut OrbitCamera, pointer_delta: Vec2) {
    if pointer_delta.length_squared() <= f32::EPSILON {
        return;
    }

    let forward = Vec3::new(camera.yaw.sin(), 0.0, camera.yaw.cos()).normalize_or_zero();
    let right = Vec3::new(forward.z, 0.0, -forward.x).normalize_or_zero();
    let scale = camera.distance * camera.pan_speed;

    camera.target += (-right * pointer_delta.x + forward * pointer_delta.y) * scale;
    camera.target.x = camera.target.x.clamp(0.0, WORLD_PLANE_SIZE);
    camera.target.z = camera.target.z.clamp(0.0, WORLD_PLANE_SIZE);
    camera.target.y = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_orbits_world_origin() {
        let camera = OrbitCamera::default();
        let position = camera.position();

        assert!(position.x < camera.target.x);
        assert!(position.y > 0.0);
        assert!(position.z > camera.target.z);
    }

    #[test]
    fn panning_moves_camera_target_on_world_plane() {
        let mut camera = OrbitCamera::default();
        let before = camera.target;

        pan_camera_target(&mut camera, Vec2::new(25.0, -10.0));

        assert_ne!(camera.target, before);
        assert_eq!(camera.target.y, 0.0);
        assert!((0.0..=WORLD_PLANE_SIZE).contains(&camera.target.x));
        assert!((0.0..=WORLD_PLANE_SIZE).contains(&camera.target.z));
    }
}
