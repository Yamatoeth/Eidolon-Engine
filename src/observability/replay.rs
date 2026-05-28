//! Simulation recording and playback system.

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

use crate::engine::SimulationTime;
use crate::observability::inspector::ObservabilityConfig;
use crate::scenarios::loader::{ScenarioCatalog, ScenarioLoadRequested};
use crate::simulation::{Agent, AgentId, AgentState, Needs, ResourceNode, StateKind};

/// Snapshot of one agent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AgentSnapshot {
    /// Stable agent ID.
    pub id: AgentId,
    /// Agent position.
    pub position: Vec3,
    /// Agent needs.
    pub needs: Needs,
    /// Agent state.
    pub state: StateKind,
}

/// Snapshot of one resource node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResourceSnapshot {
    /// Resource entity at capture time.
    pub entity: Entity,
    /// Current resource amount.
    pub amount: f32,
    /// Whether the resource was depleted.
    pub is_depleted: bool,
}

/// One recorded replay frame.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayFrame {
    /// Simulation tick.
    pub tick: u64,
    /// Simulation timestamp.
    pub elapsed: f32,
    /// Agent snapshots.
    pub agents: Vec<AgentSnapshot>,
    /// Resource snapshots.
    pub resources: Vec<ResourceSnapshot>,
}

/// Replay recording buffer and playback cursor.
#[derive(Resource, Debug)]
pub struct ReplayBuffer {
    /// Recorded frames.
    pub frames: Vec<ReplayFrame>,
    /// Whether recording is active.
    pub is_recording: bool,
    /// Whether playback mode is active.
    pub is_replaying: bool,
    /// Current playback cursor.
    pub playback_index: usize,
    /// Last tick recorded.
    pub last_recorded_tick: u64,
}

impl Default for ReplayBuffer {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            is_recording: true,
            is_replaying: false,
            playback_index: 0,
            last_recorded_tick: 0,
        }
    }
}

/// Record periodic simulation snapshots.
pub fn replay_record_system(
    sim_time: Res<SimulationTime>,
    config: Res<ObservabilityConfig>,
    mut replay: ResMut<ReplayBuffer>,
    agents: Query<(&Agent, &Transform, &Needs, &AgentState)>,
    resources: Query<(Entity, &ResourceNode)>,
) {
    if sim_time.paused || !replay.is_recording || replay.is_replaying {
        return;
    }

    let interval = u64::from(config.replay_record_interval_ticks.max(1));
    if sim_time.tick.saturating_sub(replay.last_recorded_tick) < interval {
        return;
    }

    replay.frames.push(ReplayFrame {
        tick: sim_time.tick,
        elapsed: sim_time.elapsed,
        agents: agents
            .iter()
            .map(|(agent, transform, needs, state)| AgentSnapshot {
                id: agent.id,
                position: transform.translation,
                needs: *needs,
                state: state.current,
            })
            .collect(),
        resources: resources
            .iter()
            .map(|(entity, resource)| ResourceSnapshot {
                entity,
                amount: resource.amount,
                is_depleted: resource.is_depleted,
            })
            .collect(),
    });
    replay.last_recorded_tick = sim_time.tick;
    replay.playback_index = replay.frames.len().saturating_sub(1);

    const MAX_REPLAY_FRAMES: usize = 2_000;
    if replay.frames.len() > MAX_REPLAY_FRAMES {
        let overflow = replay.frames.len() - MAX_REPLAY_FRAMES;
        replay.frames.drain(0..overflow);
        replay.playback_index = replay.playback_index.saturating_sub(overflow);
    }
}

/// Draw replay controls.
pub fn replay_ui_system(
    mut contexts: EguiContexts,
    mut replay: ResMut<ReplayBuffer>,
    catalog: Option<Res<ScenarioCatalog>>,
    mut scenario_load_requests: EventWriter<ScenarioLoadRequested>,
) {
    egui::Window::new("Replay Controls")
        .default_width(420.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(if replay.is_recording {
                        "Stop Rec"
                    } else {
                        "Record"
                    })
                    .clicked()
                {
                    replay.is_recording = !replay.is_recording;
                }
                if ui
                    .button(if replay.is_replaying {
                        "Live"
                    } else {
                        "Replay"
                    })
                    .clicked()
                {
                    replay.is_replaying = !replay.is_replaying;
                }
                if ui.button("Clear").clicked() {
                    replay.frames.clear();
                    replay.playback_index = 0;
                }
                if ui.button("Seed Replay").clicked() {
                    if let Some(active_key) = catalog
                        .as_deref()
                        .and_then(|catalog| catalog.active_key.as_ref())
                    {
                        scenario_load_requests.send(ScenarioLoadRequested {
                            key: active_key.clone(),
                        });
                    }
                }
            });

            let frame_count = replay.frames.len();
            ui.label(format!(
                "Frames: {frame_count}  Recording: {}  Replay: {}",
                replay.is_recording, replay.is_replaying
            ));

            if frame_count > 0 {
                let max_index = frame_count - 1;
                ui.add(egui::Slider::new(&mut replay.playback_index, 0..=max_index).text("frame"));
                if let Some(frame) = replay.frames.get(replay.playback_index) {
                    ui.label(format!(
                        "Frame tick={} t={:.2}s agents={} resources={}",
                        frame.tick,
                        frame.elapsed,
                        frame.agents.len(),
                        frame.resources.len()
                    ));
                }
            }
        });
}
