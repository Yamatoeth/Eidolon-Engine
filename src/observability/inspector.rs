//! Live entity/component browser with egui.

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

use crate::engine::{EngineAction, EngineActionEvent, SimulationTime};

/// Runtime configuration for Phase 1 observability panels.
#[derive(Resource, Debug, Clone)]
pub struct ObservabilityConfig {
    /// Whether the entity inspector shell is visible.
    pub inspector_open: bool,
    /// Whether the future event timeline panel is visible.
    pub timeline_open: bool,
    /// Whether in-world debug overlays are enabled.
    pub overlays_enabled: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            inspector_open: false,
            timeline_open: false,
            overlays_enabled: true,
        }
    }
}

/// Apply observability-owned engine actions to observability resources.
pub fn handle_observability_actions(
    mut events: EventReader<EngineActionEvent>,
    mut config: ResMut<ObservabilityConfig>,
) {
    for event in events.read() {
        if event.action == EngineAction::ToggleInspector {
            config.inspector_open = !config.inspector_open;
        }
    }
}

/// Draw the Phase 1 inspector shell.
pub fn inspector_ui_system(
    mut contexts: EguiContexts,
    mut config: ResMut<ObservabilityConfig>,
    sim_time: Res<SimulationTime>,
) {
    if !config.inspector_open {
        return;
    }

    egui::Window::new("Entity Inspector")
        .open(&mut config.inspector_open)
        .default_width(320.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Simulation");
            ui.separator();
            ui.label(format!("Tick: {}", sim_time.tick));
            ui.label(format!("Elapsed: {:.2}s", sim_time.elapsed));
            ui.label(format!(
                "State: {}",
                if sim_time.paused { "Paused" } else { "Running" }
            ));
            ui.separator();
            ui.label("Entity list will be populated when simulation entities exist.");
        });
}
