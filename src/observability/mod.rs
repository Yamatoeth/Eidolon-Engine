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
                .add_systems(
                    Update,
                    (
                        inspector::handle_observability_actions,
                        inspector::inspector_ui_system,
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
