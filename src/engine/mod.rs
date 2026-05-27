//! Engine Layer — Rendering, input, camera, ECS loop
//!
//! Responsible for all technical runtime concerns. Has no knowledge of simulation semantics.

use bevy::prelude::*;

pub mod camera;
pub mod input;
pub mod render;
pub mod time;
pub mod window;

pub use camera::OrbitCamera;
pub use input::{EngineAction, EngineActionEvent, InputMap};
pub use time::SimulationTime;

/// Runtime systems for rendering, input, camera control, and simulation timing.
pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationTime>()
            .init_resource::<InputMap>()
            .init_resource::<render::DebugGridConfig>()
            .insert_resource(Time::<Fixed>::from_seconds(f64::from(time::FIXED_TIMESTEP)))
            .add_event::<EngineActionEvent>()
            .add_systems(
                Startup,
                (
                    window::setup_window,
                    render::spawn_scene,
                    camera::spawn_orbit_camera,
                ),
            )
            .add_systems(
                Update,
                (
                    input::handle_keyboard_input,
                    input::apply_engine_actions,
                    camera::handle_camera_input,
                    render::draw_debug_grid_system,
                ),
            )
            .add_systems(FixedUpdate, time::update_simulation_time);
    }
}
