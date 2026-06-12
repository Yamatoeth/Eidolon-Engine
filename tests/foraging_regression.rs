use bevy::prelude::*;
use emergent_sim::{
    ai::{ActionKind, DecisionOutput},
    engine::SimulationTime,
    simulation::{
        resource::{resource_consume_system, rest_recovery_system},
        Agent, AgentId, AgentState, CarriedResource, Needs, ResourceConsumed, ResourceDelivered,
        ResourceDepleted, ResourceKind, ResourceNode, StateKind, VillageStore, Zone, ZoneId,
        ZoneKind,
    },
};

#[test]
fn food_is_carried_before_it_satisfies_hunger_at_rest_zone() {
    let mut app = App::new();
    app.init_resource::<SimulationTime>()
        .add_event::<ResourceConsumed>()
        .add_event::<ResourceDelivered>()
        .add_event::<ResourceDepleted>()
        .add_systems(Update, resource_consume_system);

    let resource = app
        .world_mut()
        .spawn((
            ResourceNode::food(100.0, 1.0, 0.0),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let rest_zone = app
        .world_mut()
        .spawn((
            Zone {
                id: ZoneId(0),
                kind: ZoneKind::Rest,
                radius: 3.0,
            },
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
    assert!(cargo.amount > 0.0);
    assert_eq!(app.world().get::<Needs>(agent).unwrap().hunger, 0.8);
    assert!(
        app.world().get::<ResourceNode>(resource).unwrap().amount < 100.0,
        "pickup should reduce the source resource"
    );

    app.world_mut().entity_mut(agent).insert((
        DecisionOutput {
            action: ActionKind::Deliver,
            target: Some(rest_zone),
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
        app.world().get::<VillageStore>(rest_zone).unwrap().food > 0.0,
        "delivered food should enter village storage"
    );
    assert_eq!(app.world().get::<Needs>(agent).unwrap().hunger, 0.8);
}

#[test]
fn agents_inside_village_eat_from_shared_store() {
    let mut app = App::new();
    app.init_resource::<SimulationTime>()
        .add_systems(Update, rest_recovery_system);

    app.world_mut().spawn((
        Zone {
            id: ZoneId(0),
            kind: ZoneKind::Rest,
            radius: 4.0,
        },
        VillageStore {
            food: 30.0,
            capacity: 100.0,
        },
        Transform::from_xyz(5.0, 0.0, 0.0),
    ));
    let agent = app
        .world_mut()
        .spawn((
            Agent {
                id: AgentId(0),
                age: 0.0,
            },
            AgentState {
                current: StateKind::Resting,
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
}

#[test]
fn collect_picks_up_non_food_resources_in_range() {
    let mut app = App::new();
    app.init_resource::<SimulationTime>()
        .add_event::<ResourceConsumed>()
        .add_event::<ResourceDelivered>()
        .add_event::<ResourceDepleted>()
        .add_systems(Update, resource_consume_system);

    let resource = app
        .world_mut()
        .spawn((
            ResourceNode {
                kind: ResourceKind::Material,
                amount: 40.0,
                max_amount: 40.0,
                regen_rate: 0.0,
                is_depleted: false,
            },
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
                action: ActionKind::Collect,
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
        .expect("collect should pick up a non-food resource in range");
    assert_eq!(cargo.kind, ResourceKind::Material);
    assert_eq!(cargo.source, resource);
    assert!(
        app.world().get::<ResourceNode>(resource).unwrap().amount < 40.0,
        "collect should reduce the source resource"
    );
}
