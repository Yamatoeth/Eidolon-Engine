//! Time and simulation clock management

use bevy::prelude::*;

/// Fixed timestep for simulation (60 Hz)
pub const FIXED_TIMESTEP: f32 = 1.0 / 60.0;

/// Global simulation time, separate from Bevy's Time for determinism
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct SimulationTime {
    /// Elapsed simulation time in seconds
    pub elapsed: f32,
    /// Current simulation tick (incremented each FixedUpdate)
    pub tick: u64,
    /// Whether simulation is paused
    pub paused: bool,
}

impl SimulationTime {
    /// Create a new simulation time
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            tick: 0,
            paused: false,
        }
    }

    /// Advance simulation time by one fixed timestep
    pub fn advance(&mut self) {
        if !self.paused {
            self.elapsed += FIXED_TIMESTEP;
            self.tick = self.tick.wrapping_add(1);
        }
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
}

/// Update simulation clock
pub fn update_simulation_time(mut sim_time: ResMut<SimulationTime>) {
    sim_time.advance();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_increments_elapsed_time_and_tick() {
        let mut sim_time = SimulationTime::new();

        sim_time.advance();

        assert_eq!(sim_time.tick, 1);
        assert!((sim_time.elapsed - FIXED_TIMESTEP).abs() < f32::EPSILON);
    }

    #[test]
    fn paused_clock_does_not_advance() {
        let mut sim_time = SimulationTime::new();

        sim_time.toggle_pause();
        sim_time.advance();

        assert_eq!(sim_time.tick, 0);
        assert_eq!(sim_time.elapsed, 0.0);
    }
}
