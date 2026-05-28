//! Typed simulation events and event queue.

use bevy::prelude::*;

use crate::simulation::ResourceKind;

/// Agent's needs crossed a threshold.
#[derive(Event, Clone, Copy, Debug, PartialEq)]
pub struct NeedThresholdReached {
    /// Agent entity whose need crossed the threshold.
    pub agent: Entity,
    /// Need that crossed the threshold.
    pub need: NeedKind,
    /// Threshold level reached by the need.
    pub level: ThresholdLevel,
}

/// Biological need kinds tracked by agents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeedKind {
    /// Hunger rises toward starvation.
    Hunger,
    /// Fatigue rises toward exhaustion.
    Fatigue,
    /// Energy falls toward exhaustion.
    Energy,
}

/// Coarse need urgency thresholds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThresholdLevel {
    /// Need has entered the warning range.
    Warning,
    /// Need has entered the critical range.
    Critical,
}

/// Agent died and was removed from the simulation.
#[derive(Event, Clone, Copy, Debug, PartialEq)]
pub struct AgentDied {
    /// Removed agent entity.
    pub agent: Entity,
    /// Cause of death.
    pub cause: DeathCause,
}

/// Agent death causes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeathCause {
    /// Hunger reached its fatal threshold.
    Starvation,
    /// Fatigue or energy reached a fatal threshold.
    Exhaustion,
}

/// Agent spawned into the simulation.
#[derive(Event, Clone, Copy, Debug, PartialEq)]
pub struct AgentSpawned {
    /// Spawned agent entity.
    pub agent: Entity,
    /// Initial world position.
    pub position: Vec3,
}

/// Agent consumed part of a resource node.
#[derive(Event, Clone, Copy, Debug, PartialEq)]
pub struct ResourceConsumed {
    /// Agent entity that consumed the resource.
    pub agent: Entity,
    /// Resource entity consumed from.
    pub resource: Entity,
    /// Amount consumed.
    pub amount: f32,
    /// Resource kind.
    pub kind: ResourceKind,
}

/// Resource node reached zero available amount.
#[derive(Event, Clone, Copy, Debug, PartialEq)]
pub struct ResourceDepleted {
    /// Depleted resource entity.
    pub resource: Entity,
    /// Resource world position.
    pub position: Vec3,
    /// Resource kind.
    pub kind: ResourceKind,
}

/// Resource node recovered above its replenishment threshold.
#[derive(Event, Clone, Copy, Debug, PartialEq)]
pub struct ResourceReplenished {
    /// Replenished resource entity.
    pub resource: Entity,
}
