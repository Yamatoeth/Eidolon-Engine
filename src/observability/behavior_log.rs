//! JSON export for agent behavior traces.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::Serialize;

use crate::ai::{ActionKind, AgentBehaviorLogged};
use crate::engine::SimulationTime;
use crate::observability::inspector::ObservabilityConfig;
use crate::simulation::{Agent, StateKind};

const DEFAULT_BEHAVIOR_LOG_PATH: &str = "logs/agent_behavior.jsonl";

/// Runtime destination for behavior JSON lines.
#[derive(Resource, Debug, Clone)]
pub struct BehaviorLogExport {
    /// Output file path.
    pub path: PathBuf,
}

impl Default for BehaviorLogExport {
    fn default() -> Self {
        Self {
            path: PathBuf::from(DEFAULT_BEHAVIOR_LOG_PATH),
        }
    }
}

/// Append behavior changes as newline-delimited JSON.
pub fn behavior_log_export_system(
    sim_time: Res<SimulationTime>,
    config: Res<ObservabilityConfig>,
    export: Res<BehaviorLogExport>,
    agents: Query<&Agent>,
    mut behavior_events: EventReader<AgentBehaviorLogged>,
) {
    if !config.behavior_log_export_enabled {
        behavior_events.clear();
        return;
    }

    let events = behavior_events.read().cloned().collect::<Vec<_>>();
    if events.is_empty() {
        return;
    }

    if let Some(parent) = export.path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            warn!(
                "failed to create behavior log directory {}: {error}",
                parent.display()
            );
            return;
        }
    }

    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&export.path)
    {
        Ok(file) => file,
        Err(error) => {
            warn!(
                "failed to open behavior log {}: {error}",
                export.path.display()
            );
            return;
        },
    };

    for event in &events {
        write_behavior_event(&mut file, &sim_time, &agents, event);
    }
}

fn write_behavior_event(
    file: &mut std::fs::File,
    sim_time: &SimulationTime,
    agents: &Query<&Agent>,
    event: &AgentBehaviorLogged,
) {
    let entry = BehaviorLogEntry::from_event(sim_time, agents.get(event.agent).ok(), event);
    if let Err(error) = serde_json::to_writer(&mut *file, &entry) {
        warn!("failed to serialize behavior log entry: {error}");
        return;
    }
    if let Err(error) = writeln!(file) {
        warn!("failed to write behavior log entry: {error}");
    }
}

#[derive(Serialize)]
struct BehaviorLogEntry {
    tick: u64,
    elapsed: f32,
    agent_entity: String,
    agent_id: Option<u64>,
    previous_action: &'static str,
    action: &'static str,
    previous_intent: String,
    intent: String,
    state: &'static str,
    target_entity: Option<String>,
    target_position: Option<JsonVec3>,
    score: f32,
    needs: JsonNeeds,
    scores: Vec<JsonActionScore>,
}

impl BehaviorLogEntry {
    fn from_event(
        sim_time: &SimulationTime,
        agent: Option<&Agent>,
        event: &AgentBehaviorLogged,
    ) -> Self {
        Self {
            tick: sim_time.tick,
            elapsed: sim_time.elapsed,
            agent_entity: format!("{:?}", event.agent),
            agent_id: agent.map(|agent| agent.id.0),
            previous_action: action_label(event.previous_action),
            action: action_label(event.action),
            previous_intent: format!("{:?}", event.previous_intent),
            intent: format!("{:?}", event.intent),
            state: state_label(event.state),
            target_entity: event.target.map(|target| format!("{target:?}")),
            target_position: event.target_position.map(JsonVec3::from),
            score: event.score,
            needs: JsonNeeds {
                hunger: event.hunger,
                fatigue: event.fatigue,
                energy: event.energy,
            },
            scores: event
                .scores
                .iter()
                .map(|(action, score)| JsonActionScore {
                    action: action_label(*action),
                    score: *score,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct JsonVec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl From<Vec3> for JsonVec3 {
    fn from(value: Vec3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

#[derive(Serialize)]
struct JsonNeeds {
    hunger: f32,
    fatigue: f32,
    energy: f32,
}

#[derive(Serialize)]
struct JsonActionScore {
    action: &'static str,
    score: f32,
}

fn action_label(action: ActionKind) -> &'static str {
    match action {
        ActionKind::Idle => "idle",
        ActionKind::MoveTo => "move_to",
        ActionKind::Eat => "eat",
        ActionKind::Rest => "rest",
        ActionKind::Deliver => "deliver",
        ActionKind::Explore => "explore",
        ActionKind::Collect => "collect",
    }
}

fn state_label(state: StateKind) -> &'static str {
    match state {
        StateKind::Idle => "idle",
        StateKind::MovingToTarget => "moving_to_target",
        StateKind::Eating => "eating",
        StateKind::Resting => "resting",
        StateKind::Carrying => "carrying",
        StateKind::Exploring => "exploring",
        StateKind::Fleeing => "fleeing",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::AgentIntent;
    use tempfile::tempdir;

    #[test]
    fn behavior_log_export_writes_json_line() {
        let tempdir = tempdir().expect("tempdir should be created");
        let log_path = tempdir.path().join("behavior.jsonl");
        let mut app = App::new();
        app.init_resource::<SimulationTime>()
            .init_resource::<ObservabilityConfig>()
            .insert_resource(BehaviorLogExport {
                path: log_path.clone(),
            })
            .add_event::<AgentBehaviorLogged>()
            .add_systems(Update, behavior_log_export_system);

        let agent = app
            .world_mut()
            .spawn(Agent {
                id: crate::simulation::AgentId(7),
                age: 0.0,
            })
            .id();
        app.world_mut().send_event(AgentBehaviorLogged {
            agent,
            previous_action: ActionKind::Idle,
            action: ActionKind::Deliver,
            previous_intent: AgentIntent::Idle,
            intent: AgentIntent::Deliver { zone: agent },
            state: StateKind::Carrying,
            target: Some(agent),
            target_position: Some(Vec3::new(1.0, 0.0, 2.0)),
            score: 0.9,
            hunger: 0.2,
            fatigue: 0.3,
            energy: 0.8,
            scores: vec![(ActionKind::Deliver, 0.9), (ActionKind::Idle, 0.1)],
        });

        app.update();

        let content = std::fs::read_to_string(log_path).expect("log should be written");
        let json: serde_json::Value =
            serde_json::from_str(content.trim()).expect("log line should be valid json");
        assert_eq!(json["agent_id"], 7);
        assert_eq!(json["previous_action"], "idle");
        assert_eq!(json["action"], "deliver");
        assert_eq!(json["state"], "carrying");
        assert_eq!(json["scores"][0]["action"], "deliver");
    }
}
