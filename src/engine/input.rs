//! Input handling and game control mapping

use bevy::prelude::*;

use crate::engine::render::DebugGridConfig;
use crate::engine::time::SimulationTime;
use crate::scenarios::loader::ScenarioLoadRequested;
use crate::scenarios::presets::ScenarioPreset;

/// Engine-level actions produced from raw input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineAction {
    /// Toggle the deterministic simulation clock.
    TogglePause,
    /// Reset the deterministic simulation clock.
    ResetSimulationTime,
    /// Toggle the entity inspector panel.
    ToggleInspector,
    /// Toggle the event timeline panel.
    ToggleTimeline,
    /// Toggle in-world observability overlays.
    ToggleOverlays,
    /// Toggle the engine debug grid overlay.
    ToggleDebugGrid,
    /// Load a hardcoded scenario preset.
    LoadPreset(ScenarioPreset),
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
                (KeyCode::F2, EngineAction::ToggleTimeline),
                (KeyCode::F3, EngineAction::ToggleOverlays),
                (KeyCode::KeyG, EngineAction::ToggleDebugGrid),
                (
                    KeyCode::Digit1,
                    EngineAction::LoadPreset(ScenarioPreset::Equilibrium),
                ),
                (
                    KeyCode::Digit2,
                    EngineAction::LoadPreset(ScenarioPreset::Scarcity),
                ),
                (
                    KeyCode::Digit3,
                    EngineAction::LoadPreset(ScenarioPreset::Overpopulation),
                ),
                (
                    KeyCode::Digit4,
                    EngineAction::LoadPreset(ScenarioPreset::StressTest),
                ),
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
    mut scenario_load_requests: EventWriter<ScenarioLoadRequested>,
) {
    for event in events.read() {
        match event.action {
            EngineAction::TogglePause => sim_time.toggle_pause(),
            EngineAction::ResetSimulationTime => *sim_time = SimulationTime::new(),
            EngineAction::ToggleDebugGrid => debug_grid.enabled = !debug_grid.enabled,
            EngineAction::LoadPreset(preset) => {
                scenario_load_requests.send(ScenarioLoadRequested {
                    key: preset.to_scenario_name().to_owned(),
                });
            },
            EngineAction::ToggleInspector
            | EngineAction::ToggleTimeline
            | EngineAction::ToggleOverlays => {},
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
            input_map.actions_for_key(KeyCode::F2).collect::<Vec<_>>(),
            vec![EngineAction::ToggleTimeline]
        );
        assert_eq!(
            input_map.actions_for_key(KeyCode::F3).collect::<Vec<_>>(),
            vec![EngineAction::ToggleOverlays]
        );
        assert_eq!(
            input_map.actions_for_key(KeyCode::KeyG).collect::<Vec<_>>(),
            vec![EngineAction::ToggleDebugGrid]
        );
        assert_eq!(
            input_map
                .actions_for_key(KeyCode::Digit1)
                .collect::<Vec<_>>(),
            vec![EngineAction::LoadPreset(ScenarioPreset::Equilibrium)]
        );
        assert_eq!(
            input_map
                .actions_for_key(KeyCode::Digit4)
                .collect::<Vec<_>>(),
            vec![EngineAction::LoadPreset(ScenarioPreset::StressTest)]
        );
    }
}
