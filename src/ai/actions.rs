//! Action definitions and scoring functions.

use bevy::prelude::*;

use crate::ai::decision::{AgentRole, VisibleResource};
use crate::ai::utility::{Curve, ScoringContext};
use crate::simulation::{ResourceKind, StateKind};

/// Agent action categories selected by the utility scorer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionKind {
    /// Do nothing for this decision interval.
    Idle,
    /// Move toward a target position.
    MoveTo,
    /// Eat from a food resource.
    Eat,
    /// Rest in a rest zone.
    Rest,
    /// Carry gathered resources back to a rest zone.
    Deliver,
    /// Explore with a deterministic wander target.
    Explore,
    /// Future non-food resource collection.
    Collect,
}

/// Result of scoring one action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionScore {
    /// Action that was scored.
    pub action: ActionKind,
    /// Utility score in `[0.0, 1.0]` after weighting.
    pub score: f32,
    /// Optional target entity for the selected action.
    pub target: Option<Entity>,
    /// Optional target world position for movement.
    pub target_position: Option<Vec3>,
}

impl ActionScore {
    /// Create a scored action.
    #[must_use]
    pub fn new(
        action: ActionKind,
        score: f32,
        target: Option<Entity>,
        target_position: Option<Vec3>,
    ) -> Self {
        Self {
            action,
            score: score.clamp(0.0, 1.0),
            target,
            target_position,
        }
    }
}

/// Score eating based on hunger and nearby food availability.
#[must_use]
pub fn score_eat(ctx: &ScoringContext<'_>) -> ActionScore {
    if ctx.carried_resource.is_some() {
        return ActionScore::new(ActionKind::Eat, 0.0, None, None);
    }

    let Some(food) = nearest_food_candidate(ctx) else {
        return ActionScore::new(ActionKind::Eat, 0.0, None, None);
    };

    let hunger_urgency = Curve::Quadratic.evaluate(ctx.needs.hunger);
    let not_eating = if ctx.state.current == StateKind::Eating {
        0.35
    } else {
        1.0
    };
    let distance_factor = distance_factor(food.distance, ctx.config.perception_radius);
    let role_bias = match ctx.role {
        AgentRole::Forager => 1.15,
        AgentRole::Worker => 0.95,
        AgentRole::Scout => 0.85,
    };
    let score =
        hunger_urgency * not_eating * distance_factor * ctx.config.utility_weights.eat * role_bias;

    ActionScore::new(
        ActionKind::Eat,
        score,
        Some(food.entity),
        Some(food.position),
    )
}

/// Score delivery when an agent is carrying a gathered resource.
#[must_use]
pub fn score_deliver(ctx: &ScoringContext<'_>) -> ActionScore {
    let Some(cargo) = ctx.carried_resource else {
        return ActionScore::new(ActionKind::Deliver, 0.0, None, None);
    };
    let Some(store) = ctx.perception.nearest_village_store else {
        return ActionScore::new(ActionKind::Deliver, 0.0, None, None);
    };

    let role_bias = match ctx.role {
        AgentRole::Forager => 1.1,
        AgentRole::Worker => 1.0,
        AgentRole::Scout => 0.9,
    };
    let cargo_urgency = (cargo.amount / cargo.capacity).clamp(0.35, 1.0) * role_bias;
    ActionScore::new(
        ActionKind::Deliver,
        cargo_urgency,
        Some(store.entity),
        Some(store.position),
    )
}

/// Score resting based on fatigue and nearby rest zones.
#[must_use]
pub fn score_rest(ctx: &ScoringContext<'_>) -> ActionScore {
    if ctx.carried_resource.is_some() {
        eprintln!(
            "[SCORE_REST] fatigue={:.2} score={:.3} rest_zone={:?}",
            ctx.needs.fatigue, 0.0, ctx.perception.nearest_rest_zone
        );
        return ActionScore::new(ActionKind::Rest, 0.0, None, None);
    }

    let Some(rest_zone) = ctx
        .perception
        .nearest_rest_zone
        .or(ctx.perception.nearest_village_store)
    else {
        eprintln!(
            "[SCORE_REST] fatigue={:.2} score={:.3} rest_zone={:?}",
            ctx.needs.fatigue, 0.0, ctx.perception.nearest_rest_zone
        );
        return ActionScore::new(ActionKind::Rest, 0.0, None, None);
    };

    let fatigue_urgency = Curve::Quadratic.evaluate(ctx.needs.fatigue);
    let not_resting = if ctx.state.current == StateKind::Resting {
        0.45
    } else {
        1.0
    };
    let distance_factor = distance_factor(rest_zone.distance, ctx.config.perception_radius);
    let role_bias = match ctx.role {
        AgentRole::Worker => 1.1,
        AgentRole::Forager => 0.95,
        AgentRole::Scout => 0.9,
    };
    let score = fatigue_urgency
        * not_resting
        * distance_factor
        * ctx.config.utility_weights.rest
        * role_bias;
    eprintln!(
        "[SCORE_REST] fatigue={:.2} score={:.3} rest_zone={:?}",
        ctx.needs.fatigue, score, ctx.perception.nearest_rest_zone
    );

    ActionScore::new(
        ActionKind::Rest,
        score,
        Some(rest_zone.entity),
        Some(rest_zone.position),
    )
}

/// Score exploration when needs are not urgent.
#[must_use]
pub fn score_explore(ctx: &ScoringContext<'_>) -> ActionScore {
    if ctx.carried_resource.is_some() {
        return ActionScore::new(ActionKind::Explore, 0.0, None, None);
    }

    let hunger_ok = 1.0 - Curve::Threshold { cutoff: 0.7 }.evaluate(ctx.needs.hunger);
    let fatigue_ok = 1.0 - Curve::Threshold { cutoff: 0.75 }.evaluate(ctx.needs.fatigue);
    let energy_ok = Curve::InverseQuadratic.evaluate(ctx.needs.energy);
    let role_bias = match ctx.role {
        AgentRole::Scout => 1.25,
        AgentRole::Forager => 0.95,
        AgentRole::Worker => 0.85,
    };
    let score = hunger_ok
        * fatigue_ok
        * energy_ok
        * ctx.config.utility_weights.explore
        * boredom_factor(ctx.state.time_in_state)
        * role_bias;

    ActionScore::new(ActionKind::Explore, score, None, Some(ctx.explore_target))
}

/// Score generic collection for visible non-food resources.
#[must_use]
pub fn score_collect(ctx: &ScoringContext<'_>) -> ActionScore {
    if ctx.carried_resource.is_some() {
        return ActionScore::new(ActionKind::Collect, 0.0, None, None);
    }

    let Some(resource) = ctx
        .perception
        .visible_resources
        .iter()
        .find(|resource| resource.kind != ResourceKind::Food)
    else {
        return ActionScore::new(ActionKind::Collect, 0.0, None, None);
    };

    let not_starving = 1.0 - Curve::Threshold { cutoff: 0.8 }.evaluate(ctx.needs.hunger);
    let score = not_starving * ctx.config.utility_weights.collect;

    ActionScore::new(
        ActionKind::Collect,
        score,
        Some(resource.entity),
        Some(resource.position),
    )
}

/// Score idle as the low-priority fallback action.
#[must_use]
pub fn score_idle(ctx: &ScoringContext<'_>) -> ActionScore {
    ActionScore::new(
        ActionKind::Idle,
        ctx.config.utility_weights.idle,
        None,
        None,
    )
}

fn distance_factor(distance: f32, perception_radius: f32) -> f32 {
    (1.0 - (distance / perception_radius.max(0.1)).clamp(0.0, 1.0) * 0.35).clamp(0.0, 1.0)
}

fn boredom_factor(time_in_state: f32) -> f32 {
    (time_in_state / 4.0).clamp(0.35, 1.0)
}

fn nearest_food_candidate(ctx: &ScoringContext<'_>) -> Option<VisibleResource> {
    ctx.perception.nearest_food().or_else(|| {
        ctx.memory
            .nearest_food(ctx.position, ctx.now)
            .map(|resource| VisibleResource {
                entity: resource.entity,
                position: resource.position,
                kind: resource.kind,
                amount: resource.estimated_amount,
                distance: ctx.position.distance(resource.position),
            })
    })
}
