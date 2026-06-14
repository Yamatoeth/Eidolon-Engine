//! Global simulation constraints and pressures.

use bevy::prelude::*;

use crate::engine::SimulationTime;
use crate::simulation::{
    Agent, AgentState, Needs, ResourceNode, SimulationConfig, SimulationMetrics, StateKind,
};

const RESOURCE_COMPETITION_RADIUS: f32 = 8.0;
const MIN_COMPETING_AGENTS: usize = 3;
const MAX_COMPETITION_DIVISOR: usize = 4;
const HIGH_POPULATION_THRESHOLD: u32 = 20;
const LOW_POPULATION_THRESHOLD: u32 = 6;
const HIGH_POPULATION_REGEN_MULTIPLIER: f32 = 0.65;
const LOW_POPULATION_REGEN_MULTIPLIER: f32 = 1.5;

/// Consumption multiplier applied to agents under resource competition pressure.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CompetitionFactor(pub f32);

impl Default for CompetitionFactor {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Global resource regeneration multiplier driven by living population count.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct RegenPressureMultiplier(pub f32);

impl Default for RegenPressureMultiplier {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Apply hunger-driven fatigue pressure after baseline needs decay.
pub fn needs_cascade_system(
    sim_time: Res<SimulationTime>,
    config: Res<SimulationConfig>,
    mut agents: Query<(&mut Needs, &AgentState), With<Agent>>,
) {
    if sim_time.paused {
        return;
    }

    let dt = crate::engine::time::FIXED_TIMESTEP;
    let fatigue_rate = config.needs_decay_rates.fatigue_per_sec * config.global_decay_multiplier;

    for (mut needs, state) in &mut agents {
        let extra_multiplier = if needs.hunger >= 0.9 {
            1.0
        } else if needs.hunger >= 0.7 {
            0.5
        } else {
            0.0
        };

        if state.current != StateKind::Resting && extra_multiplier > 0.0 {
            needs.fatigue = (needs.fatigue + fatigue_rate * extra_multiplier * dt).clamp(0.0, 1.0);
        }
    }
}

/// Scale down per-agent resource consumption when agents crowd the same node.
pub fn resource_competition_system(
    mut commands: Commands,
    sim_time: Res<SimulationTime>,
    agents: Query<(Entity, &Transform), With<Agent>>,
    resources: Query<&Transform, With<ResourceNode>>,
) {
    if sim_time.paused {
        return;
    }

    let agent_positions = agents.iter().collect::<Vec<_>>();
    let mut factors = agent_positions
        .iter()
        .map(|(entity, _)| (*entity, CompetitionFactor::default()))
        .collect::<Vec<_>>();

    for resource_transform in &resources {
        let competitors = agent_positions
            .iter()
            .enumerate()
            .filter_map(|(index, (_, agent_transform))| {
                let distance = agent_transform
                    .translation
                    .distance(resource_transform.translation);
                (distance <= RESOURCE_COMPETITION_RADIUS).then_some(index)
            })
            .collect::<Vec<_>>();

        if competitors.len() < MIN_COMPETING_AGENTS {
            continue;
        }

        let divisor = competitors.len().min(MAX_COMPETITION_DIVISOR) as f32;
        let factor = CompetitionFactor(1.0 / divisor);
        for index in competitors {
            let current = &mut factors[index].1;
            current.0 = current.0.min(factor.0);
        }
    }

    for (entity, factor) in factors {
        commands.entity(entity).insert(factor);
    }
}

/// Update global regeneration pressure from current population metrics.
pub fn population_pressure_system(
    metrics: Res<SimulationMetrics>,
    mut multiplier: ResMut<RegenPressureMultiplier>,
) {
    multiplier.0 = if metrics.agent_count > HIGH_POPULATION_THRESHOLD {
        HIGH_POPULATION_REGEN_MULTIPLIER
    } else if metrics.agent_count <= LOW_POPULATION_THRESHOLD {
        LOW_POPULATION_REGEN_MULTIPLIER
    } else {
        1.0
    };
}
