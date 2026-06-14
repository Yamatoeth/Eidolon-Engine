use bevy::prelude::*;
use emergent_sim::{
    ai::decision::build_perception,
    simulation::{ResourceNode, SpatialGrid, VillageStore, Zone, ZoneId, ZoneKind},
};

#[derive(Resource)]
struct AgentUnderTest(Entity);

#[derive(Resource)]
struct RestZoneUnderTest(Entity);

#[derive(Resource, Default)]
struct PerceivedRestZone(Option<Entity>);

#[test]
fn rest_zones_use_dedicated_perception_radius() {
    let mut app = App::new();
    app.init_resource::<SpatialGrid>()
        .init_resource::<PerceivedRestZone>()
        .add_systems(Update, capture_rest_zone_perception);

    let agent = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(AgentUnderTest(agent));
    let rest_zone = app
        .world_mut()
        .spawn((
            Zone {
                id: ZoneId(0),
                kind: ZoneKind::Rest,
                radius: 3.0,
            },
            Transform::from_xyz(20.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut()
        .insert_resource(RestZoneUnderTest(rest_zone));

    app.update();

    assert_eq!(
        app.world().resource::<PerceivedRestZone>().0,
        Some(app.world().resource::<RestZoneUnderTest>().0)
    );
}

fn capture_rest_zone_perception(
    agent: Res<AgentUnderTest>,
    spatial_grid: Res<SpatialGrid>,
    resources: Query<(Entity, &Transform, &ResourceNode)>,
    zones: Query<(Entity, &Transform, &Zone)>,
    stores: Query<(Entity, &Transform, &VillageStore)>,
    mut perceived: ResMut<PerceivedRestZone>,
) {
    perceived.0 = build_perception(
        agent.0,
        Vec3::ZERO,
        5.0,
        25.0,
        &spatial_grid,
        &resources,
        &zones,
        &stores,
    )
    .nearest_rest_zone
    .map(|zone| zone.entity);
}
