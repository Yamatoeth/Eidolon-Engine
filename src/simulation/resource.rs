//! Resource nodes, spawn/depletion/regeneration.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ai::actions::ActionKind;
use crate::ai::decision::DecisionOutput;
use crate::engine::SimulationTime;
use crate::simulation::events::{
    ResourceConsumed, ResourceDelivered, ResourceDepleted, ResourceReplenished,
};
use crate::simulation::rules::{CompetitionFactor, RegenPressureMultiplier};
use crate::simulation::{Agent, AgentState, Needs, SimulationConfig, StateKind, Zone, ZoneKind};

const RESOURCE_CONSUME_RATE: f32 = 18.0;
const RESOURCE_REPLENISH_THRESHOLD_FRACTION: f32 = 0.2;
const EAT_RANGE: f32 = 1.5;
const CARRY_CAPACITY: f32 = 24.0;
const VILLAGE_EAT_RATE: f32 = 0.18;

type ForagingAgentQueryItem<'w> = (
    Entity,
    &'w Transform,
    &'w Needs,
    &'w DecisionOutput,
    Option<&'w CompetitionFactor>,
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
    pub food: f32,
    /// Maximum food storage.
    pub capacity: f32,
}

impl VillageStore {
    /// Create an empty village store.
    #[must_use]
    pub fn new(capacity: f32) -> Self {
        Self {
            food: 0.0,
            capacity,
        }
    }

    fn deposit_food(&mut self, amount: f32) -> f32 {
        let accepted = amount.min((self.capacity - self.food).max(0.0));
        self.food += accepted;
        accepted
    }

    fn withdraw_food(&mut self, requested: f32) -> f32 {
        let taken = requested.min(self.food).max(0.0);
        self.food -= taken;
        taken
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

/// Gather food near resources, then consume it when agents return to a rest zone.
pub fn resource_consume_system(
    mut commands: Commands,
    sim_time: Res<SimulationTime>,
    mut agents: Query<ForagingAgentQueryItem, With<Agent>>,
    mut resources: Query<(Entity, &Transform, &mut ResourceNode)>,
    mut zones: Query<(Entity, &Transform, &Zone, &mut VillageStore)>,
    mut consumed_events: EventWriter<ResourceConsumed>,
    mut delivered_events: EventWriter<ResourceDelivered>,
    mut depleted_events: EventWriter<ResourceDepleted>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;

    for (agent, agent_transform, needs, decision, competition, carried) in &mut agents {
        if let Some(cargo) = carried {
            if let Some((zone, accepted)) =
                deposit_in_current_village(agent_transform.translation, cargo, &mut zones)
            {
                if accepted <= f32::EPSILON {
                    continue;
                }
                delivered_events.send(ResourceDelivered {
                    agent,
                    zone,
                    amount: accepted,
                    kind: cargo.kind,
                });
                commands.entity(agent).remove::<CarriedResource>();
            }
            continue;
        }

        if !matches!(decision.action, ActionKind::Eat | ActionKind::Collect) {
            continue;
        }

        let Some(target) = decision.target else {
            continue;
        };
        let Ok((resource_entity, resource_transform, mut resource)) = resources.get_mut(target)
        else {
            continue;
        };

        if resource.is_depleted
            || matches!(decision.action, ActionKind::Eat) && resource.kind != ResourceKind::Food
            || matches!(decision.action, ActionKind::Collect) && resource.kind == ResourceKind::Food
        {
            continue;
        }

        let distance = agent_transform
            .translation
            .distance(resource_transform.translation);
        if distance > EAT_RANGE {
            continue;
        }

        let competition_factor = competition.map_or(1.0, |factor| factor.0);
        let desired =
            (needs.hunger * RESOURCE_CONSUME_RATE * dt).max(CARRY_CAPACITY) * competition_factor;
        let amount = desired.min(resource.amount);
        if amount <= f32::EPSILON {
            continue;
        }

        resource.amount -= amount;
        commands.entity(agent).insert(CarriedResource {
            kind: resource.kind,
            amount,
            capacity: CARRY_CAPACITY,
            source: resource_entity,
        });
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

fn deposit_in_current_village(
    position: Vec3,
    cargo: &CarriedResource,
    zones: &mut Query<(Entity, &Transform, &Zone, &mut VillageStore)>,
) -> Option<(Entity, f32)> {
    zones
        .iter_mut()
        .find(|(_, transform, zone, _)| {
            zone.kind == ZoneKind::Rest && position.distance(transform.translation) <= zone.radius
        })
        .map(|(entity, _transform, _zone, mut store)| {
            let accepted = match cargo.kind {
                ResourceKind::Food => store.deposit_food(cargo.amount),
                ResourceKind::Water | ResourceKind::Material => 0.0,
            };
            (entity, accepted)
        })
}

/// Recover fatigue and energy while agents are resting.
pub fn rest_recovery_system(
    sim_time: Res<SimulationTime>,
    mut agents: Query<(&Transform, &AgentState, &mut Needs), With<Agent>>,
    mut stores: Query<(&Transform, &Zone, &mut VillageStore)>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;

    for (transform, state, mut needs) in &mut agents {
        if needs.hunger > 0.0 {
            feed_from_current_village(transform.translation, &mut needs, dt, &mut stores);
        }

        if state.current == StateKind::Resting {
            needs.fatigue = (needs.fatigue - 0.05 * dt).clamp(0.0, 1.0);
            needs.energy = (needs.energy + 0.03 * dt).clamp(0.0, 1.0);
        }
    }
}

fn feed_from_current_village(
    position: Vec3,
    needs: &mut Needs,
    dt: f32,
    stores: &mut Query<(&Transform, &Zone, &mut VillageStore)>,
) {
    let Some((_transform, _zone, mut store)) = stores.iter_mut().find(|(transform, zone, _)| {
        zone.kind == ZoneKind::Rest && position.distance(transform.translation) <= zone.radius
    }) else {
        return;
    };

    let requested = (needs.hunger * VILLAGE_EAT_RATE * CARRY_CAPACITY * dt).max(0.0);
    let food = store.withdraw_food(requested);
    if food <= f32::EPSILON {
        return;
    }

    needs.hunger = (needs.hunger - food / CARRY_CAPACITY).clamp(0.0, 1.0);
    needs.energy = (needs.energy + food / CARRY_CAPACITY * 0.35).clamp(0.0, 1.0);
}
