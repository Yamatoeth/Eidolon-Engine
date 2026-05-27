//! Scenario Layer — World configs, presets, simulation scripts
//!
//! Responsible for simulation configurations. Purely data + setup code.

use bevy::prelude::*;

pub mod builder;
pub mod loader;
pub mod presets;

pub struct ScenariosPlugin;

impl Plugin for ScenariosPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: Register scenario systems
    }
}
