//! Resource nodes, spawn/depletion/regeneration.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ai::actions::ActionKind;
use crate::ai::decision::DecisionOutput;
use crate::engine::SimulationTime;
use crate::simulation::events::{ResourceConsumed, ResourceDepleted, ResourceReplenished};
use crate::simulation::{Agent, AgentState, Needs, SimulationConfig, StateKind};

const RESOURCE_CONSUME_RATE: f32 = 18.0;
const RESOURCE_REPLENISH_THRESHOLD_FRACTION: f32 = 0.2;
const EAT_RANGE: f32 = 1.5;

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

/// Regenerate depleted and partially used resource nodes.
pub fn resource_regen_system(
    sim_time: Res<SimulationTime>,
    config: Res<SimulationConfig>,
    mut query: Query<(Entity, &mut ResourceNode)>,
    mut replenished_events: EventWriter<ResourceReplenished>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;

    for (entity, mut resource) in &mut query {
        let before_depleted = resource.is_depleted;
        resource.amount = (resource.amount
            + resource.regen_rate * config.global_regen_multiplier * dt)
            .min(resource.max_amount);

        if before_depleted
            && resource.amount >= resource.max_amount * RESOURCE_REPLENISH_THRESHOLD_FRACTION
        {
            resource.is_depleted = false;
            replenished_events.send(ResourceReplenished { resource: entity });
        }
    }
}

/// Consume food resources when agents are close enough to their selected target.
pub fn resource_consume_system(
    sim_time: Res<SimulationTime>,
    mut agents: Query<(Entity, &Transform, &mut Needs, &DecisionOutput), With<Agent>>,
    mut resources: Query<(Entity, &Transform, &mut ResourceNode)>,
    mut consumed_events: EventWriter<ResourceConsumed>,
    mut depleted_events: EventWriter<ResourceDepleted>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;

    for (agent, agent_transform, mut needs, decision) in &mut agents {
        if decision.action != ActionKind::Eat {
            continue;
        }

        let Some(target) = decision.target else {
            continue;
        };
        let Ok((resource_entity, resource_transform, mut resource)) = resources.get_mut(target)
        else {
            continue;
        };

        if resource.kind != ResourceKind::Food || resource.is_depleted {
            continue;
        }

        let distance = agent_transform
            .translation
            .distance(resource_transform.translation);
        if distance > EAT_RANGE {
            continue;
        }

        let desired = (needs.hunger * RESOURCE_CONSUME_RATE * dt).max(0.0);
        let amount = desired.min(resource.amount);
        if amount <= f32::EPSILON {
            continue;
        }

        resource.amount -= amount;
        needs.hunger = (needs.hunger - amount / resource.max_amount).clamp(0.0, 1.0);
        needs.energy = (needs.energy + amount / resource.max_amount * 0.4).clamp(0.0, 1.0);
        consumed_events.send(ResourceConsumed {
            agent,
            resource: resource_entity,
            amount,
            kind: resource.kind,
        });

        if resource.amount <= f32::EPSILON {
            resource.amount = 0.0;
            resource.is_depleted = true;
            depleted_events.send(ResourceDepleted {
                resource: resource_entity,
                position: resource_transform.translation,
                kind: resource.kind,
            });
        }
    }
}

/// Recover fatigue and energy while agents are resting.
pub fn rest_recovery_system(
    sim_time: Res<SimulationTime>,
    mut query: Query<(&AgentState, &mut Needs), With<Agent>>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;

    for (state, mut needs) in &mut query {
        if state.current == StateKind::Resting {
            needs.fatigue = (needs.fatigue - 0.05 * dt).clamp(0.0, 1.0);
            needs.energy = (needs.energy + 0.03 * dt).clamp(0.0, 1.0);
        }
    }
}
