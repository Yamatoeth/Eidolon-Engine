//! Event history and chronological visualization.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

use crate::ai::{ActionKind, AgentBehaviorLogged};
use crate::engine::SimulationTime;
use crate::observability::inspector::ObservabilityConfig;
use crate::simulation::{
    AgentDied, AgentSpawned, NeedThresholdReached, ResourceConsumed, ResourceDelivered,
    ResourceDepleted, ResourceReplenished,
};

/// Timeline event category filter.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TimelineFilter {
    /// Show all events.
    #[default]
    All,
    /// Agent lifecycle and need events.
    Agents,
    /// Agent AI behavior decisions.
    Behavior,
    /// Resource consumption and regen events.
    Resources,
}

/// Flattened event category stored by the timeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimelineEventKind {
    /// Agent was spawned.
    AgentSpawned,
    /// Agent died.
    AgentDied,
    /// Need threshold was crossed.
    NeedThreshold,
    /// Agent changed AI behavior.
    AgentBehavior,
    /// Resource was consumed.
    ResourceConsumed,
    /// Resource was delivered to a village store.
    ResourceDelivered,
    /// Resource was depleted.
    ResourceDepleted,
    /// Resource was replenished.
    ResourceReplenished,
}

/// One timeline entry.
#[derive(Clone, Debug)]
pub struct TimelineEvent {
    /// Simulation timestamp.
    pub timestamp: f32,
    /// Simulation tick.
    pub tick: u64,
    /// Event category.
    pub kind: TimelineEventKind,
    /// Involved entities.
    pub entities: Vec<Entity>,
    /// Display summary.
    pub summary: String,
}

/// Rolling event timeline.
#[derive(Resource, Debug)]
pub struct EventTimeline {
    /// Stored events.
    pub events: VecDeque<TimelineEvent>,
    /// Maximum retained events.
    pub max_entries: usize,
    /// Current filter.
    pub filter: TimelineFilter,
}

impl Default for EventTimeline {
    fn default() -> Self {
        Self {
            events: VecDeque::new(),
            max_entries: 1_000,
            filter: TimelineFilter::All,
        }
    }
}

impl EventTimeline {
    /// Push a new event and trim the rolling buffer.
    pub fn push(&mut self, event: TimelineEvent) {
        self.events.push_back(event);
        while self.events.len() > self.max_entries {
            self.events.pop_front();
        }
    }
}

/// Mirror agent simulation events into the rolling timeline.
pub fn timeline_record_agents_system(
    sim_time: Res<SimulationTime>,
    config: Res<ObservabilityConfig>,
    mut timeline: ResMut<EventTimeline>,
    mut agent_spawned: EventReader<AgentSpawned>,
    mut agent_died: EventReader<AgentDied>,
    mut need_thresholds: EventReader<NeedThresholdReached>,
) {
    timeline.max_entries = config.timeline_max_entries;

    for event in agent_spawned.read() {
        timeline.push(entry(
            &sim_time,
            TimelineEventKind::AgentSpawned,
            vec![event.agent],
            format!("Agent spawned at {}", format_position(event.position)),
        ));
    }
    for event in agent_died.read() {
        timeline.push(entry(
            &sim_time,
            TimelineEventKind::AgentDied,
            vec![event.agent],
            format!("Agent died: {:?}", event.cause),
        ));
    }
    for event in need_thresholds.read() {
        timeline.push(entry(
            &sim_time,
            TimelineEventKind::NeedThreshold,
            vec![event.agent],
            format!("{:?} reached {:?}", event.need, event.level),
        ));
    }
}

/// Mirror AI behavior decisions into the rolling timeline.
pub fn timeline_record_behavior_system(
    sim_time: Res<SimulationTime>,
    config: Res<ObservabilityConfig>,
    mut timeline: ResMut<EventTimeline>,
    mut behavior_events: EventReader<AgentBehaviorLogged>,
) {
    timeline.max_entries = config.timeline_max_entries;

    for event in behavior_events.read() {
        timeline.push(entry(
            &sim_time,
            TimelineEventKind::AgentBehavior,
            vec![event.agent],
            format_behavior_summary(event),
        ));
    }
}

/// Mirror resource simulation events into the rolling timeline.
pub fn timeline_record_resources_system(
    sim_time: Res<SimulationTime>,
    config: Res<ObservabilityConfig>,
    mut timeline: ResMut<EventTimeline>,
    mut resource_consumed: EventReader<ResourceConsumed>,
    mut resource_delivered: EventReader<ResourceDelivered>,
    mut resource_depleted: EventReader<ResourceDepleted>,
    mut resource_replenished: EventReader<ResourceReplenished>,
) {
    timeline.max_entries = config.timeline_max_entries;

    for event in resource_consumed.read() {
        timeline.push(entry(
            &sim_time,
            TimelineEventKind::ResourceConsumed,
            vec![event.agent, event.resource],
            format!("{:?} consumed {:.2}", event.kind, event.amount),
        ));
    }
    for event in resource_delivered.read() {
        timeline.push(entry(
            &sim_time,
            TimelineEventKind::ResourceDelivered,
            vec![event.agent, event.zone],
            format!("{:?} delivered {:.2}", event.kind, event.amount),
        ));
    }
    for event in resource_depleted.read() {
        timeline.push(entry(
            &sim_time,
            TimelineEventKind::ResourceDepleted,
            vec![event.resource],
            format!(
                "{:?} depleted at {}",
                event.kind,
                format_position(event.position)
            ),
        ));
    }
    for event in resource_replenished.read() {
        timeline.push(entry(
            &sim_time,
            TimelineEventKind::ResourceReplenished,
            vec![event.resource],
            "Resource replenished".to_string(),
        ));
    }
}

/// Draw event timeline UI.
pub fn timeline_ui_system(
    mut contexts: EguiContexts,
    config: Res<ObservabilityConfig>,
    mut timeline: ResMut<EventTimeline>,
) {
    if !config.timeline_open {
        return;
    }

    egui::Window::new("Event Timeline")
        .default_width(430.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter");
                ui.selectable_value(&mut timeline.filter, TimelineFilter::All, "All");
                ui.selectable_value(&mut timeline.filter, TimelineFilter::Agents, "Agents");
                ui.selectable_value(&mut timeline.filter, TimelineFilter::Behavior, "Behavior");
                ui.selectable_value(&mut timeline.filter, TimelineFilter::Resources, "Resources");
            });
            ui.label(format!("Events: {}", timeline.events.len()));
            draw_density(ui, &timeline);
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for event in timeline
                        .events
                        .iter()
                        .rev()
                        .filter(|event| event_matches_filter(&event.kind, timeline.filter))
                    {
                        ui.label(format!(
                            "t={:.2}s tick={} {:?}: {}",
                            event.timestamp, event.tick, event.kind, event.summary
                        ));
                    }
                });
        });
}

fn entry(
    sim_time: &SimulationTime,
    kind: TimelineEventKind,
    entities: Vec<Entity>,
    summary: String,
) -> TimelineEvent {
    TimelineEvent {
        timestamp: sim_time.elapsed,
        tick: sim_time.tick,
        kind,
        entities,
        summary,
    }
}

fn event_matches_filter(kind: &TimelineEventKind, filter: TimelineFilter) -> bool {
    match filter {
        TimelineFilter::All => true,
        TimelineFilter::Agents => matches!(
            kind,
            TimelineEventKind::AgentSpawned
                | TimelineEventKind::AgentDied
                | TimelineEventKind::NeedThreshold
        ),
        TimelineFilter::Behavior => matches!(kind, TimelineEventKind::AgentBehavior),
        TimelineFilter::Resources => matches!(
            kind,
            TimelineEventKind::ResourceConsumed
                | TimelineEventKind::ResourceDelivered
                | TimelineEventKind::ResourceDepleted
                | TimelineEventKind::ResourceReplenished
        ),
    }
}

fn draw_density(ui: &mut egui::Ui, timeline: &EventTimeline) {
    const BUCKETS: usize = 24;
    let mut buckets = [0_usize; BUCKETS];
    for (index, _event) in timeline.events.iter().rev().take(240).enumerate() {
        buckets[index * BUCKETS / 240] += 1;
    }
    let max = buckets.iter().copied().max().unwrap_or(1).max(1);
    let density: String = buckets
        .iter()
        .map(|count| match count * 4 / max {
            0 => '.',
            1 => '-',
            2 => '=',
            _ => '#',
        })
        .collect();
    ui.monospace(format!("density {density}"));
}

fn format_behavior_summary(event: &AgentBehaviorLogged) -> String {
    let scores = event
        .scores
        .iter()
        .map(|(action, score)| format!("{}={score:.2}", action_label(*action)))
        .collect::<Vec<_>>()
        .join(" ");
    let target = event
        .target_position
        .map_or_else(|| "-".to_string(), format_position);

    format!(
        "{:?}->{:?} intent {:?}->{:?} state={:?} score={:.2} needs h={:.2} f={:.2} e={:.2} target={} scores [{}]",
        event.previous_action,
        event.action,
        event.previous_intent,
        event.intent,
        event.state,
        event.score,
        event.hunger,
        event.fatigue,
        event.energy,
        target,
        scores
    )
}

fn action_label(action: ActionKind) -> &'static str {
    match action {
        ActionKind::Idle => "idle",
        ActionKind::MoveTo => "move",
        ActionKind::Eat => "eat",
        ActionKind::Rest => "rest",
        ActionKind::Deliver => "deliver",
        ActionKind::Explore => "explore",
        ActionKind::Collect => "collect",
    }
}

fn format_position(position: Vec3) -> String {
    format!("({:.1}, {:.1}, {:.1})", position.x, position.y, position.z)
}
