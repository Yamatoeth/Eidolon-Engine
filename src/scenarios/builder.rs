//! World builder API for programmatic scenarios.

use std::f32::consts::TAU;

use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::scenarios::loader::{ActiveScenario, AgentDistribution, ScenarioConfig};
use crate::simulation::{
    Agent, AgentId, AgentSpawned, AgentState, Collider, Needs, ResourceNode, SimulationConfig,
    StateKind, Velocity, Zone, ZoneId, ZoneKind,
};

const RESOURCE_NODE_MAX_AMOUNT: f32 = 100.0;
const RESOURCE_NODE_RADIUS: f32 = 1.0;
const AGENT_CAPSULE_RADIUS: f32 = 0.35;
const AGENT_CAPSULE_LENGTH: f32 = 1.0;

/// Marker for entities spawned from a scenario load.
#[derive(Component, Clone, Copy, Debug)]
pub struct ScenarioSpawned;

/// Spawn the active scenario's static world entities.
pub fn spawn_active_scenario_system(
    mut commands: Commands,
    scenario: Option<Res<ActiveScenario>>,
    sim_config: Res<SimulationConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut agent_spawned_events: EventWriter<AgentSpawned>,
) {
    let Some(scenario) = scenario else {
        return;
    };

    spawn_scenario_world(
        &mut commands,
        &scenario.config,
        &sim_config,
        &mut meshes,
        &mut materials,
        &mut agent_spawned_events,
    );
}

/// Spawn zones and resource nodes for a scenario.
pub fn spawn_scenario_world(
    commands: &mut Commands,
    scenario: &ScenarioConfig,
    sim_config: &SimulationConfig,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    agent_spawned_events: &mut EventWriter<AgentSpawned>,
) {
    for (index, zone) in scenario.zones.iter().enumerate() {
        let zone_id = ZoneId(index as u64);
        spawn_zone(commands, meshes, materials, zone_id, zone);

        if zone.kind == ZoneKind::Resource {
            spawn_resource_nodes(commands, meshes, materials, scenario, zone);
        }
    }

    spawn_agents(
        commands,
        meshes,
        materials,
        scenario,
        sim_config,
        agent_spawned_events,
    );
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

fn spawn_agents(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    scenario: &ScenarioConfig,
    sim_config: &SimulationConfig,
    agent_spawned_events: &mut EventWriter<AgentSpawned>,
) {
    let mesh = meshes.add(Capsule3d::new(AGENT_CAPSULE_RADIUS, AGENT_CAPSULE_LENGTH));
    let mut rng = ChaCha8Rng::seed_from_u64(scenario.seed ^ 0xA6E3_9D8B_05C1_7F42);

    for index in 0..sim_config.initial_agent_count {
        let position = agent_spawn_position(index, scenario, sim_config, &mut rng);
        let material = materials.add(StandardMaterial {
            base_color: agent_color(StateKind::Idle),
            perceptual_roughness: 0.65,
            ..default()
        });
        let entity = commands
            .spawn((
                Agent {
                    id: AgentId(u64::from(index)),
                    age: 0.0,
                },
                Needs::default(),
                AgentState::default(),
                Velocity::default(),
                Collider {
                    radius: sim_config.agent_collider_radius,
                },
                ScenarioSpawned,
                Transform::from_translation(position),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
            ))
            .id();

        agent_spawned_events.send(AgentSpawned {
            agent: entity,
            position,
        });
    }
}

/// Keep agent material colors synchronized with their current simulation state.
pub fn agent_visual_state_system(
    query: Query<(&AgentState, &MeshMaterial3d<StandardMaterial>), With<Agent>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (state, material_handle) in &query {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.base_color = agent_color(state.current);
        }
    }
}

fn agent_spawn_position(
    index: u32,
    scenario: &ScenarioConfig,
    sim_config: &SimulationConfig,
    rng: &mut ChaCha8Rng,
) -> Vec3 {
    match scenario.agents.distribution {
        AgentDistribution::Uniform => uniform_agent_position(index, sim_config),
        AgentDistribution::Clustered => clustered_agent_position(index, sim_config),
        AgentDistribution::Random => Vec3::new(
            rng.gen_range(0.0..=sim_config.world_size.x),
            sim_config.agent_visual_height,
            rng.gen_range(0.0..=sim_config.world_size.y),
        ),
    }
}

fn uniform_agent_position(index: u32, sim_config: &SimulationConfig) -> Vec3 {
    let count = sim_config.initial_agent_count.max(1);
    let columns = (count as f32).sqrt().ceil() as u32;
    let rows = count.div_ceil(columns);
    let col = index % columns;
    let row = index / columns;
    let x_step = sim_config.world_size.x / (columns + 1) as f32;
    let z_step = sim_config.world_size.y / (rows + 1) as f32;

    Vec3::new(
        x_step * (col + 1) as f32,
        sim_config.agent_visual_height,
        z_step * (row + 1) as f32,
    )
}

fn clustered_agent_position(index: u32, sim_config: &SimulationConfig) -> Vec3 {
    let count = sim_config.initial_agent_count.max(1);
    let angle = TAU * index as f32 / count as f32;
    let ring = (index / 8) as f32 + 1.0;
    let radius = ring * sim_config.agent_collider_radius * 2.6;
    let center = Vec2::new(sim_config.world_size.x * 0.5, sim_config.world_size.y * 0.5);

    Vec3::new(
        (center.x + angle.cos() * radius).clamp(0.0, sim_config.world_size.x),
        sim_config.agent_visual_height,
        (center.y + angle.sin() * radius).clamp(0.0, sim_config.world_size.y),
    )
}

/// Color used for agent visuals in the current state.
#[must_use]
pub fn agent_color(state: StateKind) -> Color {
    match state {
        StateKind::Idle => Color::srgb(0.72, 0.74, 0.78),
        StateKind::Exploring | StateKind::MovingToTarget => Color::srgb(0.24, 0.72, 0.86),
        StateKind::Eating => Color::srgb(0.36, 0.82, 0.38),
        StateKind::Resting => Color::srgb(0.38, 0.52, 0.95),
        StateKind::Fleeing => Color::srgb(0.92, 0.28, 0.20),
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
