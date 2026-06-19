//! 3D in-world debug rendering with Bevy Gizmos.

use bevy::prelude::*;

use crate::ai::{AIDebugInfo, DecisionOutput};
use crate::observability::inspector::{InspectorSelected, ObservabilityConfig};
use crate::simulation::{Agent, AgentState, Needs, SpatialGrid, StateKind, Zone, ZoneKind};

type AgentOverlayItem<'a> = (
    &'a Transform,
    &'a AgentState,
    Option<&'a DecisionOutput>,
    Option<&'a AIDebugInfo>,
    Option<&'a InspectorSelected>,
);

type AgentNeedBarItem<'a> = (&'a Transform, &'a Needs, &'a AgentState);

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

    for (transform, _state, decision, debug, selected) in &agent_query {
        if selected.is_some() {
            draw_selected_highlight(&mut gizmos, transform.translation);
        }
        if let (Some(decision), Some(debug)) = (decision, debug) {
            draw_ai_score_bars(&mut gizmos, transform.translation, decision, debug);
        }
    }
}

/// Draw floating billboard need bars over agents with visible hunger or fatigue pressure.
pub fn agent_need_bars_system(
    config: Res<ObservabilityConfig>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    agent_query: Query<AgentNeedBarItem<'_>, With<Agent>>,
    mut gizmos: Gizmos,
) {
    if !config.overlays_enabled {
        return;
    }

    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let camera_transform = camera_transform.compute_transform();
    let right = camera_transform.rotation * Vec3::X;
    let up = camera_transform.rotation * Vec3::Y;
    let normal = -camera_transform.forward();
    let orientation = Quat::from_rotation_arc(Vec3::Z, *normal);

    for (transform, needs, state) in &agent_query {
        if needs.hunger < 0.15 && needs.fatigue < 0.15 {
            continue;
        }

        let value = needs.hunger.max(needs.fatigue).clamp(0.0, 1.0);
        let center = transform.translation + Vec3::Y * 2.2;
        draw_billboard_need_bar(&mut gizmos, center, right, up, value);
        draw_state_dot(&mut gizmos, center + up * 0.16, orientation, state.current);
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
        StateKind::Idle => Color::WHITE,
        StateKind::MovingToTarget => Color::srgb(0.0, 0.78, 0.92),
        StateKind::Eating | StateKind::Carrying => Color::srgb(0.96, 0.64, 0.38),
        StateKind::Resting => Color::srgb(0.61, 0.36, 0.90),
        StateKind::Exploring => Color::srgb(0.95, 0.82, 0.25),
        StateKind::Fleeing => Color::srgb(0.90, 0.22, 0.28),
    }
}

fn draw_billboard_need_bar(gizmos: &mut Gizmos, center: Vec3, right: Vec3, up: Vec3, value: f32) {
    const BAR_WIDTH: f32 = 1.0;
    const BAR_HEIGHT: f32 = 0.08;
    const BAR_SEGMENTS: usize = 5;

    let left = center - right * (BAR_WIDTH * 0.5);
    let background_color = Color::srgb(0.102, 0.102, 0.180);
    let foreground_color = need_value_color(value);
    let fill_width = BAR_WIDTH * value;

    for index in 0..BAR_SEGMENTS {
        let offset = -BAR_HEIGHT * 0.5
            + BAR_HEIGHT * (index as f32 / (BAR_SEGMENTS.saturating_sub(1)) as f32);
        let row_start = left + up * offset;
        if fill_width > 0.0 {
            gizmos.line(row_start, row_start + right * fill_width, foreground_color);
        }
        if fill_width < BAR_WIDTH {
            let fill_end = row_start + right * fill_width;
            gizmos.line(fill_end, row_start + right * BAR_WIDTH, background_color);
        }
    }
}

fn draw_state_dot(gizmos: &mut Gizmos, center: Vec3, orientation: Quat, state: StateKind) {
    gizmos.circle(
        Isometry3d::new(center, orientation),
        0.055,
        agent_state_color(state),
    );
}

fn need_value_color(value: f32) -> Color {
    if value > 0.7 {
        Color::srgb(0.90, 0.22, 0.28)
    } else if value >= 0.4 {
        Color::srgb(0.95, 0.82, 0.25)
    } else {
        Color::srgb(0.14, 0.53, 0.21)
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
