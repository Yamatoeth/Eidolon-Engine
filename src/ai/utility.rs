//! Utility function scoring system.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ai::decision::{AgentRole, PerceptionData};
use crate::ai::memory::AgentMemory;
use crate::simulation::{AgentState, CarriedResource, Needs};

/// Runtime AI configuration.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AIConfig {
    /// Utility action weights.
    pub utility_weights: UtilityWeights,
    /// Seconds between utility decisions.
    pub decision_interval: f32,
    /// Maximum perception distance in world units.
    pub perception_radius: f32,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            utility_weights: UtilityWeights::default(),
            decision_interval: 0.5,
            perception_radius: 30.0,
        }
    }
}

/// Tunable weights for utility actions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct UtilityWeights {
    /// Eat action weight.
    pub eat: f32,
    /// Rest action weight.
    pub rest: f32,
    /// Explore action weight.
    pub explore: f32,
    /// Collect action weight.
    pub collect: f32,
    /// Idle fallback weight.
    pub idle: f32,
}

impl Default for UtilityWeights {
    fn default() -> Self {
        Self {
            eat: 1.0,
            rest: 0.9,
            explore: 0.4,
            collect: 0.6,
            idle: 0.1,
        }
    }
}

/// Utility input curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curve {
    /// `f(x) = x`.
    Linear,
    /// `f(x) = x²`.
    Quadratic,
    /// `f(x) = 1 - (1 - x)²`.
    InverseQuadratic,
    /// `0` below cutoff, `1` at/above cutoff.
    Threshold {
        /// Cutoff in normalized range.
        cutoff: f32,
    },
}

impl Curve {
    /// Evaluate the curve for a normalized input.
    #[must_use]
    pub fn evaluate(self, input: f32) -> f32 {
        let x = input.clamp(0.0, 1.0);

        match self {
            Self::Linear => x,
            Self::Quadratic => x * x,
            Self::InverseQuadratic => 1.0 - (1.0 - x) * (1.0 - x),
            Self::Threshold { cutoff } => f32::from(x >= cutoff),
        }
    }
}

/// Common inputs for action scoring functions.
pub struct ScoringContext<'a> {
    /// Agent needs.
    pub needs: &'a Needs,
    /// Agent state.
    pub state: &'a AgentState,
    /// Lightweight behavioral role.
    pub role: &'a AgentRole,
    /// Recently remembered useful locations.
    pub memory: &'a AgentMemory,
    /// Resource parcel currently carried by the agent, if any.
    pub carried_resource: Option<&'a CarriedResource>,
    /// Local perception.
    pub perception: &'a PerceptionData,
    /// AI configuration.
    pub config: &'a AIConfig,
    /// Current agent position.
    pub position: Vec3,
    /// Current simulation time.
    pub now: f32,
    /// Deterministic explore target.
    pub explore_target: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_evaluate_expected_shapes() {
        assert_eq!(Curve::Linear.evaluate(0.5), 0.5);
        assert_eq!(Curve::Quadratic.evaluate(0.5), 0.25);
        assert_eq!(Curve::Threshold { cutoff: 0.7 }.evaluate(0.69), 0.0);
        assert_eq!(Curve::Threshold { cutoff: 0.7 }.evaluate(0.7), 1.0);
        assert!((Curve::InverseQuadratic.evaluate(0.5) - 0.75).abs() < f32::EPSILON);
    }
}
