//! Agent lifecycle, needs degradation, state transitions.

use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::ai::actions::ActionKind;
use crate::ai::decision::DecisionOutput;
use crate::engine::SimulationTime;
use crate::simulation::events::{
    AgentDied, DeathCause, NeedKind, NeedThresholdReached, ThresholdLevel,
};
use crate::simulation::world::SimulationConfig;
use crate::simulation::{CarriedResource, VillageStore};

/// Stable identifier for an agent entity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AgentId(pub u64);

/// Core identity and age of an agent.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Agent {
    /// Stable agent ID assigned at spawn.
    pub id: AgentId,
    /// Simulation seconds since spawn.
    pub age: f32,
}

/// Physical and biological needs tracked by the simulation.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Needs {
    /// Hunger urgency: `0.0` satisfied, `1.0` starving.
    pub hunger: f32,
    /// Fatigue urgency: `0.0` rested, `1.0` exhausted.
    pub fatigue: f32,
    /// Available energy: `1.0` full, `0.0` depleted.
    pub energy: f32,
}

impl Default for Needs {
    fn default() -> Self {
        Self {
            hunger: 0.0,
            fatigue: 0.0,
            energy: 1.0,
        }
    }
}

/// Internal agent state machine data.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AgentState {
    /// Current state.
    pub current: StateKind,
    /// Previous state.
    pub previous: StateKind,
    /// Simulation seconds spent in the current state.
    pub time_in_state: f32,
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            current: StateKind::Idle,
            previous: StateKind::Idle,
            time_in_state: 0.0,
        }
    }
}

/// Agent state categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateKind {
    /// No active movement.
    Idle,
    /// Reserved for later AI pathing.
    MovingToTarget,
    /// Reserved for later resource consumption.
    Eating,
    /// Reserved for later rest behavior.
    Resting,
    /// Returning to a rest zone while carrying gathered resources.
    Carrying,
    /// Dumb random walk movement.
    Exploring,
    /// Future competition or hazard response.
    Fleeing,
}

/// Linear velocity in world units per second.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Velocity {
    /// Linear X/Y/Z velocity.
    pub linear: Vec3,
}

impl Default for Velocity {
    fn default() -> Self {
        Self { linear: Vec3::ZERO }
    }
}

/// Deterministic simulation RNG.
#[derive(Resource, Debug, Clone)]
pub struct SimRng {
    rng: ChaCha8Rng,
}

impl Default for SimRng {
    fn default() -> Self {
        Self::from_seed(42)
    }
}

impl SimRng {
    /// Create a deterministic RNG from a scenario seed.
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Reset this RNG to a new deterministic seed.
    pub fn reseed(&mut self, seed: u64) {
        *self = Self::from_seed(seed);
    }

    /// Return a random `f32` in the provided range.
    pub fn next_in_range(&mut self, min: f32, max: f32) -> f32 {
        self.rng.gen_range(min..=max)
    }

    /// Return a random unit direction constrained to the X/Z plane.
    pub fn next_xz_direction(&mut self) -> Vec3 {
        let angle = self.next_in_range(0.0, std::f32::consts::TAU);
        Vec3::new(angle.cos(), 0.0, angle.sin())
    }
}

/// Rolling simulation metrics for observability panels.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct SimulationMetrics {
    /// Number of live agents.
    pub agent_count: u32,
    /// Average hunger across live agents.
    pub avg_hunger: f32,
    /// Average fatigue across live agents.
    pub avg_fatigue: f32,
    /// Average energy across live agents.
    pub avg_energy: f32,
    /// Number of agents currently carrying resources.
    pub carrying_count: u32,
    /// Total food stored in rest zones.
    pub village_food: f32,
}

/// Advance agent age and needs at the fixed simulation rate.
pub fn needs_decay_system(
    sim_time: Res<SimulationTime>,
    config: Res<SimulationConfig>,
    mut query: Query<(Entity, &mut Agent, &mut Needs)>,
    mut threshold_events: EventWriter<NeedThresholdReached>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;
    let decay_multiplier = config.global_decay_multiplier;

    for (entity, mut agent, mut needs) in &mut query {
        let before = *needs;
        agent.age += dt;
        apply_needs_decay(&mut needs, config.needs_decay_rates, decay_multiplier, dt);
        send_threshold_events(entity, before, *needs, &mut threshold_events);
    }
}

/// Update dumb random walk velocity and position.
pub fn random_walk_system(
    sim_time: Res<SimulationTime>,
    config: Res<SimulationConfig>,
    mut rng: ResMut<SimRng>,
    mut query: Query<(&mut Transform, &mut Velocity, &mut AgentState), With<Agent>>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;
    for (mut transform, mut velocity, mut state) in &mut query {
        state.time_in_state += dt;

        if state.current == StateKind::Idle || state.time_in_state >= config.random_walk_turn_secs {
            velocity.linear = rng.next_xz_direction() * config.agent_move_speed;
            transition_state(&mut state, StateKind::Exploring);
        }

        transform.translation += velocity.linear * dt;
        transform.translation.x = transform.translation.x.clamp(0.0, config.world_size.x);
        transform.translation.z = transform.translation.z.clamp(0.0, config.world_size.y);
        transform.translation.y = config.agent_visual_height;

        if velocity.linear.length_squared() > f32::EPSILON {
            let yaw = velocity.linear.x.atan2(velocity.linear.z);
            transform.rotation = Quat::from_rotation_y(yaw);
        }
    }
}

/// Transition agent state from the latest AI decision.
pub fn agent_state_transition_system(
    sim_time: Res<SimulationTime>,
    mut query: Query<(&DecisionOutput, &Transform, &mut AgentState), With<Agent>>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;

    for (decision, transform, mut state) in &mut query {
        let next = state_for_decision(decision, transform.translation);
        if state.current != next {
            state.previous = state.current;
            state.current = next;
            state.time_in_state = 0.0;
        } else {
            state.time_in_state += dt;
        }
    }
}

/// Move agents toward AI-selected target positions.
pub fn agent_movement_system(
    sim_time: Res<SimulationTime>,
    config: Res<SimulationConfig>,
    mut query: Query<(&DecisionOutput, &mut Transform, &mut Velocity), With<Agent>>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;

    for (decision, mut transform, mut velocity) in &mut query {
        let should_move = matches!(
            decision.action,
            ActionKind::MoveTo
                | ActionKind::Eat
                | ActionKind::Rest
                | ActionKind::Explore
                | ActionKind::Deliver
                | ActionKind::Collect
        );
        let Some(target) = decision.target_position.filter(|_| should_move) else {
            velocity.linear = Vec3::ZERO;
            continue;
        };

        let offset = Vec3::new(
            target.x - transform.translation.x,
            0.0,
            target.z - transform.translation.z,
        );
        let distance = offset.length();
        if distance <= 0.2
            || matches!(decision.action, ActionKind::Eat | ActionKind::Rest) && distance <= 1.2
        {
            velocity.linear = Vec3::ZERO;
            continue;
        }

        velocity.linear = offset.normalize() * config.agent_move_speed;
        transform.translation += velocity.linear * dt;
        transform.translation.x = transform.translation.x.clamp(0.0, config.world_size.x);
        transform.translation.z = transform.translation.z.clamp(0.0, config.world_size.y);
        transform.translation.y = config.agent_visual_height;

        let yaw = velocity.linear.x.atan2(velocity.linear.z);
        transform.rotation = Quat::from_rotation_y(yaw);
    }
}

/// Despawn agents whose needs have reached a fatal threshold.
pub fn agent_death_system(
    mut commands: Commands,
    sim_time: Res<SimulationTime>,
    query: Query<(Entity, &Needs), With<Agent>>,
    mut events: EventWriter<AgentDied>,
) {
    if sim_time.paused {
        return;
    }

    for (entity, needs) in &query {
        let cause = death_cause(*needs);
        if let Some(cause) = cause {
            commands.entity(entity).despawn();
            events.send(AgentDied {
                agent: entity,
                cause,
            });
        }
    }
}

/// Refresh aggregate simulation metrics for read-only UI.
pub fn metrics_update_system(
    agents: Query<(&Needs, Option<&CarriedResource>), With<Agent>>,
    stores: Query<&VillageStore>,
    mut metrics: ResMut<SimulationMetrics>,
) {
    let mut count = 0_u32;
    let mut hunger = 0.0;
    let mut fatigue = 0.0;
    let mut energy = 0.0;
    let mut carrying_count = 0_u32;

    for (needs, carried) in &agents {
        count = count.saturating_add(1);
        hunger += needs.hunger;
        fatigue += needs.fatigue;
        energy += needs.energy;
        if carried.is_some() {
            carrying_count = carrying_count.saturating_add(1);
        }
    }

    metrics.agent_count = count;
    metrics.carrying_count = carrying_count;
    metrics.village_food = stores.iter().map(|store| store.food).sum();
    if count == 0 {
        metrics.avg_hunger = 0.0;
        metrics.avg_fatigue = 0.0;
        metrics.avg_energy = 0.0;
    } else {
        let count = count as f32;
        metrics.avg_hunger = hunger / count;
        metrics.avg_fatigue = fatigue / count;
        metrics.avg_energy = energy / count;
    }
}

fn apply_needs_decay(
    needs: &mut Needs,
    rates: crate::simulation::world::NeedsDecayRates,
    multiplier: f32,
    dt: f32,
) {
    needs.hunger = (needs.hunger + rates.hunger_per_sec * multiplier * dt).clamp(0.0, 1.0);
    needs.fatigue = (needs.fatigue + rates.fatigue_per_sec * multiplier * dt).clamp(0.0, 1.0);
    needs.energy = (needs.energy - rates.energy_per_sec * multiplier * dt).clamp(0.0, 1.0);
}

fn send_threshold_events(
    agent: Entity,
    before: Needs,
    after: Needs,
    events: &mut EventWriter<NeedThresholdReached>,
) {
    send_rising_need_thresholds(agent, NeedKind::Hunger, before.hunger, after.hunger, events);
    send_rising_need_thresholds(
        agent,
        NeedKind::Fatigue,
        before.fatigue,
        after.fatigue,
        events,
    );
    send_falling_need_thresholds(agent, NeedKind::Energy, before.energy, after.energy, events);
}

fn send_rising_need_thresholds(
    agent: Entity,
    need: NeedKind,
    before: f32,
    after: f32,
    events: &mut EventWriter<NeedThresholdReached>,
) {
    if before < 0.7 && after >= 0.7 {
        events.send(NeedThresholdReached {
            agent,
            need,
            level: ThresholdLevel::Warning,
        });
    }
    if before < 0.9 && after >= 0.9 {
        events.send(NeedThresholdReached {
            agent,
            need,
            level: ThresholdLevel::Critical,
        });
    }
}

fn send_falling_need_thresholds(
    agent: Entity,
    need: NeedKind,
    before: f32,
    after: f32,
    events: &mut EventWriter<NeedThresholdReached>,
) {
    if before > 0.3 && after <= 0.3 {
        events.send(NeedThresholdReached {
            agent,
            need,
            level: ThresholdLevel::Warning,
        });
    }
    if before > 0.1 && after <= 0.1 {
        events.send(NeedThresholdReached {
            agent,
            need,
            level: ThresholdLevel::Critical,
        });
    }
}

fn transition_state(state: &mut AgentState, next: StateKind) {
    if state.current != next {
        state.previous = state.current;
        state.current = next;
    }
    state.time_in_state = 0.0;
}

fn state_for_decision(decision: &DecisionOutput, position: Vec3) -> StateKind {
    match decision.action {
        ActionKind::Idle | ActionKind::Collect => StateKind::Idle,
        ActionKind::Explore => StateKind::Exploring,
        ActionKind::MoveTo => StateKind::MovingToTarget,
        ActionKind::Deliver => StateKind::Carrying,
        ActionKind::Eat => state_for_target_action(decision, position, StateKind::Eating),
        ActionKind::Rest => state_for_target_action(decision, position, StateKind::Resting),
    }
}

fn state_for_target_action(
    decision: &DecisionOutput,
    position: Vec3,
    arrived_state: StateKind,
) -> StateKind {
    let Some(target) = decision.target_position else {
        return StateKind::Idle;
    };
    if position.distance(target) <= 1.2 {
        arrived_state
    } else {
        StateKind::MovingToTarget
    }
}

fn death_cause(needs: Needs) -> Option<DeathCause> {
    if needs.hunger >= 1.0 {
        Some(DeathCause::Starvation)
    } else if needs.fatigue >= 1.0 || needs.energy <= 0.0 {
        Some(DeathCause::Exhaustion)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::world::NeedsDecayRates;

    #[test]
    fn needs_decay_clamps_to_valid_range() {
        let mut needs = Needs {
            hunger: 0.99,
            fatigue: 0.99,
            energy: 0.01,
        };

        apply_needs_decay(
            &mut needs,
            NeedsDecayRates {
                hunger_per_sec: 1.0,
                fatigue_per_sec: 1.0,
                energy_per_sec: 1.0,
            },
            1.0,
            1.0,
        );

        assert_eq!(needs.hunger, 1.0);
        assert_eq!(needs.fatigue, 1.0);
        assert_eq!(needs.energy, 0.0);
    }

    #[test]
    fn starvation_takes_precedence_for_death_cause() {
        let needs = Needs {
            hunger: 1.0,
            fatigue: 1.0,
            energy: 0.0,
        };

        assert_eq!(death_cause(needs), Some(DeathCause::Starvation));
    }

    #[test]
    fn rng_repeats_for_same_seed() {
        let mut a = SimRng::from_seed(7);
        let mut b = SimRng::from_seed(7);

        assert_eq!(a.next_xz_direction(), b.next_xz_direction());
        assert_eq!(a.next_in_range(-5.0, 5.0), b.next_in_range(-5.0, 5.0));
    }
}
