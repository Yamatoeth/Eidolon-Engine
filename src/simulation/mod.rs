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

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: Register simulation systems and resources
    }
}
