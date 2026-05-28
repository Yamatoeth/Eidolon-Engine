//! Observability Layer — Inspector, timeline, replay, overlays
//!
//! Responsible for all debug and inspection tooling. Reads ECS state but never mutates simulation.

use bevy::prelude::*;

#[cfg(feature = "observability")]
use bevy_inspector_egui::{bevy_egui::EguiPlugin, DefaultInspectorConfigPlugin};

#[cfg(feature = "observability")]
pub mod inspector;
#[cfg(feature = "debug_overlays")]
pub mod overlays;
#[cfg(feature = "observability")]
pub mod replay;
#[cfg(feature = "observability")]
pub mod timeline;

pub struct ObservabilityPlugin;

impl Plugin for ObservabilityPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "observability")]
        {
            app.add_plugins((EguiPlugin, DefaultInspectorConfigPlugin))
                .init_resource::<inspector::ObservabilityConfig>()
                .init_resource::<inspector::InspectorState>()
                .init_resource::<timeline::EventTimeline>()
                .init_resource::<replay::ReplayBuffer>()
                .add_systems(
                    Update,
                    (
                        inspector::handle_observability_actions,
                        inspector::click_to_inspect_system,
                        inspector::sync_inspector_selection_system,
                        inspector::inspector_ui_system,
                        inspector::scenario_selector_ui_system,
                        timeline::timeline_ui_system,
                        replay::replay_ui_system,
                    ),
                )
                .add_systems(
                    FixedUpdate,
                    (
                        timeline::timeline_record_agents_system,
                        timeline::timeline_record_resources_system,
                        replay::replay_record_system,
                    ),
                );
        }

        #[cfg(feature = "debug_overlays")]
        {
            app.add_systems(Update, overlays::static_world_overlay_system);
        }

        #[cfg(not(feature = "observability"))]
        let _ = app;
    }
}
