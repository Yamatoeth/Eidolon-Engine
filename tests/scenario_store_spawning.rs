use bevy::prelude::*;
use emergent_sim::{
    scenarios::{
        builder::spawn_active_scenario_system,
        loader::{load_scenario_catalog_from_path, ActiveScenario},
    },
    simulation::{AgentSpawned, VillageStore, ZoneKind},
};

#[test]
fn every_catalog_scenario_spawns_one_village_store_per_rest_zone() {
    let catalog = load_scenario_catalog_from_path("assets/scenarios/index.ron").unwrap();

    for entry in catalog.entries {
        let expected_store_count = entry
            .config
            .zones
            .iter()
            .filter(|zone| zone.kind == ZoneKind::Rest)
            .count();
        assert!(
            expected_store_count > 0,
            "{} must define at least one Rest zone",
            entry.key
        );

        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(entry.config.simulation_config())
            .insert_resource(ActiveScenario {
                config: entry.config,
            })
            .add_event::<AgentSpawned>()
            .add_systems(Update, spawn_active_scenario_system);

        app.update();

        let actual_store_count = app
            .world_mut()
            .query::<&VillageStore>()
            .iter(app.world())
            .count();
        assert_eq!(
            actual_store_count, expected_store_count,
            "{} should spawn exactly one VillageStore per Rest zone",
            entry.key
        );
    }
}
