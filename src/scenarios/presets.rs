//! Hardcoded scenario presets for quick launch.

/// Built-in scenario presets available from runtime hotkeys.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioPreset {
    /// Balanced agents/resources, stable population.
    Equilibrium,
    /// Few resources, agents compete and die.
    Scarcity,
    /// Many agents, resources depleted quickly.
    Overpopulation,
    /// Extreme decay rates, rapid boom-bust.
    StressTest,
}

impl ScenarioPreset {
    /// Return the scenario catalog key backed by `assets/scenarios/{name}.ron`.
    #[must_use]
    pub const fn to_scenario_name(self) -> &'static str {
        match self {
            Self::Equilibrium => "equilibrium",
            Self::Scarcity => "scarcity",
            Self::Overpopulation => "overpopulation",
            Self::StressTest => "stress_test",
        }
    }

    /// Return the human-readable preset label shown in UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Equilibrium => "Balanced ecosystem — stable emergent dynamics",
            Self::Scarcity => "Resource scarcity — competition and starvation cycles",
            Self::Overpopulation => "Overpopulation — rapid resource collapse",
            Self::StressTest => "Stress test — extreme pressure, fast cycles",
        }
    }

    /// Return the preset associated with a number key.
    #[must_use]
    pub const fn from_number(number: u8) -> Option<Self> {
        match number {
            1 => Some(Self::Equilibrium),
            2 => Some(Self::Scarcity),
            3 => Some(Self::Overpopulation),
            4 => Some(Self::StressTest),
            _ => None,
        }
    }

    /// Return the preset matching a scenario catalog key.
    #[must_use]
    pub fn from_scenario_name(name: &str) -> Option<Self> {
        Self::all()
            .into_iter()
            .find(|preset| preset.to_scenario_name() == name)
    }

    /// Return all hotkey presets in display order.
    #[must_use]
    pub const fn all() -> [Self; 4] {
        [
            Self::Equilibrium,
            Self::Scarcity,
            Self::Overpopulation,
            Self::StressTest,
        ]
    }
}
