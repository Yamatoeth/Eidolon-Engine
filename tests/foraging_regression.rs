use bevy::prelude::*;
use emergent_sim::{
    ai::{
        decision::ai_scoring_system, AIConfig, AIDebugInfo, ActionKind, AgentBehaviorLogged,
        AgentIntent, AgentMemory, AgentRole, DecisionOutput,
    },
    engine::SimulationTime,
    simulation::{
        resource::{resource_consume_system, rest_recovery_system},
        Agent, AgentId, AgentState, CarriedResource, Needs, ResourceConsumed, ResourceDepleted,
        ResourceKind, ResourceNode, SimRng, SpatialGrid, StateKind, VillageStore,
    },
};

#[test]
fn food_is_carried_before_it_satisfies_hunger_at_rest_zone() {
    let mut app = App::new();
    app.init_resource::<SimulationTime>()
        .add_event::<ResourceConsumed>()
        .add_event::<ResourceDepleted>()
        .add_systems(Update, resource_consume_system);

    let resource = app
        .world_mut()
        .spawn((
            ResourceNode::food(100.0, 1.0, 0.0),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let store = app
        .world_mut()
        .spawn((
            VillageStore::new(100.0),
            Transform::from_xyz(10.0, 0.0, 0.0),
        ))
        .id();
    let agent = app
        .world_mut()
        .spawn((
            Agent {
                id: AgentId(0),
                age: 0.0,
            },
            Needs {
                hunger: 0.8,
                fatigue: 0.0,
                energy: 0.5,
            },
            DecisionOutput {
                action: ActionKind::Eat,
                target: Some(resource),
                target_position: Some(Vec3::ZERO),
                score: 1.0,
                last_decision_time: 0.0,
            },
            Transform::from_xyz(0.0, 0.0, 1.0),
        ))
        .id();

    app.update();

    let cargo = app
        .world()
        .get::<CarriedResource>(agent)
        .expect("agent should pick up food before hunger is satisfied");
    assert_eq!(cargo.kind, ResourceKind::Food);
    assert_eq!(cargo.source, resource);
    assert_eq!(cargo.amount, 24.0);
    assert_eq!(app.world().get::<Needs>(agent).unwrap().hunger, 0.8);
    assert_eq!(
        app.world().get::<ResourceNode>(resource).unwrap().amount,
        76.0
    );

    app.world_mut().entity_mut(agent).insert((
        DecisionOutput {
            action: ActionKind::Deliver,
            target: Some(store),
            target_position: Some(Vec3::new(10.0, 0.0, 0.0)),
            score: 1.0,
            last_decision_time: 0.5,
        },
        Transform::from_xyz(10.0, 0.0, 0.0),
    ));

    app.update();

    assert!(
        app.world().get::<CarriedResource>(agent).is_none(),
        "cargo should be consumed when the agent reaches a rest zone"
    );
    assert!(
        app.world().get::<VillageStore>(store).unwrap().food_amount > 0.0,
        "delivered food should enter village storage"
    );
    assert_eq!(
        app.world().get::<VillageStore>(store).unwrap().food_amount,
        24.0
    );
    assert_eq!(app.world().get::<Needs>(agent).unwrap().hunger, 0.8);
}

#[test]
fn agents_inside_village_store_radius_eat_even_when_not_resting() {
    let mut app = App::new();
    app.init_resource::<SimulationTime>()
        .add_systems(Update, rest_recovery_system);

    let store = app
        .world_mut()
        .spawn((
            VillageStore {
                food_amount: 30.0,
                max_capacity: 100.0,
                radius: 3.0,
            },
            Transform::from_xyz(5.0, 0.0, 0.0),
        ))
        .id();
    let agent = app
        .world_mut()
        .spawn((
            Agent {
                id: AgentId(0),
                age: 0.0,
            },
            AgentState {
                current: StateKind::Idle,
                previous: StateKind::MovingToTarget,
                time_in_state: 0.0,
            },
            Needs {
                hunger: 0.8,
                fatigue: 0.4,
                energy: 0.5,
            },
            Transform::from_xyz(5.0, 0.0, 1.0),
        ))
        .id();

    app.update();

    assert!(
        app.world().get::<Needs>(agent).unwrap().hunger < 0.8,
        "agents in a village should be able to eat from the shared store"
    );
    assert!(
        app.world().get::<VillageStore>(store).unwrap().food_amount < 30.0,
        "feeding should spend food from the shared store"
    );
}

#[test]
fn depleted_food_resource_is_marked_depleted() {
    let mut app = App::new();
    app.init_resource::<SimulationTime>()
        .add_event::<ResourceConsumed>()
        .add_event::<ResourceDepleted>()
        .add_systems(Update, resource_consume_system);

    let resource = app
        .world_mut()
        .spawn((
            ResourceNode::food(12.0, 1.0, 0.0),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let agent = app
        .world_mut()
        .spawn((
            Agent {
                id: AgentId(0),
                age: 0.0,
            },
            Needs::default(),
            DecisionOutput {
                action: ActionKind::Eat,
                target: Some(resource),
                target_position: Some(Vec3::ZERO),
                score: 1.0,
                last_decision_time: 0.0,
            },
            Transform::from_xyz(0.0, 0.0, 1.0),
        ))
        .id();

    app.update();

    let cargo = app
        .world()
        .get::<CarriedResource>(agent)
        .expect("eat should pick up food in range");
    assert_eq!(cargo.kind, ResourceKind::Food);
    assert_eq!(cargo.source, resource);
    assert_eq!(cargo.amount, 12.0);
    let resource = app.world().get::<ResourceNode>(resource).unwrap();
    assert_eq!(resource.amount, 0.0);
    assert!(resource.is_depleted);
}

#[test]
fn ai_selects_deliver_to_visible_village_store_for_carried_food() {
    let mut app = App::new();
    app.init_resource::<SimulationTime>()
        .init_resource::<AIConfig>()
        .init_resource::<SimRng>()
        .init_resource::<SpatialGrid>()
        .add_event::<AgentBehaviorLogged>()
        .add_systems(Update, ai_scoring_system);

    let store = app
        .world_mut()
        .spawn((
            VillageStore {
                food_amount: 50.0,
                max_capacity: 500.0,
                radius: 3.0,
            },
            Transform::from_xyz(4.0, 0.0, 0.0),
        ))
        .id();
    let agent = app
        .world_mut()
        .spawn((
            Agent {
                id: AgentId(0),
                age: 0.0,
            },
            Needs {
                hunger: 0.2,
                fatigue: 0.0,
                energy: 1.0,
            },
            AgentState::default(),
            AgentRole::Forager,
            AgentMemory::default(),
            CarriedResource {
                kind: ResourceKind::Food,
                amount: 24.0,
                capacity: 24.0,
                source: store,
            },
            AgentIntent::default(),
            DecisionOutput::default(),
            AIDebugInfo::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    app.update();

    let decision = app.world().get::<DecisionOutput>(agent).unwrap();
    assert_eq!(decision.action, ActionKind::Deliver);
    assert_eq!(decision.target, Some(store));
    assert_eq!(decision.target_position, Some(Vec3::new(4.0, 0.0, 0.0)));
    assert!(decision.score > 0.7);

    let behavior_events = app.world().resource::<Events<AgentBehaviorLogged>>();
    let logged = behavior_events
        .iter_current_update_events()
        .find(|event| event.agent == agent)
        .expect("behavior change should be logged");
    assert_eq!(logged.previous_action, ActionKind::Idle);
    assert_eq!(logged.action, ActionKind::Deliver);
    assert_eq!(logged.intent, AgentIntent::Deliver { zone: store });
}
