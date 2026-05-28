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

pub use agent::{
    Agent, AgentId, AgentState, Needs, SimRng, SimulationMetrics, StateKind, Velocity,
};
pub use events::{
    AgentDied, AgentSpawned, DeathCause, NeedKind, NeedThresholdReached, ResourceConsumed,
    ResourceDepleted, ResourceReplenished, ThresholdLevel,
};
pub use resource::{ResourceKind, ResourceNode};
pub use spatial::{Collider, GridCell, SpatialGrid};
pub use world::{NeedsDecayRates, SimulationConfig, Zone, ZoneId, ZoneKind};

/// Core simulation plugin for world resources and deterministic spatial updates.
pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationConfig>()
            .init_resource::<SimRng>()
            .init_resource::<SpatialGrid>()
            .init_resource::<SimulationMetrics>()
            .add_event::<NeedThresholdReached>()
            .add_event::<AgentDied>()
            .add_event::<AgentSpawned>()
            .add_event::<ResourceConsumed>()
            .add_event::<ResourceDepleted>()
            .add_event::<ResourceReplenished>()
            .add_systems(
                FixedUpdate,
                (
                    agent::needs_decay_system,
                    resource::resource_regen_system,
                    spatial::spatial_grid_update_system,
                    agent::agent_state_transition_system,
                    agent::agent_movement_system,
                    resource::resource_consume_system,
                    resource::rest_recovery_system,
                    agent::agent_death_system,
                    agent::metrics_update_system,
                )
                    .chain(),
            );
    }
}
