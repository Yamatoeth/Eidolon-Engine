//! Agent lifecycle, needs degradation, state transitions.

use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::ai::actions::ActionKind;
use crate::ai::decision::DecisionOutput;
use crate::engine::SimulationTime;
use crate::simulation::events::{
    AgentDied, AgentSpawned, DeathCause, NeedKind, NeedThresholdReached, ThresholdLevel,
};
use crate::simulation::spatial::Collider;
use crate::simulation::world::SimulationConfig;
use crate::simulation::{CarriedResource, ResourceNode, VillageStore};

const LOW_POPULATION_SPAWN_THRESHOLD: u32 = 8;
const SPAWN_RESOURCE_THRESHOLD: f32 = 200.0;
const SPAWN_VILLAGE_FOOD_THRESHOLD: f32 = 100.0;
const SPAWN_COOLDOWN_SECS: f32 = 15.0;
const SPAWN_BATCH_SIZE: u32 = 2;
const SPAWN_RADIUS_AROUND_STORE: f32 = 20.0;

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

/// Countdown controlling low-population recovery spawns.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct SpawnCooldown(pub f32);

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
    /// Total food available in resource nodes plus village stores.
    pub total_resource_available: f32,
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
    mut query: Query<(&mut DecisionOutput, &Transform, &Needs, &mut AgentState), With<Agent>>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;

    for (mut decision, transform, needs, mut state) in &mut query {
        let next = state_for_decision(&decision, transform.translation, needs, &state);
        if state.current != next {
            state.previous = state.current;
            state.current = next;
            state.time_in_state = 0.0;
            if state.current == StateKind::Resting {
                decision.target_position = None;
            }
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
        let should_move = !matches!(decision.action, ActionKind::Idle)
            && matches!(
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
        if distance <= movement_arrival_threshold(decision.action) {
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
    query: Query<(Entity, &Agent, &Needs)>,
    mut events: EventWriter<AgentDied>,
) {
    if sim_time.paused {
        return;
    }

    for (entity, agent, needs) in &query {
        let cause = death_cause(*needs);
        if let Some(cause) = cause {
            eprintln!(
                "[DEATH] cause={:?} hunger={:.2} fatigue={:.2} age={:.1}s",
                cause, needs.hunger, needs.fatigue, agent.age
            );
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
    resources: Query<&ResourceNode>,
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
    metrics.village_food = stores.iter().map(|store| store.food_amount).sum();
    metrics.total_resource_available = resources
        .iter()
        .map(|resource| resource.amount)
        .sum::<f32>()
        + metrics.village_food;
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

/// Spawn replacement agents when population is low and stored food can support recovery.
#[allow(clippy::too_many_arguments)]
pub fn agent_spawn_system(
    mut commands: Commands,
    sim_time: Res<SimulationTime>,
    config: Res<SimulationConfig>,
    metrics: Res<SimulationMetrics>,
    mut cooldown: ResMut<SpawnCooldown>,
    mut rng: ResMut<SimRng>,
    agents: Query<&Agent>,
    stores: Query<(&Transform, &VillageStore)>,
    mut events: EventWriter<AgentSpawned>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;
    cooldown.0 = (cooldown.0 - dt).max(0.0);
    if cooldown.0 > 0.0
        || metrics.agent_count >= LOW_POPULATION_SPAWN_THRESHOLD
        || metrics.total_resource_available <= SPAWN_RESOURCE_THRESHOLD
        || metrics.village_food <= SPAWN_VILLAGE_FOOD_THRESHOLD
    {
        return;
    }

    let Some(store_position) = nearest_store_to_world_center(&config, &stores) else {
        return;
    };

    let mut next_id = agents
        .iter()
        .map(|agent| agent.id.0)
        .max()
        .map_or(0, |id| id.saturating_add(1));

    for _ in 0..SPAWN_BATCH_SIZE {
        let direction = rng.next_xz_direction();
        let distance = rng.next_in_range(0.0, SPAWN_RADIUS_AROUND_STORE);
        let mut position = store_position + direction * distance;
        position.x = position.x.clamp(0.0, config.world_size.x);
        position.y = config.agent_visual_height;
        position.z = position.z.clamp(0.0, config.world_size.y);

        let entity = commands
            .spawn((
                Agent {
                    id: AgentId(next_id),
                    age: 0.0,
                },
                Needs {
                    hunger: 0.3,
                    fatigue: 0.1,
                    energy: 0.9,
                },
                AgentState::default(),
                Velocity::default(),
                Collider {
                    radius: config.agent_collider_radius,
                },
                Transform::from_translation(position),
            ))
            .id();
        events.send(AgentSpawned {
            agent: entity,
            position,
        });
        next_id = next_id.saturating_add(1);
    }

    eprintln!(
        "[SPAWN] spawned 2 agents, population was {}",
        metrics.agent_count
    );
    cooldown.0 = SPAWN_COOLDOWN_SECS;
}

fn nearest_store_to_world_center(
    config: &SimulationConfig,
    stores: &Query<(&Transform, &VillageStore)>,
) -> Option<Vec3> {
    let world_center = Vec3::new(config.world_size.x * 0.5, 0.0, config.world_size.y * 0.5);
    stores
        .iter()
        .filter(|(_, store)| store.food_amount > 0.0)
        .min_by(|(left_transform, _), (right_transform, _)| {
            left_transform
                .translation
                .distance(world_center)
                .total_cmp(&right_transform.translation.distance(world_center))
        })
        .map(|(transform, _)| transform.translation)
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

fn state_for_decision(
    decision: &DecisionOutput,
    position: Vec3,
    needs: &Needs,
    state: &AgentState,
) -> StateKind {
    // Leave Resting only when genuinely rested or critically hungry.
    if state.current == StateKind::Resting
        && needs.hunger < 0.85
        && (state.time_in_state < 2.0 || needs.fatigue >= 0.08)
    {
        return StateKind::Resting;
    }

    match decision.action {
        ActionKind::Idle | ActionKind::Collect => StateKind::Idle,
        ActionKind::Explore => StateKind::Exploring,
        ActionKind::MoveTo => StateKind::MovingToTarget,
        ActionKind::Deliver => {
            state_for_target_action(decision, position, StateKind::Carrying, 4.0)
        },
        ActionKind::Eat => state_for_target_action(decision, position, StateKind::Eating, 2.5),
        ActionKind::Rest => state_for_target_action(decision, position, StateKind::Resting, 5.0),
    }
}

fn state_for_target_action(
    decision: &DecisionOutput,
    position: Vec3,
    arrived_state: StateKind,
    threshold: f32,
) -> StateKind {
    let Some(target) = decision.target_position else {
        return StateKind::Idle;
    };
    let distance = position.distance(target);
    if distance <= threshold {
        arrived_state
    } else {
        StateKind::MovingToTarget
    }
}

fn movement_arrival_threshold(action: ActionKind) -> f32 {
    match action {
        ActionKind::Eat => 2.5,
        ActionKind::Deliver => 4.0,
        ActionKind::Rest => 5.0,
        _ => 0.2,
    }
}

fn death_cause(needs: Needs) -> Option<DeathCause> {
    if needs.hunger >= 1.0 {
        Some(DeathCause::Starvation)
    } else if needs.fatigue >= 1.0 || needs.energy <= 0.0 && needs.fatigue >= 0.8 {
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
