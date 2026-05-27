//! World builder API for programmatic scenarios.

use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::scenarios::loader::{ActiveScenario, ScenarioConfig};
use crate::simulation::{Collider, ResourceNode, Zone, ZoneId, ZoneKind};

const RESOURCE_NODE_MAX_AMOUNT: f32 = 100.0;
const RESOURCE_NODE_RADIUS: f32 = 1.0;

/// Marker for entities spawned from a scenario load.
#[derive(Component, Clone, Copy, Debug)]
pub struct ScenarioSpawned;

/// Spawn the active scenario's static world entities.
pub fn spawn_active_scenario_system(
    mut commands: Commands,
    scenario: Option<Res<ActiveScenario>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(scenario) = scenario else {
        return;
    };

    spawn_scenario_world(&mut commands, &scenario.config, &mut meshes, &mut materials);
}

/// Spawn zones and resource nodes for a scenario.
pub fn spawn_scenario_world(
    commands: &mut Commands,
    scenario: &ScenarioConfig,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    for (index, zone) in scenario.zones.iter().enumerate() {
        let zone_id = ZoneId(index as u64);
        spawn_zone(commands, meshes, materials, zone_id, zone);

        if zone.kind == ZoneKind::Resource {
            spawn_resource_nodes(commands, meshes, materials, scenario, zone);
        }
    }
}

fn spawn_zone(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    id: ZoneId,
    zone: &crate::scenarios::loader::ZoneConfig,
) {
    let material = materials.add(StandardMaterial {
        base_color: zone_color(zone.kind),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.spawn((
        Zone {
            id,
            kind: zone.kind,
            radius: zone.radius,
        },
        Transform::from_translation(zone.center + Vec3::Y * 0.04),
        Collider {
            radius: zone.radius,
        },
        ScenarioSpawned,
        Mesh3d(meshes.add(Cylinder::new(zone.radius, 0.04).mesh().resolution(64))),
        MeshMaterial3d(material),
    ));
}

fn spawn_resource_nodes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    scenario: &ScenarioConfig,
    zone: &crate::scenarios::loader::ZoneConfig,
) {
    let count = scenario.resources.nodes_per_resource_zone;
    if count == 0 {
        return;
    }

    let mesh = meshes.add(Sphere::new(RESOURCE_NODE_RADIUS));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.52, 0.18),
        perceptual_roughness: 0.75,
        ..default()
    });
    let placement_radius = (zone.radius * 0.45).max(RESOURCE_NODE_RADIUS * 2.0);

    for index in 0..count {
        let angle = TAU * index as f32 / count as f32;
        let offset = Vec3::new(
            angle.cos() * placement_radius,
            0.8,
            angle.sin() * placement_radius,
        );

        commands.spawn((
            ResourceNode::food(
                RESOURCE_NODE_MAX_AMOUNT,
                scenario.resources.initial_amount_fraction,
                scenario.resources.regen_rate * scenario.sim_overrides.global_regen_multiplier,
            ),
            Transform::from_translation(zone.center + offset),
            Collider {
                radius: RESOURCE_NODE_RADIUS,
            },
            ScenarioSpawned,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
        ));
    }
}

fn zone_color(kind: ZoneKind) -> Color {
    match kind {
        ZoneKind::Resource => Color::srgba(0.18, 0.62, 0.28, 0.32),
        ZoneKind::Rest => Color::srgba(0.18, 0.38, 0.9, 0.28),
        ZoneKind::Neutral => Color::srgba(0.58, 0.58, 0.58, 0.22),
        ZoneKind::Hazard => Color::srgba(0.86, 0.18, 0.12, 0.30),
    }
}
