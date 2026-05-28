use emergent_sim::scenarios::loader::load_scenario_catalog_from_path;
use emergent_sim::simulation::SimRng;

#[test]
fn simulation_rng_replays_same_sequence_for_same_seed() {
    let mut first = SimRng::from_seed(42);
    let mut second = SimRng::from_seed(42);

    for _ in 0..128 {
        assert_eq!(first.next_xz_direction(), second.next_xz_direction());
        assert_eq!(
            first.next_in_range(-10.0, 10.0),
            second.next_in_range(-10.0, 10.0)
        );
    }
}

#[test]
fn scenario_catalog_is_replay_loadable() {
    let catalog = load_scenario_catalog_from_path("assets/scenarios/index.ron")
        .expect("scenario catalog should load");

    assert!(catalog
        .entries
        .iter()
        .any(|entry| entry.key == "equilibrium"));
    assert!(catalog.entries.iter().any(|entry| entry.key == "scarcity"));
    assert!(catalog.entries.iter().any(|entry| entry.key == "island"));
    assert!(catalog
        .entries
        .iter()
        .any(|entry| entry.key == "overpopulation"));
    assert!(catalog
        .entries
        .iter()
        .any(|entry| entry.key == "stress_test"));
}
