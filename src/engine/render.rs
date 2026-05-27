//! Rendering and 3D scene setup

use bevy::prelude::*;

/// Width and depth of the Phase 1 world plane.
pub const WORLD_PLANE_SIZE: f32 = 100.0;

/// Runtime toggle for the engine debug grid overlay.
#[derive(Resource, Debug, Clone, Copy)]
pub struct DebugGridConfig {
    /// Whether the ground grid should be drawn.
    pub enabled: bool,
    /// Distance between grid lines.
    pub cell_size: f32,
    /// Total grid width and depth.
    pub size: f32,
}

impl Default for DebugGridConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cell_size: 10.0,
            size: WORLD_PLANE_SIZE,
        }
    }
}

/// Spawn the 3D scene with lighting, shadows, and a flat world plane.
pub fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground_mesh = meshes.add(
        Plane3d::default()
            .mesh()
            .size(WORLD_PLANE_SIZE, WORLD_PLANE_SIZE),
    );
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.38, 0.25),
        perceptual_roughness: 0.95,
        ..default()
    });

    commands.spawn((
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_material),
        Transform::IDENTITY,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 20000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::PI / 4.0,
            std::f32::consts::PI / 4.0,
            0.0,
        )),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 450.0,
    });
}

/// Draw a toggleable debug grid over the ground plane.
pub fn draw_debug_grid_system(config: Res<DebugGridConfig>, mut gizmos: Gizmos) {
    if !config.enabled {
        return;
    }

    let half_size = config.size * 0.5;
    let line_count = (config.size / config.cell_size).round() as i32;
    let color = Color::srgba(0.85, 0.9, 0.95, 0.28);
    let y = 0.02;

    for index in 0..=line_count {
        let offset = -half_size + index as f32 * config.cell_size;
        gizmos.line(
            Vec3::new(-half_size, y, offset),
            Vec3::new(half_size, y, offset),
            color,
        );
        gizmos.line(
            Vec3::new(offset, y, -half_size),
            Vec3::new(offset, y, half_size),
            color,
        );
    }
}
