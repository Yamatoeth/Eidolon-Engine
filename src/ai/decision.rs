//! Decision pipeline: score → select → execute.

use bevy::prelude::*;

use crate::ai::actions::{
    score_collect, score_deliver, score_eat, score_explore, score_idle, score_rest, ActionKind,
    ActionScore,
};
use crate::ai::memory::AgentMemory;
use crate::ai::utility::{AIConfig, ScoringContext};
use crate::engine::SimulationTime;
use crate::simulation::{
    Agent, AgentState, CarriedResource, Needs, ResourceKind, ResourceNode, SimRng, SpatialGrid,
    StateKind, VillageStore, Zone, ZoneKind,
};

type AIAgentQueryItem<'w> = (
    Entity,
    &'w Transform,
    &'w Needs,
    &'w AgentState,
    &'w AgentRole,
    &'w AgentMemory,
    Option<&'w CarriedResource>,
    &'w mut AgentIntent,
    &'w mut DecisionOutput,
    &'w mut AIDebugInfo,
);

/// Lightweight behavioral role that biases utility decisions.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentRole {
    /// Prioritizes discovering and broadcasting useful locations.
    Scout,
    /// Prioritizes food collection and delivery.
    Forager,
    /// Prioritizes rest-zone stability and store usage.
    Worker,
}

/// High-level intent selected by the AI layer.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentIntent {
    /// No current goal.
    Idle,
    /// Searching unknown space.
    Explore,
    /// Moving to a resource node.
    Forage { resource: Entity },
    /// Returning cargo to a rest zone.
    Deliver { zone: Entity },
    /// Recovering around a rest zone.
    Rest { zone: Entity },
}

impl Default for AgentIntent {
    fn default() -> Self {
        Self::Idle
    }
}

/// Output of the AI decision pipeline.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct DecisionOutput {
    /// Selected action.
    pub action: ActionKind,
    /// Optional target entity for the selected action.
    pub target: Option<Entity>,
    /// Optional target world position.
    pub target_position: Option<Vec3>,
    /// Selected action utility score.
    pub score: f32,
    /// Last simulation time this decision was evaluated.
    pub last_decision_time: f32,
}

impl Default for DecisionOutput {
    fn default() -> Self {
        Self {
            action: ActionKind::Idle,
            target: None,
            target_position: None,
            score: 0.0,
            last_decision_time: f32::NEG_INFINITY,
        }
    }
}

/// Last utility scores for debug overlays and inspector display.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct AIDebugInfo {
    /// Last scored actions.
    pub last_scores: Vec<(ActionKind, f32)>,
    /// Last decision timestamp.
    pub last_decision_time: f32,
}

/// Behavioral decision trace emitted when an agent changes behavior.
#[derive(Event, Clone, Debug, PartialEq)]
pub struct AgentBehaviorLogged {
    /// Agent entity whose behavior changed.
    pub agent: Entity,
    /// Previous selected action.
    pub previous_action: ActionKind,
    /// Newly selected action.
    pub action: ActionKind,
    /// Previous high-level intent.
    pub previous_intent: AgentIntent,
    /// Newly selected high-level intent.
    pub intent: AgentIntent,
    /// Current simulation state.
    pub state: StateKind,
    /// Optional selected target entity.
    pub target: Option<Entity>,
    /// Optional selected world target.
    pub target_position: Option<Vec3>,
    /// Selected action utility score.
    pub score: f32,
    /// Hunger at decision time.
    pub hunger: f32,
    /// Fatigue at decision time.
    pub fatigue: f32,
    /// Energy at decision time.
    pub energy: f32,
    /// Last scored actions in evaluation order.
    pub scores: Vec<(ActionKind, f32)>,
}

impl Default for AIDebugInfo {
    fn default() -> Self {
        Self {
            last_scores: Vec::new(),
            last_decision_time: f32::NEG_INFINITY,
        }
    }
}

/// Perceived resource entry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibleResource {
    /// Resource entity.
    pub entity: Entity,
    /// Resource position.
    pub position: Vec3,
    /// Resource kind.
    pub kind: ResourceKind,
    /// Current resource amount.
    pub amount: f32,
    /// Distance from perceiving agent.
    pub distance: f32,
}

/// Perceived zone entry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibleZone {
    /// Zone entity.
    pub entity: Entity,
    /// Zone center position.
    pub position: Vec3,
    /// Zone kind.
    pub kind: ZoneKind,
    /// Distance from perceiving agent to zone center.
    pub distance: f32,
}

/// Local perception data for one agent decision.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PerceptionData {
    /// Visible resources inside perception range.
    pub visible_resources: Vec<VisibleResource>,
    /// Nearest rest zone inside perception range.
    pub nearest_rest_zone: Option<VisibleZone>,
    /// Nearest village store inside perception range.
    pub nearest_village_store: Option<VisibleZone>,
    /// Current zone containing the agent, if any.
    pub current_zone: Option<VisibleZone>,
}

impl PerceptionData {
    /// Return the nearest non-depleted food resource.
    #[must_use]
    pub fn nearest_food(&self) -> Option<VisibleResource> {
        self.visible_resources
            .iter()
            .copied()
            .filter(|resource| resource.kind == ResourceKind::Food && resource.amount > 0.0)
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }
}

/// Ensure agents spawned before AI startup have AI components.
pub fn attach_ai_components_system(
    mut commands: Commands,
    query: Query<Entity, (With<Agent>, Without<DecisionOutput>)>,
    agents: Query<&Agent>,
) {
    for entity in &query {
        let role = agents
            .get(entity)
            .map_or(AgentRole::Forager, |agent| role_for_agent(agent.id.0));
        commands.entity(entity).insert((
            role,
            AgentIntent::default(),
            AgentMemory::default(),
            DecisionOutput::default(),
            AIDebugInfo::default(),
        ));
    }
}

/// Evaluate utility scores and write `DecisionOutput` components.
#[allow(clippy::too_many_arguments)]
pub fn ai_scoring_system(
    sim_time: Res<SimulationTime>,
    config: Res<AIConfig>,
    mut rng: ResMut<SimRng>,
    spatial_grid: Res<SpatialGrid>,
    mut behavior_events: EventWriter<AgentBehaviorLogged>,
    mut agents: Query<AIAgentQueryItem>,
    resources: Query<(Entity, &Transform, &ResourceNode)>,
    zones: Query<(Entity, &Transform, &Zone)>,
    stores: Query<(Entity, &Transform, &VillageStore)>,
) {
    if sim_time.paused {
        return;
    }

    for (
        entity,
        transform,
        needs,
        state,
        role,
        memory,
        carried_resource,
        mut intent,
        mut decision,
        mut debug,
    ) in &mut agents
    {
        if sim_time.elapsed - decision.last_decision_time < config.decision_interval {
            continue;
        }

        let previous_decision = *decision;
        let previous_intent = *intent;
        let perception = build_perception(
            entity,
            transform.translation,
            config.perception_radius,
            config.rest_zone_perception_radius,
            &spatial_grid,
            &resources,
            &zones,
            &stores,
        );
        let explore_target =
            transform.translation + rng.next_xz_direction() * config.perception_radius * 0.5;
        let ctx = ScoringContext {
            needs,
            state,
            role,
            memory,
            carried_resource,
            perception: &perception,
            config: &config,
            position: transform.translation,
            now: sim_time.elapsed,
            explore_target,
        };
        let scores = [
            score_deliver(&ctx),
            score_eat(&ctx),
            score_rest(&ctx),
            score_explore(&ctx),
            score_collect(&ctx),
            score_idle(&ctx),
        ];
        let selected = select_best_action(&scores);
        let next_intent = intent_for_action(selected);
        let score_snapshot = scores
            .iter()
            .map(|score| (score.action, score.score))
            .collect::<Vec<_>>();

        if should_log_behavior_change(previous_decision, previous_intent, selected, next_intent) {
            behavior_events.send(AgentBehaviorLogged {
                agent: entity,
                previous_action: previous_decision.action,
                action: selected.action,
                previous_intent,
                intent: next_intent,
                state: state.current,
                target: selected.target,
                target_position: selected.target_position,
                score: selected.score,
                hunger: needs.hunger,
                fatigue: needs.fatigue,
                energy: needs.energy,
                scores: score_snapshot.clone(),
            });
        }

        *decision = DecisionOutput {
            action: selected.action,
            target: selected.target,
            target_position: selected.target_position,
            score: selected.score,
            last_decision_time: sim_time.elapsed,
        };
        debug.last_scores = score_snapshot;
        debug.last_decision_time = sim_time.elapsed;
        *intent = next_intent;
    }
}

/// Build local perception for one agent.
#[must_use]
pub fn build_perception(
    agent: Entity,
    position: Vec3,
    radius: f32,
    rest_zone_radius: f32,
    spatial_grid: &SpatialGrid,
    resources: &Query<(Entity, &Transform, &ResourceNode)>,
    zones: &Query<(Entity, &Transform, &Zone)>,
    stores: &Query<(Entity, &Transform, &VillageStore)>,
) -> PerceptionData {
    let mut perception = PerceptionData::default();

    for candidate in spatial_grid.entities_in_radius(position, radius) {
        if candidate == agent {
            continue;
        }

        if let Ok((entity, transform, resource)) = resources.get(candidate) {
            let distance = position.distance(transform.translation);
            if distance <= radius && !resource.is_depleted {
                perception.visible_resources.push(VisibleResource {
                    entity,
                    position: transform.translation,
                    kind: resource.kind,
                    amount: resource.amount,
                    distance,
                });
            }
        }

        if let Ok((entity, transform, zone)) = zones.get(candidate) {
            update_current_zone(&mut perception, position, entity, transform, zone);
        }
    }

    for (entity, transform, zone) in zones.iter() {
        if zone.kind != ZoneKind::Rest {
            continue;
        }
        let distance = position.distance(transform.translation);
        if distance > rest_zone_radius {
            continue;
        }
        let zone_entry = VisibleZone {
            entity,
            position: transform.translation,
            kind: zone.kind,
            distance,
        };
        if perception
            .nearest_rest_zone
            .map_or(true, |nearest| distance < nearest.distance)
        {
            perception.nearest_rest_zone = Some(zone_entry);
        }
    }

    for (entity, transform, _store) in stores.iter() {
        let distance = position.distance(transform.translation);
        if distance > rest_zone_radius {
            continue;
        }
        let store_entry = VisibleZone {
            entity,
            position: transform.translation,
            kind: ZoneKind::Rest,
            distance,
        };
        if perception
            .nearest_village_store
            .map_or(true, |nearest| distance < nearest.distance)
        {
            perception.nearest_village_store = Some(store_entry);
        }
    }

    perception
}

fn update_current_zone(
    perception: &mut PerceptionData,
    position: Vec3,
    entity: Entity,
    transform: &Transform,
    zone: &Zone,
) {
    let distance = position.distance(transform.translation);
    if distance <= zone.radius {
        perception.current_zone = Some(VisibleZone {
            entity,
            position: transform.translation,
            kind: zone.kind,
            distance,
        });
    }
}

/// Select the highest scoring action.
#[must_use]
pub fn select_best_action(scores: &[ActionScore]) -> ActionScore {
    scores
        .iter()
        .copied()
        .max_by(|a, b| a.score.total_cmp(&b.score))
        .unwrap_or_else(|| ActionScore::new(ActionKind::Idle, 0.0, None, None))
}

fn role_for_agent(agent_id: u64) -> AgentRole {
    match agent_id % 5 {
        0 => AgentRole::Scout,
        1 | 2 => AgentRole::Forager,
        _ => AgentRole::Worker,
    }
}

fn should_log_behavior_change(
    previous_decision: DecisionOutput,
    previous_intent: AgentIntent,
    selected: ActionScore,
    next_intent: AgentIntent,
) -> bool {
    previous_decision.action != selected.action
        || previous_decision.target != selected.target
        || previous_intent != next_intent
}

fn intent_for_action(action: ActionScore) -> AgentIntent {
    match action.action {
        ActionKind::Idle | ActionKind::MoveTo => AgentIntent::Idle,
        ActionKind::Explore | ActionKind::Collect => AgentIntent::Explore,
        ActionKind::Eat => {
            action
                .target
                .map_or(AgentIntent::Explore, |resource| AgentIntent::Forage {
                    resource,
                })
        },
        ActionKind::Deliver => action
            .target
            .map_or(AgentIntent::Idle, |zone| AgentIntent::Deliver { zone }),
        ActionKind::Rest => action
            .target
            .map_or(AgentIntent::Idle, |zone| AgentIntent::Rest { zone }),
    }
}
