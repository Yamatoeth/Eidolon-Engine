//! Lightweight agent memory and local knowledge sharing.

use bevy::prelude::*;

use crate::engine::SimulationTime;
use crate::simulation::{Agent, ResourceKind, ResourceNode, Zone, ZoneKind};

const MEMORY_TTL_SECS: f32 = 45.0;
const MAX_KNOWN_RESOURCES: usize = 8;
const MAX_KNOWN_REST_ZONES: usize = 4;
const SHARE_RADIUS: f32 = 9.0;

type MemoryReadQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Transform, &'static AgentMemory), With<Agent>>;
type MemoryWriteQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Transform, &'static mut AgentMemory), With<Agent>>;

/// Remembered resource location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownResource {
    /// Resource entity remembered by the agent.
    pub entity: Entity,
    /// Last known world position.
    pub position: Vec3,
    /// Resource kind.
    pub kind: ResourceKind,
    /// Last observed amount.
    pub estimated_amount: f32,
    /// Simulation timestamp when last observed.
    pub last_seen: f32,
}

/// Remembered rest-zone location.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownRestZone {
    /// Zone entity remembered by the agent.
    pub entity: Entity,
    /// Last known zone center.
    pub position: Vec3,
    /// Simulation timestamp when last observed.
    pub last_seen: f32,
}

/// Per-agent lightweight memory of useful world locations.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct AgentMemory {
    /// Recently seen resources.
    pub resources: Vec<KnownResource>,
    /// Recently seen rest zones.
    pub rest_zones: Vec<KnownRestZone>,
}

impl AgentMemory {
    /// Best remembered food resource for the provided position.
    #[must_use]
    pub fn nearest_food(&self, position: Vec3, now: f32) -> Option<KnownResource> {
        self.resources
            .iter()
            .copied()
            .filter(|resource| {
                resource.kind == ResourceKind::Food
                    && resource.estimated_amount > 0.0
                    && now - resource.last_seen <= MEMORY_TTL_SECS
            })
            .min_by(|a, b| {
                position
                    .distance(a.position)
                    .total_cmp(&position.distance(b.position))
            })
    }

    fn remember_resource(&mut self, resource: KnownResource) {
        remember_or_insert(
            &mut self.resources,
            MAX_KNOWN_RESOURCES,
            resource,
            |entry| entry.entity,
        );
    }

    fn remember_rest_zone(&mut self, zone: KnownRestZone) {
        remember_or_insert(&mut self.rest_zones, MAX_KNOWN_REST_ZONES, zone, |entry| {
            entry.entity
        });
    }

    fn prune(&mut self, now: f32) {
        self.resources
            .retain(|resource| now - resource.last_seen <= MEMORY_TTL_SECS);
        self.rest_zones
            .retain(|zone| now - zone.last_seen <= MEMORY_TTL_SECS);
    }
}

/// Keep memory fresh from currently visible entities.
pub fn update_agent_memory_system(
    sim_time: Res<SimulationTime>,
    mut agents: Query<(&Transform, &mut AgentMemory), With<Agent>>,
    resources: Query<(Entity, &Transform, &ResourceNode)>,
    zones: Query<(Entity, &Transform, &Zone)>,
) {
    if sim_time.paused {
        return;
    }

    for (agent_transform, mut memory) in &mut agents {
        memory.prune(sim_time.elapsed);
        let position = agent_transform.translation;

        for (entity, transform, resource) in &resources {
            let distance = position.distance(transform.translation);
            if distance <= 30.0 && !resource.is_depleted {
                memory.remember_resource(KnownResource {
                    entity,
                    position: transform.translation,
                    kind: resource.kind,
                    estimated_amount: resource.amount,
                    last_seen: sim_time.elapsed,
                });
            }
        }

        for (entity, transform, zone) in &zones {
            let distance = position.distance(transform.translation);
            if zone.kind == ZoneKind::Rest && distance <= 30.0 {
                memory.remember_rest_zone(KnownRestZone {
                    entity,
                    position: transform.translation,
                    last_seen: sim_time.elapsed,
                });
            }
        }
    }
}

/// Share recent knowledge between nearby agents.
pub fn share_agent_memory_system(
    sim_time: Res<SimulationTime>,
    mut agents: ParamSet<(MemoryReadQuery, MemoryWriteQuery)>,
) {
    if sim_time.paused {
        return;
    }

    let known = agents
        .p0()
        .iter()
        .map(|(entity, transform, memory)| (entity, transform.translation, memory.clone()))
        .collect::<Vec<_>>();

    for (receiver_entity, receiver_transform, mut receiver_memory) in &mut agents.p1() {
        for (sender_entity, sender_position, sender_memory) in &known {
            if receiver_entity == *sender_entity
                || receiver_transform.translation.distance(*sender_position) > SHARE_RADIUS
            {
                continue;
            }

            for resource in &sender_memory.resources {
                receiver_memory.remember_resource(*resource);
            }
            for zone in &sender_memory.rest_zones {
                receiver_memory.remember_rest_zone(*zone);
            }
        }
    }
}

fn remember_or_insert<T, F>(entries: &mut Vec<T>, max_entries: usize, entry: T, entity: F)
where
    T: Copy,
    F: Fn(&T) -> Entity,
{
    if let Some(existing) = entries
        .iter_mut()
        .find(|existing| entity(existing) == entity(&entry))
    {
        *existing = entry;
    } else {
        entries.push(entry);
    }

    entries.sort_by_key(|entry| std::cmp::Reverse(entity(entry).index()));
    entries.truncate(max_entries);
}
