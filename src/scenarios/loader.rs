//! Loads scenario definitions from RON assets.

use std::fs;
use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::simulation::{NeedsDecayRates, SimulationConfig, ZoneKind};

/// Default scenario loaded at startup.
pub const DEFAULT_SCENARIO_PATH: &str = "assets/scenarios/equilibrium.ron";

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
        }
    }
}

/// Resource containing the active scenario.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ActiveScenario {
    /// Parsed scenario.
    pub config: ScenarioConfig,
}

/// Load a scenario from a RON file.
pub fn load_scenario_from_path(path: impl AsRef<Path>) -> anyhow::Result<ScenarioConfig> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("failed to read scenario {}: {error}", path.display()))?;
    ron::from_str(&content)
        .map_err(|error| anyhow::anyhow!("failed to parse scenario {}: {error}", path.display()))
}

/// Load and install the default scenario resource.
pub fn load_default_scenario_system(
    mut commands: Commands,
    mut sim_config: ResMut<SimulationConfig>,
) {
    match load_scenario_from_path(DEFAULT_SCENARIO_PATH) {
        Ok(config) => {
            *sim_config = config.simulation_config();
            commands.insert_resource(ActiveScenario { config });
        },
        Err(error) => {
            error!("{error}");
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_equilibrium_scenario() {
        let scenario: ScenarioConfig =
            ron::from_str(include_str!("../../assets/scenarios/equilibrium.ron"))
                .expect("equilibrium scenario should parse");

        assert_eq!(scenario.name, "Stable Equilibrium");
        assert_eq!(scenario.zones.len(), 5);
        assert_eq!(scenario.simulation_config().initial_resource_count, 9);
    }
}
