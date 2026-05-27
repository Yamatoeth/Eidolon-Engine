//! Input handling and game control mapping

use bevy::prelude::*;

use crate::engine::render::DebugGridConfig;
use crate::engine::time::SimulationTime;

/// Engine-level actions produced from raw input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineAction {
    /// Toggle the deterministic simulation clock.
    TogglePause,
    /// Reset the deterministic simulation clock.
    ResetSimulationTime,
    /// Toggle the entity inspector panel.
    ToggleInspector,
    /// Toggle the engine debug grid overlay.
    ToggleDebugGrid,
}

/// Event emitted when an input mapping activates an engine action.
#[derive(Event, Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineActionEvent {
    /// The mapped action.
    pub action: EngineAction,
}

/// Keyboard mapping table for engine actions.
#[derive(Resource, Debug, Clone)]
pub struct InputMap {
    bindings: Vec<(KeyCode, EngineAction)>,
}

impl Default for InputMap {
    fn default() -> Self {
        Self {
            bindings: vec![
                (KeyCode::Space, EngineAction::TogglePause),
                (KeyCode::KeyR, EngineAction::ResetSimulationTime),
                (KeyCode::F1, EngineAction::ToggleInspector),
                (KeyCode::KeyG, EngineAction::ToggleDebugGrid),
            ],
        }
    }
}

impl InputMap {
    /// Returns all actions bound to the provided key.
    pub fn actions_for_key(&self, key: KeyCode) -> impl Iterator<Item = EngineAction> + '_ {
        self.bindings
            .iter()
            .filter_map(move |(bound_key, action)| (*bound_key == key).then_some(*action))
    }

    /// Returns all configured bindings in deterministic order.
    #[must_use]
    pub fn bindings(&self) -> &[(KeyCode, EngineAction)] {
        &self.bindings
    }
}

/// Convert just-pressed keys into engine action events.
pub fn handle_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    input_map: Res<InputMap>,
    mut events: EventWriter<EngineActionEvent>,
) {
    for (key, _) in input_map.bindings() {
        if keyboard.just_pressed(*key) {
            for action in input_map.actions_for_key(*key) {
                events.send(EngineActionEvent { action });
            }
        }
    }
}

/// Apply engine-owned actions to engine resources.
pub fn apply_engine_actions(
    mut events: EventReader<EngineActionEvent>,
    mut sim_time: ResMut<SimulationTime>,
    mut debug_grid: ResMut<DebugGridConfig>,
) {
    for event in events.read() {
        match event.action {
            EngineAction::TogglePause => sim_time.toggle_pause(),
            EngineAction::ResetSimulationTime => *sim_time = SimulationTime::new(),
            EngineAction::ToggleDebugGrid => debug_grid.enabled = !debug_grid.enabled,
            EngineAction::ToggleInspector => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_input_map_contains_phase_one_actions() {
        let input_map = InputMap::default();

        assert_eq!(
            input_map
                .actions_for_key(KeyCode::Space)
                .collect::<Vec<_>>(),
            vec![EngineAction::TogglePause]
        );
        assert_eq!(
            input_map.actions_for_key(KeyCode::F1).collect::<Vec<_>>(),
            vec![EngineAction::ToggleInspector]
        );
        assert_eq!(
            input_map.actions_for_key(KeyCode::KeyG).collect::<Vec<_>>(),
            vec![EngineAction::ToggleDebugGrid]
        );
    }
}
