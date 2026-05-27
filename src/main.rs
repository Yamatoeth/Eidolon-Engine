// emergent-sim — main entry point

use bevy::prelude::*;
use emergent_sim::{
    AIPlugin, EnginePlugin, ObservabilityPlugin, ScenariosPlugin, SimulationPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EnginePlugin)
        .add_plugins(SimulationPlugin)
        .add_plugins(AIPlugin)
        .add_plugins(ObservabilityPlugin)
        .add_plugins(ScenariosPlugin)
        .run();
}
