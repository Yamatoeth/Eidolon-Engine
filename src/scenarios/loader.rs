//! Loads scenario definitions from RON assets.

use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::scenarios::builder::{spawn_agent_at_position, spawn_scenario_world, ScenarioSpawned};
use crate::scenarios::presets::ScenarioPreset;
use crate::simulation::{
    AgentId, AgentSpawned, NeedsDecayRates, SimRng, SimulationConfig, ZoneKind,
};

/// Default scenario loaded at startup.
pub const DEFAULT_SCENARIO_PATH: &str = "assets/scenarios/equilibrium.ron";
const SCENARIO_DIR: &str = "assets/scenarios";
const SCENARIO_INDEX_PATH: &str = "assets/scenarios/index.ron";

/// Scenario-level agent distribution modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentDistribution {
    /// Spread agents evenly across the world.
    Uniform,
    /// Spawn agents in a dense central cluster.
    Clustered,
    /// Spawn agents randomly.
    Random,
}

/// Agent spawn settings in a scenario file.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentSpawnConfig {
    /// Number of agents requested by the scenario.
    pub count: u32,
    /// Spatial distribution strategy.
    pub distribution: AgentDistribution,
}

/// Zone settings in a scenario file.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZoneConfig {
    /// Zone category.
    pub kind: ZoneKind,
    /// Center in X/Y/Z world space.
    pub center: Vec3,
    /// Radius in world units.
    pub radius: f32,
}

/// Resource spawn settings in a scenario file.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceSpawnConfig {
    /// Nodes spawned inside each resource zone.
    pub nodes_per_resource_zone: u32,
    /// Fraction of max supply each node starts with.
    pub initial_amount_fraction: f32,
    /// Regeneration units per simulation second.
    pub regen_rate: f32,
}

/// Simulation override settings in a scenario file.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimulationOverrides {
    /// Need decay scalar.
    pub global_decay_multiplier: f32,
    /// Resource regeneration scalar.
    pub global_regen_multiplier: f32,
}

/// Timed scenario events are parsed but executed in a later phase.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimedScenarioEvent {
    /// Simulation time for the event.
    pub at: f32,
    /// Event name or payload placeholder.
    pub kind: String,
}

/// Supported timed event kinds.
#[derive(Clone, Debug, PartialEq)]
pub enum TimedEventKind {
    /// Spawn agents near the center.
    SpawnAgents { count: u32 },
    /// Override global decay multiplier.
    SetDecayMultiplier(f32),
    /// Override global regen multiplier.
    SetRegenMultiplier(f32),
}

impl TimedScenarioEvent {
    /// Parse the event payload into a supported event kind.
    #[must_use]
    pub fn parsed_kind(&self) -> Option<TimedEventKind> {
        parse_timed_event_kind(&self.kind)
    }
}

/// Fully parsed scenario definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScenarioConfig {
    /// Display name.
    pub name: String,
    /// Human-readable purpose.
    pub description: String,
    /// Deterministic seed.
    pub seed: u64,
    /// X/Z world extents.
    pub world_size: Vec2,
    /// Agent spawn settings.
    pub agents: AgentSpawnConfig,
    /// Zone definitions.
    pub zones: Vec<ZoneConfig>,
    /// Resource spawn settings.
    pub resources: ResourceSpawnConfig,
    /// Simulation scalar overrides.
    pub sim_overrides: SimulationOverrides,
    /// Future timed events.
    pub events: Vec<TimedScenarioEvent>,
}

impl ScenarioConfig {
    /// Convert this scenario into the simulation-wide config resource.
    #[must_use]
    pub fn simulation_config(&self) -> SimulationConfig {
        let resource_zone_count = self
            .zones
            .iter()
            .filter(|zone| zone.kind == ZoneKind::Resource)
            .count() as u32;

        SimulationConfig {
            world_size: self.world_size,
            initial_agent_count: self.agents.count,
            initial_resource_count: resource_zone_count
                .saturating_mul(self.resources.nodes_per_resource_zone),
            needs_decay_rates: NeedsDecayRates {
                hunger_per_sec: 0.02,
                fatigue_per_sec: 0.015,
                energy_per_sec: 0.01,
            },
            spatial_grid_cell_size: 10.0,
            seed: self.seed,
            global_decay_multiplier: self.sim_overrides.global_decay_multiplier,
            global_regen_multiplier: self.sim_overrides.global_regen_multiplier,
            ..SimulationConfig::default()
        }
    }
}

/// Resource containing the active scenario.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ActiveScenario {
    /// Parsed scenario.
    pub config: ScenarioConfig,
}

/// Display label for the currently active scenario.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ActiveScenarioLabel {
    /// Short display name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
}

/// Registered scenario available in the selector.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioCatalogEntry {
    /// Stable scenario key from `index.ron`.
    pub key: String,
    /// Path to the scenario file.
    pub path: PathBuf,
    /// Parsed scenario.
    pub config: ScenarioConfig,
}

/// Runtime scenario catalog.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ScenarioCatalog {
    /// Available scenario entries.
    pub entries: Vec<ScenarioCatalogEntry>,
    /// Key of the active scenario.
    pub active_key: Option<String>,
}

/// Event requesting a scenario switch by key.
#[derive(Event, Clone, Debug, PartialEq)]
pub struct ScenarioLoadRequested {
    /// Scenario key from the catalog.
    pub key: String,
}

/// Tracks timed events that have already fired for the active scenario.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ScenarioEventState {
    /// Active scenario key associated with this state.
    pub scenario_key: Option<String>,
    /// Executed timed event indices.
    pub executed_indices: Vec<usize>,
    /// Next stable agent ID suffix for timed/player spawns.
    pub next_agent_id: u64,
}

/// Load a scenario from a RON file.
pub fn load_scenario_from_path(path: impl AsRef<Path>) -> anyhow::Result<ScenarioConfig> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("failed to read scenario {}: {error}", path.display()))?;
    ron::from_str(&content)
        .map_err(|error| anyhow::anyhow!("failed to parse scenario {}: {error}", path.display()))
}

/// Load all scenarios listed in the index file.
pub fn load_scenario_catalog_from_path(path: impl AsRef<Path>) -> anyhow::Result<ScenarioCatalog> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|error| {
        anyhow::anyhow!("failed to read scenario index {}: {error}", path.display())
    })?;
    let keys: Vec<String> = ron::from_str(&content).map_err(|error| {
        anyhow::anyhow!("failed to parse scenario index {}: {error}", path.display())
    })?;
    let entries = keys
        .into_iter()
        .map(|key| {
            let scenario_path = Path::new(SCENARIO_DIR).join(format!("{key}.ron"));
            let config = load_scenario_from_path(&scenario_path)?;
            Ok(ScenarioCatalogEntry {
                key,
                path: scenario_path,
                config,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(ScenarioCatalog {
        active_key: entries.first().map(|entry| entry.key.clone()),
        entries,
    })
}

fn active_scenario_label(key: &str, config: &ScenarioConfig) -> ActiveScenarioLabel {
    if let Some(preset) = ScenarioPreset::from_scenario_name(key) {
        ActiveScenarioLabel {
            name: preset.to_scenario_name().to_owned(),
            description: preset.label().to_owned(),
        }
    } else {
        ActiveScenarioLabel {
            name: config.name.clone(),
            description: config.description.clone(),
        }
    }
}

/// Load and install the default scenario resource.
pub fn load_default_scenario_system(
    mut commands: Commands,
    mut sim_config: ResMut<SimulationConfig>,
    mut sim_rng: ResMut<SimRng>,
) {
    match load_scenario_catalog_from_path(SCENARIO_INDEX_PATH) {
        Ok(mut catalog) => {
            let active = catalog
                .entries
                .iter()
                .find(|entry| entry.path == Path::new(DEFAULT_SCENARIO_PATH))
                .or_else(|| catalog.entries.first());

            if let Some(entry) = active {
                let config = entry.config.clone();
                *sim_config = config.simulation_config();
                sim_rng.reseed(config.seed);
                catalog.active_key = Some(entry.key.clone());
                commands.insert_resource(active_scenario_label(&entry.key, &config));
                commands.insert_resource(ActiveScenario { config });
                commands.insert_resource(ScenarioEventState {
                    scenario_key: Some(entry.key.clone()),
                    executed_indices: Vec::new(),
                    next_agent_id: u64::from(entry.config.agents.count),
                });
            }
            commands.insert_resource(catalog);
        },
        Err(error) => {
            error!("{error}");
            match load_scenario_from_path(DEFAULT_SCENARIO_PATH) {
                Ok(config) => {
                    *sim_config = config.simulation_config();
                    sim_rng.reseed(config.seed);
                    commands.insert_resource(ActiveScenarioLabel {
                        name: Path::new(DEFAULT_SCENARIO_PATH)
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or_default()
                            .to_owned(),
                        description: String::new(),
                    });
                    commands.insert_resource(ActiveScenario { config });
                },
                Err(error) => error!("{error}"),
            }
        },
    }
}

/// Apply requested scenario switches without restarting the app.
#[allow(clippy::too_many_arguments)]
pub fn apply_scenario_load_requests_system(
    mut commands: Commands,
    mut requests: EventReader<ScenarioLoadRequested>,
    mut catalog: ResMut<ScenarioCatalog>,
    mut active_scenario: ResMut<ActiveScenario>,
    mut sim_config: ResMut<SimulationConfig>,
    mut sim_rng: ResMut<SimRng>,
    mut sim_time: ResMut<crate::engine::SimulationTime>,
    spawned: Query<Entity, With<ScenarioSpawned>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut agent_spawned_events: EventWriter<AgentSpawned>,
) {
    for request in requests.read() {
        let Some(entry) = catalog
            .entries
            .iter()
            .find(|entry| entry.key == request.key)
            .cloned()
        else {
            warn!("unknown scenario requested: {}", request.key);
            continue;
        };

        for entity in &spawned {
            commands.entity(entity).despawn_recursive();
        }

        let config = entry.config.clone();
        *sim_config = config.simulation_config();
        sim_rng.reseed(config.seed);
        *sim_time = crate::engine::SimulationTime::new();
        active_scenario.config = config.clone();
        catalog.active_key = Some(entry.key.clone());
        commands.insert_resource(active_scenario_label(&entry.key, &config));
        commands.insert_resource(ScenarioEventState {
            scenario_key: Some(entry.key),
            executed_indices: Vec::new(),
            next_agent_id: u64::from(config.agents.count),
        });
        spawn_scenario_world(
            &mut commands,
            &config,
            &sim_config,
            &mut meshes,
            &mut materials,
            &mut agent_spawned_events,
        );
    }
}

/// Execute timed events from the active scenario.
pub fn timed_scenario_events_system(
    mut commands: Commands,
    sim_time: Res<crate::engine::SimulationTime>,
    active_scenario: Option<Res<ActiveScenario>>,
    mut event_state: ResMut<ScenarioEventState>,
    mut sim_config: ResMut<SimulationConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut agent_spawned_events: EventWriter<AgentSpawned>,
) {
    let Some(active_scenario) = active_scenario else {
        return;
    };
    if sim_time.paused {
        return;
    }

    for (index, event) in active_scenario.config.events.iter().enumerate() {
        if event_state.executed_indices.contains(&index) || sim_time.elapsed < event.at {
            continue;
        }

        match event.parsed_kind() {
            Some(TimedEventKind::SpawnAgents { count }) => {
                let center = Vec3::new(
                    sim_config.world_size.x * 0.5,
                    sim_config.agent_visual_height,
                    sim_config.world_size.y * 0.5,
                );
                for offset in 0..count {
                    let id = AgentId(event_state.next_agent_id);
                    event_state.next_agent_id = event_state.next_agent_id.saturating_add(1);
                    spawn_agent_at_position(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        id,
                        center + Vec3::X * offset as f32,
                        &sim_config,
                        &mut agent_spawned_events,
                    );
                }
            },
            Some(TimedEventKind::SetDecayMultiplier(value)) => {
                sim_config.global_decay_multiplier = value;
            },
            Some(TimedEventKind::SetRegenMultiplier(value)) => {
                sim_config.global_regen_multiplier = value;
            },
            None => warn!("unsupported timed scenario event: {}", event.kind),
        }

        event_state.executed_indices.push(index);
    }
}

/// Spawn an agent at the cursor ground point on right click.
#[allow(clippy::too_many_arguments)]
pub fn player_spawn_agent_system(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    sim_config: Res<SimulationConfig>,
    mut event_state: ResMut<ScenarioEventState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut agent_spawned_events: EventWriter<AgentSpawned>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Right) {
        return;
    }

    let Some(point) = cursor_ground_point(&windows, &camera_query) else {
        return;
    };

    let id = AgentId(event_state.next_agent_id);
    event_state.next_agent_id = event_state.next_agent_id.saturating_add(1);
    spawn_agent_at_position(
        &mut commands,
        &mut meshes,
        &mut materials,
        id,
        point,
        &sim_config,
        &mut agent_spawned_events,
    );
}

fn cursor_ground_point(
    windows: &Query<&Window>,
    camera_query: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec3> {
    let window = windows.get_single().ok()?;
    let cursor_position = window.cursor_position()?;
    let (camera, camera_transform) = camera_query.get_single().ok()?;
    let ray = camera
        .viewport_to_world(camera_transform, cursor_position)
        .ok()?;
    let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
    Some(ray.get_point(distance))
}

fn parse_timed_event_kind(input: &str) -> Option<TimedEventKind> {
    let trimmed = input.trim();
    parse_u32_arg(trimmed, "SpawnAgents")
        .map(|count| TimedEventKind::SpawnAgents { count })
        .or_else(|| {
            parse_f32_arg(trimmed, "SetDecayMultiplier").map(TimedEventKind::SetDecayMultiplier)
        })
        .or_else(|| {
            parse_f32_arg(trimmed, "SetRegenMultiplier").map(TimedEventKind::SetRegenMultiplier)
        })
}

fn parse_u32_arg(input: &str, name: &str) -> Option<u32> {
    parse_arg(input, name)?.parse().ok()
}

fn parse_f32_arg(input: &str, name: &str) -> Option<f32> {
    parse_arg(input, name)?.parse().ok()
}

fn parse_arg<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    input
        .strip_prefix(name)?
        .trim()
        .strip_prefix('(')?
        .strip_suffix(')')
        .map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios::presets::ScenarioPreset;

    #[test]
    fn parses_equilibrium_scenario() {
        let scenario: ScenarioConfig =
            ron::from_str(include_str!("../../assets/scenarios/equilibrium.ron"))
                .expect("equilibrium scenario should parse");

        assert_eq!(scenario.name, "Stable Equilibrium");
        assert_eq!(scenario.zones.len(), 5);
        assert_eq!(scenario.simulation_config().initial_resource_count, 9);
    }

    #[test]
    fn parses_timed_event_payloads() {
        assert_eq!(
            parse_timed_event_kind("SpawnAgents(5)"),
            Some(TimedEventKind::SpawnAgents { count: 5 })
        );
        assert_eq!(
            parse_timed_event_kind("SetDecayMultiplier(1.3)"),
            Some(TimedEventKind::SetDecayMultiplier(1.3))
        );
    }

    #[test]
    fn scenario_catalog_contains_all_hotkey_presets() {
        let catalog = load_scenario_catalog_from_path(SCENARIO_INDEX_PATH)
            .expect("scenario catalog should load");

        for preset in ScenarioPreset::all() {
            assert!(
                catalog
                    .entries
                    .iter()
                    .any(|entry| entry.key == preset.to_scenario_name()),
                "missing scenario preset {}",
                preset.to_scenario_name()
            );
        }
    }
}
