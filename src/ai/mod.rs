//! AI Layer — Utility scoring, decision systems
//!
//! Responsible for agent decision-making. Consumes simulation state, produces decisions.

use bevy::prelude::*;

pub mod actions;
pub mod decision;
pub mod memory;
pub mod utility;

pub use actions::{ActionKind, ActionScore};
pub use decision::{
    AIDebugInfo, AgentIntent, AgentRole, DecisionOutput, PerceptionData, VisibleResource,
    VisibleZone,
};
pub use memory::{AgentMemory, KnownResource, KnownRestZone};
pub use utility::{AIConfig, UtilityWeights};

const DEFAULT_AI_CONFIG_PATH: &str = "assets/config/ai.ron";

/// Utility AI plugin.
pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AIConfig>()
            .add_systems(Startup, load_ai_config_system)
            .add_systems(
                FixedUpdate,
                (
                    decision::attach_ai_components_system,
                    memory::update_agent_memory_system,
                    memory::share_agent_memory_system,
                    decision::ai_scoring_system,
                )
                    .chain()
                    .after(crate::engine::time::update_simulation_time)
                    .after(crate::simulation::spatial::spatial_grid_update_system)
                    .before(crate::simulation::agent::agent_state_transition_system),
            );
    }
}

/// Load AI config from the default RON asset path.
pub fn load_ai_config_system(mut config: ResMut<AIConfig>) {
    match std::fs::read_to_string(DEFAULT_AI_CONFIG_PATH) {
        Ok(content) => match ron::from_str::<AIConfig>(&content) {
            Ok(parsed) => *config = parsed,
            Err(error) => error!("failed to parse {DEFAULT_AI_CONFIG_PATH}: {error}"),
        },
        Err(error) => error!("failed to read {DEFAULT_AI_CONFIG_PATH}: {error}"),
    }
}
