//! Rendering and 3D scene setup

use bevy::prelude::*;

/// Width and depth of the Phase 1 world plane.
pub const WORLD_PLANE_SIZE: f32 = 100.0;
/// Center of the simulation's default X/Z world bounds.
pub const WORLD_PLANE_CENTER: Vec3 = Vec3::new(WORLD_PLANE_SIZE * 0.5, 0.0, WORLD_PLANE_SIZE * 0.5);

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
        base_color: Color::srgb(0.12, 0.18, 0.16),
        perceptual_roughness: 0.88,
        metallic: 0.02,
        reflectance: 0.18,
        ..default()
    });

    commands.spawn((
        Mesh3d(ground_mesh),
        MeshMaterial3d(ground_material),
        Transform::from_translation(WORLD_PLANE_CENTER),
    ));

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.94, 0.84),
            illuminance: 28000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(35.0, 80.0, 25.0).looking_at(WORLD_PLANE_CENTER, Vec3::Y),
    ));

    commands.spawn((
        PointLight {
            color: Color::srgb(0.45, 0.68, 1.0),
            intensity: 700.0,
            range: 90.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(82.0, 18.0, 72.0),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.72, 0.82, 1.0),
        brightness: 260.0,
    });
}

/// Draw a toggleable debug grid over the ground plane.
pub fn draw_debug_grid_system(config: Res<DebugGridConfig>, mut gizmos: Gizmos) {
    if !config.enabled {
        return;
    }

    let line_count = (config.size / config.cell_size).round() as i32;
    let color = Color::srgba(0.60, 0.72, 0.78, 0.20);
    let axis_color = Color::srgba(0.68, 0.92, 0.86, 0.32);
    let y = 0.02;

    for index in 0..=line_count {
        let offset = index as f32 * config.cell_size;
        let line_color = if index == 0 || index == line_count {
            axis_color
        } else {
            color
        };
        gizmos.line(
            Vec3::new(0.0, y, offset),
            Vec3::new(config.size, y, offset),
            line_color,
        );
        gizmos.line(
            Vec3::new(offset, y, 0.0),
            Vec3::new(offset, y, config.size),
            line_color,
        );
    }
}
