//! 3D in-world debug rendering with Bevy Gizmos.

use bevy::prelude::*;

use crate::simulation::{SpatialGrid, Zone, ZoneKind};

/// Draw Phase 2 zone radii and populated spatial cells.
pub fn static_world_overlay_system(
    zone_query: Query<(&Transform, &Zone)>,
    spatial_grid: Res<SpatialGrid>,
    mut gizmos: Gizmos,
) {
    for (transform, zone) in &zone_query {
        gizmos.circle(
            Isometry3d::new(
                transform.translation + Vec3::Y * 0.08,
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
            0.06,
            cell.z as f32 * spatial_grid.cell_size() + half_cell,
        );
        let color = if entities.len() > 3 {
            Color::srgba(0.9, 0.2, 0.14, 0.55)
        } else {
            Color::srgba(0.78, 0.78, 0.82, 0.35)
        };

        gizmos.rect(
            Isometry3d::new(center, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
            Vec2::splat(spatial_grid.cell_size()),
            color,
        );
    }
}

fn zone_color(kind: ZoneKind) -> Color {
    match kind {
        ZoneKind::Resource => Color::srgb(0.18, 0.78, 0.26),
        ZoneKind::Rest => Color::srgb(0.24, 0.42, 0.94),
        ZoneKind::Neutral => Color::srgb(0.65, 0.65, 0.68),
        ZoneKind::Hazard => Color::srgb(0.9, 0.22, 0.14),
    }
}
