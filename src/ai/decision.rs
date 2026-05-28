//! Decision pipeline: score → select → execute.

use bevy::prelude::*;

use crate::ai::actions::{
    score_collect, score_eat, score_explore, score_idle, score_rest, ActionKind, ActionScore,
};
use crate::ai::utility::{AIConfig, ScoringContext};
use crate::engine::SimulationTime;
use crate::simulation::{
    Agent, AgentState, Needs, ResourceKind, ResourceNode, SimRng, SpatialGrid, Zone, ZoneKind,
};

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
) {
    for entity in &query {
        commands
            .entity(entity)
            .insert((DecisionOutput::default(), AIDebugInfo::default()));
    }
}

/// Evaluate utility scores and write `DecisionOutput` components.
pub fn ai_scoring_system(
    sim_time: Res<SimulationTime>,
    config: Res<AIConfig>,
    mut rng: ResMut<SimRng>,
    spatial_grid: Res<SpatialGrid>,
    mut agents: Query<(
        Entity,
        &Transform,
        &Needs,
        &AgentState,
        &mut DecisionOutput,
        &mut AIDebugInfo,
    )>,
    resources: Query<(Entity, &Transform, &ResourceNode)>,
    zones: Query<(Entity, &Transform, &Zone)>,
) {
    if sim_time.paused {
        return;
    }

    for (entity, transform, needs, state, mut decision, mut debug) in &mut agents {
        if sim_time.elapsed - decision.last_decision_time < config.decision_interval {
            continue;
        }

        let perception = build_perception(
            entity,
            transform.translation,
            config.perception_radius,
            &spatial_grid,
            &resources,
            &zones,
        );
        let explore_target =
            transform.translation + rng.next_xz_direction() * config.perception_radius * 0.5;
        let ctx = ScoringContext {
            needs,
            state,
            perception: &perception,
            config: &config,
            explore_target,
        };
        let scores = [
            score_eat(&ctx),
            score_rest(&ctx),
            score_explore(&ctx),
            score_collect(&ctx),
            score_idle(&ctx),
        ];
        let selected = select_best_action(&scores);

        *decision = DecisionOutput {
            action: selected.action,
            target: selected.target,
            target_position: selected.target_position,
            score: selected.score,
            last_decision_time: sim_time.elapsed,
        };
        debug.last_scores = scores
            .iter()
            .map(|score| (score.action, score.score))
            .collect();
        debug.last_decision_time = sim_time.elapsed;
    }
}

/// Build local perception for one agent.
#[must_use]
pub fn build_perception(
    agent: Entity,
    position: Vec3,
    radius: f32,
    spatial_grid: &SpatialGrid,
    resources: &Query<(Entity, &Transform, &ResourceNode)>,
    zones: &Query<(Entity, &Transform, &Zone)>,
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
            let distance = position.distance(transform.translation);
            if distance <= zone.radius {
                perception.current_zone = Some(VisibleZone {
                    entity,
                    position: transform.translation,
                    kind: zone.kind,
                    distance,
                });
            }
            if zone.kind == ZoneKind::Rest && distance <= radius {
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
        }
    }

    perception
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
