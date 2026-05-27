//! Resource nodes, spawn/depletion/regeneration.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Harvestable resource node.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ResourceNode {
    /// Resource category.
    pub kind: ResourceKind,
    /// Current resource supply.
    pub amount: f32,
    /// Maximum resource supply.
    pub max_amount: f32,
    /// Regeneration units per simulation second.
    pub regen_rate: f32,
    /// Whether the node is currently depleted.
    pub is_depleted: bool,
}

/// Resource categories available in the world.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ResourceKind {
    /// Food satisfies hunger.
    Food,
    /// Future water resource.
    Water,
    /// Future material resource.
    Material,
}

impl ResourceNode {
    /// Create a food node with an initial fraction of its maximum supply.
    #[must_use]
    pub fn food(max_amount: f32, initial_fraction: f32, regen_rate: f32) -> Self {
        let amount = max_amount * initial_fraction.clamp(0.0, 1.0);

        Self {
            kind: ResourceKind::Food,
            amount,
            max_amount,
            regen_rate,
            is_depleted: amount <= f32::EPSILON,
        }
    }
}
