//! 3D in-world debug rendering with Bevy Gizmos.

use bevy::prelude::*;

use crate::ai::{AIDebugInfo, DecisionOutput};
use crate::observability::inspector::{InspectorSelected, ObservabilityConfig};
use crate::simulation::{Agent, AgentState, Needs, SpatialGrid, StateKind, Zone, ZoneKind};

type AgentOverlayItem<'a> = (
    &'a Transform,
    &'a Needs,
    &'a AgentState,
    Option<&'a DecisionOutput>,
    Option<&'a AIDebugInfo>,
    Option<&'a InspectorSelected>,
);

/// Draw static world overlays plus Phase 3 agent need/state markers.
pub fn static_world_overlay_system(
    zone_query: Query<(&Transform, &Zone)>,
    agent_query: Query<AgentOverlayItem<'_>, With<Agent>>,
    spatial_grid: Res<SpatialGrid>,
    config: Res<ObservabilityConfig>,
    mut gizmos: Gizmos,
) {
    if !config.overlays_enabled {
        return;
    }

    for (transform, zone) in &zone_query {
        gizmos.circle(
            Isometry3d::new(
                transform.translation + Vec3::Y * 0.10,
                Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
            ),
            zone.radius,
            zone_color(zone.kind),
        );
    }

    let half_cell = spatial_grid.cell_size() * 0.5;
    for (cell, entities) in spatial_grid.populated_cells() {
        let center = Vec3::new(
            cell.x as f32 * spatial_grid.cell_size() + half_cell,
            0.055,
            cell.z as f32 * spatial_grid.cell_size() + half_cell,
        );
        let color = if entities.len() > 3 {
            Color::srgba(1.0, 0.34, 0.24, 0.46)
        } else {
            Color::srgba(0.42, 0.58, 0.64, 0.24)
        };

        gizmos.rect(
            Isometry3d::new(center, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
            Vec2::splat(spatial_grid.cell_size()),
            color,
        );
    }

    for (transform, needs, state, decision, debug, selected) in &agent_query {
        draw_agent_need_bars(&mut gizmos, transform.translation, needs, state.current);
        if selected.is_some() {
            draw_selected_highlight(&mut gizmos, transform.translation);
        }
        if let (Some(decision), Some(debug)) = (decision, debug) {
            draw_ai_score_bars(&mut gizmos, transform.translation, decision, debug);
        }
    }
}

fn zone_color(kind: ZoneKind) -> Color {
    match kind {
        ZoneKind::Resource => Color::srgb(0.18, 0.78, 0.26),
        ZoneKind::Rest => Color::srgb(0.36, 0.50, 1.0),
        ZoneKind::Neutral => Color::srgb(0.58, 0.66, 0.68),
        ZoneKind::Hazard => Color::srgb(1.0, 0.28, 0.18),
    }
}

fn draw_agent_need_bars(gizmos: &mut Gizmos, position: Vec3, needs: &Needs, state: StateKind) {
    let origin = position + Vec3::new(-0.45, 1.0, 0.0);
    draw_bar(
        gizmos,
        origin,
        needs.hunger,
        Color::srgb(0.18, 0.78, 0.26),
        Color::srgb(1.0, 0.30, 0.18),
    );
    draw_bar(
        gizmos,
        origin + Vec3::Y * 0.14,
        needs.fatigue,
        Color::srgb(0.36, 0.50, 1.0),
        Color::srgb(0.95, 0.76, 0.20),
    );

    gizmos.circle(
        Isometry3d::new(
            position + Vec3::Y * 1.18,
            Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
        ),
        0.5,
        agent_state_color(state),
    );
}

fn draw_bar(gizmos: &mut Gizmos, origin: Vec3, value: f32, low_color: Color, high_color: Color) {
    let width = 0.9;
    let clamped = value.clamp(0.0, 1.0);
    let end = origin + Vec3::X * width;
    let fill_end = origin + Vec3::X * (width * clamped);

    let fill_color = if clamped >= 0.7 {
        high_color
    } else {
        low_color
    };

    gizmos.line(origin, end, Color::srgba(0.05, 0.05, 0.05, 0.9));
    gizmos.line(origin, fill_end, fill_color);
}

fn draw_selected_highlight(gizmos: &mut Gizmos, position: Vec3) {
    gizmos.circle(
        Isometry3d::new(
            position + Vec3::Y * 0.04,
            Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
        ),
        0.9,
        Color::srgb(1.0, 0.92, 0.18),
    );
}

fn agent_state_color(state: StateKind) -> Color {
    match state {
        StateKind::Idle => Color::srgb(0.70, 0.77, 0.80),
        StateKind::Exploring | StateKind::MovingToTarget => Color::srgb(0.24, 0.82, 0.92),
        StateKind::Eating => Color::srgb(0.45, 0.90, 0.52),
        StateKind::Resting => Color::srgb(0.50, 0.62, 1.0),
        StateKind::Fleeing => Color::srgb(1.0, 0.34, 0.24),
    }
}

fn draw_ai_score_bars(
    gizmos: &mut Gizmos,
    position: Vec3,
    decision: &DecisionOutput,
    debug: &AIDebugInfo,
) {
    let base = position + Vec3::new(-0.45, 1.38, 0.0);
    for (index, (action, score)) in debug.last_scores.iter().take(5).enumerate() {
        let y = index as f32 * 0.08;
        let color = if *action == decision.action {
            Color::srgb(0.98, 0.92, 0.28)
        } else {
            Color::srgba(0.78, 0.82, 0.88, 0.75)
        };
        gizmos.line(
            base + Vec3::Y * y,
            base + Vec3::new(0.75 * score.clamp(0.0, 1.0), y, 0.0),
            color,
        );
    }
}
