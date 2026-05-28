//! Scenario Layer — World configs, presets, simulation scripts
//!
//! Responsible for simulation configurations. Purely data + setup code.

use bevy::prelude::*;

pub mod builder;
pub mod loader;
pub mod presets;

/// Scenario plugin that loads the default world and spawns static entities.
pub struct ScenariosPlugin;

impl Plugin for ScenariosPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, loader::load_default_scenario_system)
            .add_systems(PostStartup, builder::spawn_active_scenario_system)
            .add_systems(Update, builder::agent_visual_state_system);
    }
}
