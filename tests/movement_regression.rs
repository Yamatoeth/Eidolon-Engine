use bevy::prelude::*;
use emergent_sim::{
    ai::{AIPlugin, ActionKind, DecisionOutput},
    engine::SimulationTime,
    scenarios::ScenariosPlugin,
    simulation::{
        agent::agent_movement_system, Agent, AgentId, SimulationConfig, SimulationPlugin, Velocity,
    },
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

#[test]
fn collect_decisions_move_toward_target_position() {
    let mut app = App::new();
    app.init_resource::<SimulationTime>()
        .insert_resource(SimulationConfig {
            agent_move_speed: 6.0,
            ..SimulationConfig::default()
        })
        .add_systems(Update, agent_movement_system);

    let agent = app
        .world_mut()
        .spawn((
            Agent {
                id: AgentId(0),
                age: 0.0,
            },
            Velocity::default(),
            DecisionOutput {
                action: ActionKind::Collect,
                target: None,
                target_position: Some(Vec3::new(10.0, 0.0, 0.0)),
                score: 1.0,
                last_decision_time: 0.0,
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    app.update();

    let transform = app.world().get::<Transform>(agent).unwrap();
    assert!(
        transform.translation.x > 0.0,
        "collect decision should move toward target: {:?}",
        transform.translation
    );
}
