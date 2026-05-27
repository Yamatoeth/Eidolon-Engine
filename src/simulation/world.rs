//! World initialization and zone definitions.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Stable identifier for a zone entity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ZoneId(pub u64);

/// A named world area with behavior-relevant semantics.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    /// Stable zone ID assigned by scenario load order.
    pub id: ZoneId,
    /// Behavior category for the zone.
    pub kind: ZoneKind,
    /// Radius in world units.
    pub radius: f32,
}

/// Semantic zone categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ZoneKind {
    /// Area that contains resource nodes.
    Resource,
    /// Area where agents can recover fatigue.
    Rest,
    /// Default passable area.
    Neutral,
    /// Future dangerous area.
    Hazard,
}

/// Need decay rates per simulation second.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NeedsDecayRates {
    /// Hunger increase per simulation second.
    pub hunger_per_sec: f32,
    /// Fatigue increase per simulation second.
    pub fatigue_per_sec: f32,
    /// Energy decrease per simulation second.
    pub energy_per_sec: f32,
}

/// Master simulation configuration derived from scenario/config assets.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// X/Z world extents.
    pub world_size: Vec2,
    /// Initial agents requested by the scenario.
    pub initial_agent_count: u32,
    /// Initial resource nodes requested by the scenario.
    pub initial_resource_count: u32,
    /// Need decay rates.
    pub needs_decay_rates: NeedsDecayRates,
    /// Uniform spatial grid cell size.
    pub spatial_grid_cell_size: f32,
    /// Deterministic simulation seed.
    pub seed: u64,
    /// Scenario-wide need decay scalar.
    pub global_decay_multiplier: f32,
    /// Scenario-wide resource regeneration scalar.
    pub global_regen_multiplier: f32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            world_size: Vec2::new(100.0, 100.0),
            initial_agent_count: 0,
            initial_resource_count: 0,
            needs_decay_rates: NeedsDecayRates {
                hunger_per_sec: 0.02,
                fatigue_per_sec: 0.015,
                energy_per_sec: 0.01,
            },
            spatial_grid_cell_size: 10.0,
            seed: 42,
            global_decay_multiplier: 1.0,
            global_regen_multiplier: 1.0,
        }
    }
}
