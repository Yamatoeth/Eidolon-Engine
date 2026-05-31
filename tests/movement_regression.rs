use bevy::prelude::*;
use emergent_sim::{
    ai::AIPlugin,
    engine::SimulationTime,
    scenarios::ScenariosPlugin,
    simulation::{Agent, SimulationPlugin},
};

#[test]
fn agents_move_after_fixed_updates() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<SimulationTime>()
        .add_plugins((SimulationPlugin, AIPlugin, ScenariosPlugin));

    app.update();
    app.world_mut().run_schedule(PostStartup);

    let before = first_agent_position(&mut app).expect("default scenario should spawn agents");

    for _ in 0..120 {
        app.world_mut().resource_mut::<SimulationTime>().advance();
        app.world_mut().run_schedule(FixedUpdate);
    }

    let after = first_agent_position(&mut app).expect("agent should still exist");
    assert!(
        before.distance(after) > 0.1,
        "agent did not move enough: before={before:?} after={after:?}"
    );
}

fn first_agent_position(app: &mut App) -> Option<Vec3> {
    app.world_mut()
        .query_filtered::<&Transform, With<Agent>>()
        .iter(app.world())
        .next()
        .map(|transform| transform.translation)
}
