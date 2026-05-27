//! Window configuration and setup

use bevy::prelude::*;

/// Window configuration constants
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 720.0;
pub const WINDOW_TITLE: &str = "emergent-sim — 3D ECS Simulation Engine";

/// Configure window properties
pub fn setup_window(mut windows: Query<&mut Window>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.title = WINDOW_TITLE.to_string();
        window.resolution.set(WINDOW_WIDTH, WINDOW_HEIGHT);
    }
}
