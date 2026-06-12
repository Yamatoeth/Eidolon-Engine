//! World builder API for programmatic scenarios.

use std::f32::consts::TAU;

use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::engine::render::agent_needs_color;
use crate::scenarios::loader::{ActiveScenario, AgentDistribution, ScenarioConfig};
use crate::simulation::{
    Agent, AgentId, AgentSpawned, AgentState, CarriedResource, Collider, Needs, ResourceNode,
    SimulationConfig, Velocity, VillageStore, Zone, ZoneId, ZoneKind,
};

const RESOURCE_NODE_MAX_AMOUNT: f32 = 100.0;
const RESOURCE_NODE_RADIUS: f32 = 1.2;
const RESOURCE_NODE_VISUAL_SCALE: f32 = 1.3;
const AGENT_VISUAL_SCALE: f32 = 1.4;
pub const AGENT_CAPSULE_RADIUS: f32 = 0.42;
pub const AGENT_CAPSULE_LENGTH: f32 = 1.2;

#[derive(Clone)]
struct AgentVisualAssets {
    body_mesh: Handle<Mesh>,
    head_mesh: Handle<Mesh>,
    visor_mesh: Handle<Mesh>,
    leg_mesh: Handle<Mesh>,
    head_material: Handle<StandardMaterial>,
    visor_material: Handle<StandardMaterial>,
    limb_material: Handle<StandardMaterial>,
}

type AgentCargoVisualQueryItem<'w> = (Entity, Option<&'w CarriedResource>, Option<&'w Children>);

/// Marker for entities spawned from a scenario load.
#[derive(Component, Clone, Copy, Debug)]
pub struct ScenarioSpawned;

#[derive(Component)]
pub struct CarriedResourceVisual;

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

    spawn_initial_agents(
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
        base_color: zone_fill_color(zone.kind),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.96,
        metallic: 0.0,
        reflectance: 0.08,
        ..default()
    });

    let mut entity = commands.spawn((
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

    if zone.kind == ZoneKind::Rest {
        entity.insert(VillageStore::new(zone.radius * 18.0));
        spawn_village_decor(commands, meshes, materials, zone.center, zone.radius);
    }
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

    let mesh = meshes.add(
        Cone::new(
            0.72 * RESOURCE_NODE_VISUAL_SCALE,
            1.85 * RESOURCE_NODE_VISUAL_SCALE,
        )
        .mesh()
        .resolution(5),
    );
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.66, 0.32),
        emissive: LinearRgba::rgb(0.24, 0.12, 0.03),
        perceptual_roughness: 0.34,
        metallic: 0.08,
        reflectance: 0.42,
        ..default()
    });
    let shard_mesh = meshes.add(
        Cone::new(
            0.34 * RESOURCE_NODE_VISUAL_SCALE,
            1.15 * RESOURCE_NODE_VISUAL_SCALE,
        )
        .mesh()
        .resolution(4),
    );
    let shard_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.78, 0.44),
        emissive: LinearRgba::rgb(0.18, 0.10, 0.03),
        perceptual_roughness: 0.38,
        metallic: 0.06,
        reflectance: 0.36,
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

        commands
            .spawn((
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
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(shard_mesh.clone()),
                    MeshMaterial3d(shard_material.clone()),
                    Transform::from_xyz(-0.52, -0.18, 0.12)
                        .with_rotation(Quat::from_rotation_z(0.28)),
                ));
                parent.spawn((
                    Mesh3d(shard_mesh.clone()),
                    MeshMaterial3d(shard_material.clone()),
                    Transform::from_xyz(0.46, -0.26, -0.22)
                        .with_rotation(Quat::from_rotation_z(-0.22)),
                ));
            });
    }
}

fn spawn_initial_agents(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    scenario: &ScenarioConfig,
    sim_config: &SimulationConfig,
    agent_spawned_events: &mut EventWriter<AgentSpawned>,
) {
    let visuals = create_agent_visual_assets(meshes, materials);
    let mut rng = ChaCha8Rng::seed_from_u64(scenario.seed ^ 0xA6E3_9D8B_05C1_7F42);

    for index in 0..sim_config.initial_agent_count {
        let position = agent_spawn_position(index, scenario, sim_config, &mut rng);
        spawn_agent_entity(
            commands,
            &visuals,
            materials,
            AgentId(u64::from(index)),
            position,
            sim_config.agent_collider_radius,
            agent_spawned_events,
        );
    }
}

/// Spawn one agent at a world position.
pub fn spawn_agent_at_position(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    agent_id: AgentId,
    position: Vec3,
    sim_config: &SimulationConfig,
    agent_spawned_events: &mut EventWriter<AgentSpawned>,
) {
    let visuals = create_agent_visual_assets(meshes, materials);
    spawn_agent_entity(
        commands,
        &visuals,
        materials,
        agent_id,
        Vec3::new(
            position.x.clamp(0.0, sim_config.world_size.x),
            sim_config.agent_visual_height,
            position.z.clamp(0.0, sim_config.world_size.y),
        ),
        sim_config.agent_collider_radius,
        agent_spawned_events,
    );
}

fn spawn_agent_entity(
    commands: &mut Commands,
    visuals: &AgentVisualAssets,
    materials: &mut Assets<StandardMaterial>,
    id: AgentId,
    position: Vec3,
    collider_radius: f32,
    agent_spawned_events: &mut EventWriter<AgentSpawned>,
) -> Entity {
    let body_material = create_agent_body_material(materials);
    let entity = commands
        .spawn((
            Agent { id, age: 0.0 },
            Needs::default(),
            AgentState::default(),
            Velocity::default(),
            Collider {
                radius: collider_radius,
            },
            ScenarioSpawned,
            Transform::from_translation(position),
            Mesh3d(visuals.body_mesh.clone()),
            MeshMaterial3d(body_material),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(visuals.head_mesh.clone()),
                MeshMaterial3d(visuals.head_material.clone()),
                Transform::from_xyz(0.0, 0.78, 0.0),
            ));
            parent.spawn((
                Mesh3d(visuals.visor_mesh.clone()),
                MeshMaterial3d(visuals.visor_material.clone()),
                Transform::from_xyz(0.0, 0.82, 0.29),
            ));
            for x in [-0.23, 0.23] {
                parent.spawn((
                    Mesh3d(visuals.leg_mesh.clone()),
                    MeshMaterial3d(visuals.limb_material.clone()),
                    Transform::from_xyz(x, -0.62, 0.0),
                ));
            }
        })
        .id();

    agent_spawned_events.send(AgentSpawned {
        agent: entity,
        position,
    });
    entity
}

fn create_agent_visual_assets(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> AgentVisualAssets {
    AgentVisualAssets {
        body_mesh: meshes.add(Cuboid::new(
            0.72 * AGENT_VISUAL_SCALE,
            1.0 * AGENT_VISUAL_SCALE,
            0.52 * AGENT_VISUAL_SCALE,
        )),
        head_mesh: meshes.add(Cuboid::new(
            0.56 * AGENT_VISUAL_SCALE,
            0.34 * AGENT_VISUAL_SCALE,
            0.46 * AGENT_VISUAL_SCALE,
        )),
        visor_mesh: meshes.add(Cuboid::new(
            0.48 * AGENT_VISUAL_SCALE,
            0.08 * AGENT_VISUAL_SCALE,
            0.06 * AGENT_VISUAL_SCALE,
        )),
        leg_mesh: meshes.add(Cuboid::new(
            0.18 * AGENT_VISUAL_SCALE,
            0.34 * AGENT_VISUAL_SCALE,
            0.18 * AGENT_VISUAL_SCALE,
        )),
        head_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.84, 0.92, 0.90),
            perceptual_roughness: 0.45,
            metallic: 0.02,
            reflectance: 0.36,
            ..default()
        }),
        visor_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.18, 0.22),
            emissive: LinearRgba::rgb(0.0, 0.20, 0.24),
            perceptual_roughness: 0.28,
            ..default()
        }),
        limb_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.32, 0.35),
            perceptual_roughness: 0.52,
            reflectance: 0.22,
            ..default()
        }),
    }
}

fn create_agent_body_material(
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: agent_needs_color(&Needs::default()),
        perceptual_roughness: 0.42,
        metallic: 0.03,
        reflectance: 0.38,
        ..default()
    })
}

/// Keep legacy scenario-registered agent visuals aligned with engine need colors.
pub fn agent_visual_state_system(
    query: Query<(&Needs, &Velocity, &MeshMaterial3d<StandardMaterial>), With<Agent>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (needs, velocity, material_handle) in &query {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.base_color = agent_needs_color(needs);
            material.emissive = if velocity.linear.length_squared() > 0.01 {
                LinearRgba::rgb(0.02, 0.08, 0.09)
            } else {
                LinearRgba::BLACK
            };
        }
    }
}

/// Attach or remove a small carried-resource marker on agents with cargo.
pub fn carried_resource_visual_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    agents: Query<AgentCargoVisualQueryItem, With<Agent>>,
    carried_visuals: Query<Entity, With<CarriedResourceVisual>>,
    mut visual_assets: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    let (mesh, material) = visual_assets
        .get_or_insert_with(|| {
            (
                meshes.add(Cone::new(0.18, 0.36).mesh().resolution(5)),
                materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.76, 0.28),
                    emissive: LinearRgba::rgb(0.14, 0.08, 0.01),
                    perceptual_roughness: 0.38,
                    reflectance: 0.35,
                    ..default()
                }),
            )
        })
        .clone();

    for (agent, cargo, children) in &agents {
        let visual_child = children.and_then(|children| {
            children
                .iter()
                .copied()
                .find(|child| carried_visuals.get(*child).is_ok())
        });

        match (cargo, visual_child) {
            (Some(_), None) => {
                commands.entity(agent).with_children(|parent| {
                    parent.spawn((
                        CarriedResourceVisual,
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_xyz(0.0, 0.36, 0.44)
                            .with_rotation(Quat::from_rotation_z(0.18)),
                    ));
                });
            },
            (None, Some(child)) => {
                commands.entity(child).despawn();
            },
            _ => {},
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

fn spawn_village_decor(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    radius: f32,
) {
    let hut_mesh = meshes.add(Cuboid::new(1.7, 1.0, 1.45));
    let roof_mesh = meshes.add(Cone::new(1.25, 0.95).mesh().resolution(4));
    let post_mesh = meshes.add(Cuboid::new(0.12, 0.55, 0.12));
    let hut_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.72, 0.70),
        perceptual_roughness: 0.82,
        reflectance: 0.12,
        ..default()
    });
    let roof_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.42, 0.76),
        perceptual_roughness: 0.88,
        reflectance: 0.10,
        ..default()
    });
    let post_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.25, 0.24),
        perceptual_roughness: 0.9,
        ..default()
    });

    for index in 0..4 {
        let angle = TAU * index as f32 / 4.0 + 0.45;
        let distance = radius * 0.42;
        let position = center + Vec3::new(angle.cos() * distance, 0.52, angle.sin() * distance);
        let yaw = -angle + std::f32::consts::FRAC_PI_2;

        commands
            .spawn((
                ScenarioSpawned,
                Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(hut_mesh.clone()),
                    MeshMaterial3d(hut_material.clone()),
                    Transform::IDENTITY,
                ));
                parent.spawn((
                    Mesh3d(roof_mesh.clone()),
                    MeshMaterial3d(roof_material.clone()),
                    Transform::from_xyz(0.0, 0.84, 0.0)
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)),
                ));
                parent.spawn((
                    Mesh3d(post_mesh.clone()),
                    MeshMaterial3d(post_material.clone()),
                    Transform::from_xyz(0.0, -0.12, 0.86),
                ));
            });
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

fn zone_fill_color(kind: ZoneKind) -> Color {
    match kind {
        ZoneKind::Resource => Color::srgba(0.15, 0.72, 0.42, 0.24),
        ZoneKind::Rest => Color::srgba(0.22, 0.42, 0.95, 0.22),
        ZoneKind::Neutral => Color::srgba(0.58, 0.63, 0.65, 0.18),
        ZoneKind::Hazard => Color::srgba(0.90, 0.18, 0.12, 0.26),
    }
}
