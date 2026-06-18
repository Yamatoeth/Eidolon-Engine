//! Resource nodes, spawn/depletion/regeneration.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ai::actions::ActionKind;
use crate::ai::decision::DecisionOutput;
use crate::engine::SimulationTime;
use crate::simulation::events::{ResourceConsumed, ResourceDepleted, ResourceReplenished};
use crate::simulation::rules::RegenPressureMultiplier;
use crate::simulation::{Agent, AgentState, Needs, SimulationConfig, StateKind};

const RESOURCE_REPLENISH_THRESHOLD_FRACTION: f32 = 0.2;
const EAT_RANGE: f32 = 2.5;
const DEPOSIT_RANGE: f32 = 4.0;
const CARRY_CAPACITY: f32 = 24.0;
const FOOD_COST_PER_HUNGER: f32 = 3.0;
const VILLAGE_FEED_RATE: f32 = 2.0;
const DEFAULT_STORE_CAPACITY: f32 = 500.0;

type ForagingAgentQueryItem<'w> = (
    Entity,
    &'w Transform,
    &'w DecisionOutput,
    Option<&'w CarriedResource>,
);

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

/// Resource parcel currently carried by an agent.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CarriedResource {
    /// Carried resource category.
    pub kind: ResourceKind,
    /// Amount carried in resource units.
    pub amount: f32,
    /// Maximum amount this parcel could contain.
    pub capacity: f32,
    /// Source resource entity this parcel was gathered from.
    pub source: Entity,
}

/// Food storage attached to rest zones.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct VillageStore {
    /// Food available for agents inside the village.
    pub food_amount: f32,
    /// Maximum food storage.
    pub max_capacity: f32,
    /// Feeding and deposit radius in world units.
    pub radius: f32,
}

impl VillageStore {
    /// Create an empty village store.
    #[must_use]
    pub fn new(capacity: f32) -> Self {
        let max_capacity = if capacity > 0.0 {
            capacity
        } else {
            DEFAULT_STORE_CAPACITY
        };
        Self {
            food_amount: 0.0,
            max_capacity,
            radius: DEPOSIT_RANGE,
        }
    }

    fn deposit_food(&mut self, amount: f32) -> f32 {
        let accepted = amount.min((self.max_capacity - self.food_amount).max(0.0));
        self.food_amount += accepted;
        accepted
    }

    fn feed_agent(&mut self, needs: &mut Needs, dt: f32) {
        if self.food_amount <= f32::EPSILON || needs.hunger <= 0.0 {
            return;
        }

        let requested_hunger_reduction = (VILLAGE_FEED_RATE * dt).min(needs.hunger);
        let requested_food = requested_hunger_reduction * FOOD_COST_PER_HUNGER;
        let actual_food = requested_food.min(self.food_amount);
        if actual_food <= f32::EPSILON {
            return;
        }

        needs.hunger = (needs.hunger - actual_food / FOOD_COST_PER_HUNGER).max(0.0);
        self.food_amount = (self.food_amount - actual_food).max(0.0);
    }
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
    regen_pressure: Option<Res<RegenPressureMultiplier>>,
    mut query: Query<(Entity, &mut ResourceNode)>,
    mut replenished_events: EventWriter<ResourceReplenished>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;
    let regen_pressure = regen_pressure.map_or(1.0, |pressure| pressure.0);

    for (entity, mut resource) in &mut query {
        let before_depleted = resource.is_depleted;
        resource.amount = (resource.amount
            + resource.regen_rate * config.global_regen_multiplier * regen_pressure * dt)
            .min(resource.max_amount);

        if before_depleted
            && resource.amount >= resource.max_amount * RESOURCE_REPLENISH_THRESHOLD_FRACTION
        {
            resource.is_depleted = false;
            replenished_events.send(ResourceReplenished { resource: entity });
        }
    }
}

/// Pick up food from resource nodes and deposit carried food into nearby village stores.
pub fn resource_consume_system(
    mut commands: Commands,
    sim_time: Res<SimulationTime>,
    mut agents: Query<ForagingAgentQueryItem, With<Agent>>,
    mut resources: Query<(Entity, &Transform, &mut ResourceNode)>,
    mut stores: Query<(Entity, &Transform, &mut VillageStore)>,
    mut consumed_events: EventWriter<ResourceConsumed>,
    mut depleted_events: EventWriter<ResourceDepleted>,
) {
    if sim_time.paused {
        return;
    }

    for (agent, agent_transform, decision, carried) in &mut agents {
        if let Some(cargo) = carried {
            if let Some((store_entity, accepted)) =
                deposit_in_nearby_store(agent_transform.translation, cargo, &mut stores)
            {
                if accepted > f32::EPSILON {
                    consumed_events.send(ResourceConsumed {
                        agent,
                        resource: store_entity,
                        amount: accepted,
                        kind: cargo.kind,
                    });
                    commands.entity(agent).remove::<CarriedResource>();
                }
            }
            continue;
        }

        if !matches!(decision.action, ActionKind::Eat) {
            continue;
        }

        let Some(target) = decision.target else {
            continue;
        };
        let Ok((resource_entity, resource_transform, mut resource)) = resources.get_mut(target)
        else {
            continue;
        };

        if resource.is_depleted || resource.kind != ResourceKind::Food {
            continue;
        }

        if agent_transform
            .translation
            .distance(resource_transform.translation)
            > EAT_RANGE
        {
            continue;
        }

        let amount = CARRY_CAPACITY.min(resource.amount);
        if amount <= f32::EPSILON {
            continue;
        }

        resource.amount -= amount;
        commands.entity(agent).insert(CarriedResource {
            kind: ResourceKind::Food,
            amount,
            capacity: CARRY_CAPACITY,
            source: resource_entity,
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

fn deposit_in_nearby_store(
    position: Vec3,
    cargo: &CarriedResource,
    stores: &mut Query<(Entity, &Transform, &mut VillageStore)>,
) -> Option<(Entity, f32)> {
    stores
        .iter_mut()
        .find(|(_, transform, store)| {
            position.distance(transform.translation) <= store.radius.max(DEPOSIT_RANGE)
        })
        .map(|(entity, _transform, mut store)| {
            let accepted = match cargo.kind {
                ResourceKind::Food => store.deposit_food(cargo.amount),
                ResourceKind::Water | ResourceKind::Material => 0.0,
            };
            (entity, accepted)
        })
}

/// Recover fatigue and energy while agents are resting, and feed agents near village stores.
pub fn rest_recovery_system(
    sim_time: Res<SimulationTime>,
    mut agents: Query<(&Transform, &AgentState, &mut Needs), With<Agent>>,
    mut stores: Query<(&Transform, &mut VillageStore)>,
    mut next_rest_count_log: Local<f32>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;
    let mut resting_count = 0_u32;
    let mut total_count = 0_u32;

    for (transform, state, mut needs) in &mut agents {
        total_count = total_count.saturating_add(1);
        let is_near_store =
            feed_from_nearby_store(transform.translation, &mut needs, dt, &mut stores);

        if is_near_store && needs.fatigue > 0.6 && state.current != StateKind::Resting {
            needs.fatigue = (needs.fatigue - 0.02 * dt).clamp(0.0, 1.0);
            needs.energy = (needs.energy + 0.01 * dt).clamp(0.0, 1.0);
        }

        if state.current == StateKind::Resting {
            resting_count = resting_count.saturating_add(1);
            let fatigue_before = needs.fatigue;
            needs.fatigue = (needs.fatigue - 0.05 * dt).clamp(0.0, 1.0);
            needs.energy = (needs.energy + 0.06 * dt).clamp(0.0, 1.0);
            if fatigue_before > 0.1 {
                eprintln!(
                    "[REST] fatigue={:.3} -> {:.3} recovery={:.4}",
                    fatigue_before,
                    needs.fatigue,
                    fatigue_before - needs.fatigue
                );
            }
        }
    }

    if sim_time.elapsed >= *next_rest_count_log {
        eprintln!(
            "[REST_COUNT] t={:.0}s agents_resting={} agents_total={}",
            sim_time.elapsed, resting_count, total_count
        );
        *next_rest_count_log = sim_time.elapsed + 10.0;
    }
}

fn feed_from_nearby_store(
    position: Vec3,
    needs: &mut Needs,
    dt: f32,
    stores: &mut Query<(&Transform, &mut VillageStore)>,
) -> bool {
    for (transform, mut store) in stores.iter_mut() {
        if position.distance(transform.translation) <= 12.0 {
            store.feed_agent(needs, dt);
            return true;
        }
    }
    false
}
