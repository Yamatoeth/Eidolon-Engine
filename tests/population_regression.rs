use bevy::prelude::*;
use emergent_sim::{
    engine::SimulationTime,
    simulation::{
        agent::agent_spawn_system, Agent, AgentSpawned, SimRng, SimulationConfig,
        SimulationMetrics, SpawnCooldown, VillageStore,
    },
};

#[test]
fn low_population_with_food_spawns_recovery_agents() {
    let mut app = App::new();
    app.init_resource::<SimulationTime>()
        .init_resource::<SimRng>()
        .init_resource::<SpawnCooldown>()
        .insert_resource(SimulationConfig::default())
        .insert_resource(SimulationMetrics {
            agent_count: 6,
            village_food: 150.0,
            total_resource_available: 300.0,
            ..default()
        })
        .add_event::<AgentSpawned>()
        .add_systems(Update, agent_spawn_system);

    app.world_mut().spawn((
        VillageStore {
            food_amount: 150.0,
            max_capacity: 1000.0,
            radius: 4.0,
        },
        Transform::from_xyz(50.0, 0.0, 50.0),
    ));

    app.update();

    assert_eq!(
        app.world_mut().query::<&Agent>().iter(app.world()).count(),
        2
    );
    assert!(app.world().resource::<SpawnCooldown>().0 > 0.0);
}
