//! AI Layer — Utility scoring, decision systems
//!
//! Responsible for agent decision-making. Consumes simulation state, produces decisions.

use bevy::prelude::*;

pub mod actions;
pub mod decision;
pub mod memory;
pub mod utility;

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: Register AI systems and resources
    }
}
