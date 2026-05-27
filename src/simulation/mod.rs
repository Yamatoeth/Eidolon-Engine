//! Simulation Layer — World logic, agents, resources, events
//!
//! Responsible for all world logic. Must be renderable independently (headless capable).

use bevy::prelude::*;

pub mod agent;
pub mod events;
pub mod resource;
pub mod rules;
pub mod spatial;
pub mod world;

pub use resource::{ResourceKind, ResourceNode};
pub use spatial::{Collider, GridCell, SpatialGrid};
pub use world::{NeedsDecayRates, SimulationConfig, Zone, ZoneId, ZoneKind};

/// Core simulation plugin for world resources and deterministic spatial updates.
pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationConfig>()
            .init_resource::<SpatialGrid>()
            .add_systems(FixedUpdate, spatial::spatial_grid_update_system);
    }
}
