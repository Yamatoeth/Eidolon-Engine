//! Live entity/component browser with egui.

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

use crate::ai::{AIDebugInfo, DecisionOutput};
use crate::engine::{EngineAction, EngineActionEvent, SimulationTime};
use crate::simulation::{Agent, AgentState, Needs, ResourceNode, SimulationMetrics, Zone};

type AgentInspectorItem<'a> = (
    &'a Agent,
    &'a Needs,
    &'a AgentState,
    Option<&'a DecisionOutput>,
    Option<&'a AIDebugInfo>,
);

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
    metrics: Res<SimulationMetrics>,
    zones: Query<&Zone>,
    resources: Query<&ResourceNode>,
    agents: Query<AgentInspectorItem<'_>>,
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
            ui.heading("Agents");
            ui.label(format!("Live agents: {}", metrics.agent_count));
            ui.label(format!("Average hunger: {:.2}", metrics.avg_hunger));
            ui.label(format!("Average fatigue: {:.2}", metrics.avg_fatigue));
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .show(ui, |ui| {
                    for (agent, needs, state, decision, debug) in agents.iter().take(64) {
                        ui.group(|ui| {
                            ui.label(format!(
                                "Agent #{:03}  {:?}  age {:.1}s",
                                agent.id.0, state.current, agent.age
                            ));
                            if let Some(decision) = decision {
                                ui.label(format!(
                                    "Decision: {:?} score {:.2}",
                                    decision.action, decision.score
                                ));
                            }
                            ui.add(
                                egui::ProgressBar::new(needs.hunger)
                                    .text(format!("hunger {:.2}", needs.hunger)),
                            );
                            ui.add(
                                egui::ProgressBar::new(needs.fatigue)
                                    .text(format!("fatigue {:.2}", needs.fatigue)),
                            );
                            ui.add(
                                egui::ProgressBar::new(needs.energy)
                                    .text(format!("energy {:.2}", needs.energy)),
                            );
                            if let Some(debug) = debug {
                                for (action, score) in debug.last_scores.iter().take(5) {
                                    ui.add(
                                        egui::ProgressBar::new(*score)
                                            .text(format!("{action:?} {score:.2}")),
                                    );
                                }
                            }
                        });
                    }
                });
            ui.separator();
            ui.heading("Static World");
            ui.label(format!("Zones: {}", zones.iter().count()));
            ui.label(format!("Resource nodes: {}", resources.iter().count()));
        });
}
