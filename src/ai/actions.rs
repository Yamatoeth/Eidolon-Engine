//! Action definitions and scoring functions.

use bevy::prelude::*;

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
    let Some(food) = ctx.perception.nearest_food() else {
        return ActionScore::new(ActionKind::Eat, 0.0, None, None);
    };

    let hunger_urgency = Curve::Quadratic.evaluate(ctx.needs.hunger);
    let not_eating = if ctx.state.current == StateKind::Eating {
        0.35
    } else {
        1.0
    };
    let distance_factor = distance_factor(food.distance, ctx.config.perception_radius);
    let score = hunger_urgency * not_eating * distance_factor * ctx.config.utility_weights.eat;

    ActionScore::new(
        ActionKind::Eat,
        score,
        Some(food.entity),
        Some(food.position),
    )
}

/// Score resting based on fatigue and nearby rest zones.
#[must_use]
pub fn score_rest(ctx: &ScoringContext<'_>) -> ActionScore {
    let Some(rest_zone) = ctx.perception.nearest_rest_zone else {
        return ActionScore::new(ActionKind::Rest, 0.0, None, None);
    };

    let fatigue_urgency = Curve::Quadratic.evaluate(ctx.needs.fatigue);
    let not_resting = if ctx.state.current == StateKind::Resting {
        0.45
    } else {
        1.0
    };
    let distance_factor = distance_factor(rest_zone.distance, ctx.config.perception_radius);
    let score = fatigue_urgency * not_resting * distance_factor * ctx.config.utility_weights.rest;

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
    let hunger_ok = 1.0 - Curve::Threshold { cutoff: 0.7 }.evaluate(ctx.needs.hunger);
    let fatigue_ok = 1.0 - Curve::Threshold { cutoff: 0.75 }.evaluate(ctx.needs.fatigue);
    let energy_ok = Curve::InverseQuadratic.evaluate(ctx.needs.energy);
    let score = hunger_ok
        * fatigue_ok
        * energy_ok
        * ctx.config.utility_weights.explore
        * boredom_factor(ctx.state.time_in_state);

    ActionScore::new(ActionKind::Explore, score, None, Some(ctx.explore_target))
}

/// Score generic collection for visible non-food resources.
#[must_use]
pub fn score_collect(ctx: &ScoringContext<'_>) -> ActionScore {
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
