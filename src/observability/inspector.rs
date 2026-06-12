//! Live entity/component browser with egui.

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

use crate::ai::{AIDebugInfo, AgentIntent, AgentMemory, AgentRole, DecisionOutput};
use crate::engine::{EngineAction, EngineActionEvent, SimulationTime};
use crate::scenarios::loader::{ScenarioCatalog, ScenarioLoadRequested};
use crate::simulation::{
    Agent, AgentState, CarriedResource, Needs, ResourceNode, SimulationMetrics, VillageStore, Zone,
};

type AgentInspectorItem<'a> = (
    &'a Agent,
    &'a Needs,
    &'a AgentState,
    Option<&'a AgentRole>,
    Option<&'a AgentIntent>,
    Option<&'a AgentMemory>,
    Option<&'a CarriedResource>,
    Option<&'a DecisionOutput>,
    Option<&'a AIDebugInfo>,
);

/// Runtime configuration for observability panels.
#[derive(Resource, Debug, Clone)]
pub struct ObservabilityConfig {
    /// Whether the entity inspector shell is visible.
    pub inspector_open: bool,
    /// Whether the future event timeline panel is visible.
    pub timeline_open: bool,
    /// Whether in-world debug overlays are enabled.
    pub overlays_enabled: bool,
    /// Maximum event entries retained by the timeline.
    pub timeline_max_entries: usize,
    /// Ticks between replay snapshot records.
    pub replay_record_interval_ticks: u32,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            inspector_open: false,
            timeline_open: false,
            overlays_enabled: true,
            timeline_max_entries: 1_000,
            replay_record_interval_ticks: 10,
        }
    }
}

/// Entity component filter used by the inspector list.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InspectorFilter {
    /// Show all supported entities.
    #[default]
    All,
    /// Show agents only.
    Agents,
    /// Show resources only.
    Resources,
    /// Show zones only.
    Zones,
}

/// Current inspector selection and filter state.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct InspectorState {
    /// Entity selected in the inspector.
    pub selected: Option<Entity>,
    /// Current list filter.
    pub filter: InspectorFilter,
}

/// Marker attached to the selected entity for overlay highlight rendering.
#[derive(Component, Debug, Clone, Copy)]
pub struct InspectorSelected;

/// Apply observability-owned engine actions to observability resources.
pub fn handle_observability_actions(
    mut events: EventReader<EngineActionEvent>,
    mut config: ResMut<ObservabilityConfig>,
) {
    for event in events.read() {
        match event.action {
            EngineAction::ToggleInspector => config.inspector_open = !config.inspector_open,
            EngineAction::ToggleTimeline => config.timeline_open = !config.timeline_open,
            EngineAction::ToggleOverlays => config.overlays_enabled = !config.overlays_enabled,
            EngineAction::TogglePause
            | EngineAction::ResetSimulationTime
            | EngineAction::ToggleDebugGrid
            | EngineAction::LoadPreset(_) => {},
        }
    }
}

/// Keep the `InspectorSelected` marker attached to the selected entity only.
pub fn sync_inspector_selection_system(
    mut commands: Commands,
    state: Res<InspectorState>,
    selected_entities: Query<Entity, With<InspectorSelected>>,
) {
    if !state.is_changed() {
        return;
    }

    for entity in &selected_entities {
        if Some(entity) != state.selected {
            commands.entity(entity).remove::<InspectorSelected>();
        }
    }

    if let Some(entity) = state.selected {
        commands.entity(entity).insert(InspectorSelected);
    }
}

/// Select the nearest supported entity under the cursor on left click.
pub fn click_to_inspect_system(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut state: ResMut<InspectorState>,
    agents: Query<(Entity, &Transform), With<Agent>>,
    resources: Query<(Entity, &Transform), With<ResourceNode>>,
    zones: Query<(Entity, &Transform, &Zone)>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(point) = cursor_ground_point(&windows, &camera_query) else {
        return;
    };

    state.selected = nearest_entity(point, &agents, &resources, &zones);
}

/// Draw the inspector entity browser and detail panel.
pub fn inspector_ui_system(
    mut contexts: EguiContexts,
    mut config: ResMut<ObservabilityConfig>,
    mut state: ResMut<InspectorState>,
    sim_time: Res<SimulationTime>,
    metrics: Res<SimulationMetrics>,
    zones: Query<(Entity, &Zone, &Transform, Option<&VillageStore>)>,
    resources: Query<(Entity, &ResourceNode, &Transform)>,
    agents: Query<(Entity, AgentInspectorItem<'_>, &Transform)>,
) {
    if !config.inspector_open {
        return;
    }

    egui::Window::new("Entity Inspector")
        .open(&mut config.inspector_open)
        .default_width(320.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Simulation");
            ui.separator();
            ui.label(format!("Tick: {}", sim_time.tick));
            ui.label(format!("Elapsed: {:.2}s", sim_time.elapsed));
            ui.label(format!(
                "State: {}",
                if sim_time.paused { "Paused" } else { "Running" }
            ));
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Filter");
                ui.selectable_value(&mut state.filter, InspectorFilter::All, "All");
                ui.selectable_value(&mut state.filter, InspectorFilter::Agents, "Agents");
                ui.selectable_value(&mut state.filter, InspectorFilter::Resources, "Resources");
                ui.selectable_value(&mut state.filter, InspectorFilter::Zones, "Zones");
            });
            ui.separator();
            ui.heading("Agents");
            ui.label(format!("Live agents: {}", metrics.agent_count));
            ui.label(format!("Average hunger: {:.2}", metrics.avg_hunger));
            ui.label(format!("Average fatigue: {:.2}", metrics.avg_fatigue));
            ui.label(format!("Average energy: {:.2}", metrics.avg_energy));
            ui.label(format!("Carrying: {}", metrics.carrying_count));
            ui.label(format!("Village food: {:.1}", metrics.village_food));
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .show(ui, |ui| {
                    draw_entity_list(ui, &mut state, &agents, &resources, &zones);
                });
            ui.separator();
            draw_selected_details(ui, state.selected, &agents, &resources, &zones);
            ui.separator();
            ui.heading("World");
            ui.label(format!("Zones: {}", zones.iter().count()));
            ui.label(format!("Resource nodes: {}", resources.iter().count()));
        });
}

/// Draw the scenario selector panel.
pub fn scenario_selector_ui_system(
    mut contexts: EguiContexts,
    config: Res<ObservabilityConfig>,
    catalog: Option<Res<ScenarioCatalog>>,
    mut scenario_load_requests: EventWriter<ScenarioLoadRequested>,
) {
    if !config.inspector_open {
        return;
    }

    egui::Window::new("Scenarios")
        .default_width(360.0)
        .show(contexts.ctx_mut(), |ui| {
            draw_scenario_selector(ui, catalog.as_deref(), &mut scenario_load_requests);
        });
}

fn draw_entity_list(
    ui: &mut egui::Ui,
    state: &mut InspectorState,
    agents: &Query<(Entity, AgentInspectorItem<'_>, &Transform)>,
    resources: &Query<(Entity, &ResourceNode, &Transform)>,
    zones: &Query<(Entity, &Zone, &Transform, Option<&VillageStore>)>,
) {
    if matches!(state.filter, InspectorFilter::All | InspectorFilter::Agents) {
        for (
            entity,
            (agent, _needs, agent_state, _role, intent, _memory, _cargo, decision, _debug),
            _transform,
        ) in agents.iter()
        {
            let label = match decision {
                Some(decision) => format!(
                    "Agent #{:03} {:?} {:?} {:?} {:.2}",
                    agent.id.0,
                    agent_state.current,
                    intent.copied().unwrap_or_default(),
                    decision.action,
                    decision.score
                ),
                None => format!("Agent #{:03} {:?}", agent.id.0, agent_state.current),
            };
            if ui
                .selectable_label(state.selected == Some(entity), label)
                .clicked()
            {
                state.selected = Some(entity);
            }
        }
    }

    if matches!(
        state.filter,
        InspectorFilter::All | InspectorFilter::Resources
    ) {
        for (entity, resource, _transform) in resources.iter() {
            let label = format!(
                "Resource {:?} {:.1}/{:.1}",
                resource.kind, resource.amount, resource.max_amount
            );
            if ui
                .selectable_label(state.selected == Some(entity), label)
                .clicked()
            {
                state.selected = Some(entity);
            }
        }
    }

    if matches!(state.filter, InspectorFilter::All | InspectorFilter::Zones) {
        for (entity, zone, _transform, store) in zones.iter() {
            let label = if let Some(store) = store {
                format!(
                    "Zone {:?} radius {:.1} food {:.1}/{:.1}",
                    zone.kind, zone.radius, store.food, store.capacity
                )
            } else {
                format!("Zone {:?} radius {:.1}", zone.kind, zone.radius)
            };
            if ui
                .selectable_label(state.selected == Some(entity), label)
                .clicked()
            {
                state.selected = Some(entity);
            }
        }
    }
}

fn draw_scenario_selector(
    ui: &mut egui::Ui,
    catalog: Option<&ScenarioCatalog>,
    scenario_load_requests: &mut EventWriter<ScenarioLoadRequested>,
) {
    ui.heading("Scenarios");
    let Some(catalog) = catalog else {
        ui.label("No scenario catalog loaded");
        return;
    };

    for entry in &catalog.entries {
        ui.horizontal(|ui| {
            let active = catalog.active_key.as_deref() == Some(entry.key.as_str());
            ui.label(if active { "*" } else { " " });
            ui.vertical(|ui| {
                ui.label(&entry.config.name);
                ui.small(&entry.config.description);
            });
            if ui.button("Load").clicked() {
                scenario_load_requests.send(ScenarioLoadRequested {
                    key: entry.key.clone(),
                });
            }
        });
    }
}

fn draw_selected_details(
    ui: &mut egui::Ui,
    selected: Option<Entity>,
    agents: &Query<(Entity, AgentInspectorItem<'_>, &Transform)>,
    resources: &Query<(Entity, &ResourceNode, &Transform)>,
    zones: &Query<(Entity, &Zone, &Transform, Option<&VillageStore>)>,
) {
    let Some(selected) = selected else {
        ui.label("No entity selected");
        return;
    };

    ui.heading("Selected");
    if let Ok((
        _entity,
        (agent, needs, state, role, intent, memory, cargo, decision, debug),
        transform,
    )) = agents.get(selected)
    {
        ui.label(format!("Agent #{}", agent.id.0));
        if let Some(role) = role {
            ui.label(format!("Role: {role:?}"));
        }
        if let Some(intent) = intent {
            ui.label(format!("Intent: {intent:?}"));
        }
        ui.label(format!(
            "Position: {}",
            format_position(transform.translation)
        ));
        ui.label(format!(
            "State: {:?} (previous {:?}, {:.2}s)",
            state.current, state.previous, state.time_in_state
        ));
        ui.add(egui::ProgressBar::new(needs.hunger).text(format!("hunger {:.2}", needs.hunger)));
        ui.add(egui::ProgressBar::new(needs.fatigue).text(format!("fatigue {:.2}", needs.fatigue)));
        ui.add(egui::ProgressBar::new(needs.energy).text(format!("energy {:.2}", needs.energy)));
        if let Some(cargo) = cargo {
            ui.label(format!(
                "Cargo: {:?} {:.1}/{:.1}",
                cargo.kind, cargo.amount, cargo.capacity
            ));
        }
        if let Some(memory) = memory {
            ui.label(format!(
                "Memory: {} resources, {} villages",
                memory.resources.len(),
                memory.rest_zones.len()
            ));
        }
        if let Some(decision) = decision {
            ui.label(format!(
                "Decision: {:?} score {:.2} target {:?}",
                decision.action, decision.score, decision.target
            ));
        }
        if let Some(debug) = debug {
            for (action, score) in &debug.last_scores {
                ui.add(egui::ProgressBar::new(*score).text(format!("{action:?} {score:.2}")));
            }
        }
        return;
    }

    if let Ok((_entity, resource, transform)) = resources.get(selected) {
        ui.label(format!("Resource {:?}", resource.kind));
        ui.label(format!(
            "Position: {}",
            format_position(transform.translation)
        ));
        ui.label(format!(
            "Amount: {:.1}/{:.1}",
            resource.amount, resource.max_amount
        ));
        ui.label(format!("Regen: {:.2}/s", resource.regen_rate));
        ui.label(format!("Depleted: {}", resource.is_depleted));
        return;
    }

    if let Ok((_entity, zone, transform, store)) = zones.get(selected) {
        ui.label(format!("Zone {:?}", zone.kind));
        ui.label(format!(
            "Position: {}",
            format_position(transform.translation)
        ));
        ui.label(format!("Radius: {:.1}", zone.radius));
        if let Some(store) = store {
            ui.add(
                egui::ProgressBar::new(store.food / store.capacity.max(1.0))
                    .text(format!("food {:.1}/{:.1}", store.food, store.capacity)),
            );
        }
    }
}

fn format_position(position: Vec3) -> String {
    format!("({:.1}, {:.1}, {:.1})", position.x, position.y, position.z)
}

fn cursor_ground_point(
    windows: &Query<&Window>,
    camera_query: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec3> {
    let window = windows.get_single().ok()?;
    let cursor_position = window.cursor_position()?;
    let (camera, camera_transform) = camera_query.get_single().ok()?;
    let ray = camera
        .viewport_to_world(camera_transform, cursor_position)
        .ok()?;
    let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
    Some(ray.get_point(distance))
}

fn nearest_entity(
    point: Vec3,
    agents: &Query<(Entity, &Transform), With<Agent>>,
    resources: &Query<(Entity, &Transform), With<ResourceNode>>,
    zones: &Query<(Entity, &Transform, &Zone)>,
) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_distance = f32::MAX;

    for (entity, transform) in agents.iter() {
        update_nearest(
            point,
            entity,
            transform.translation,
            1.5,
            &mut nearest,
            &mut nearest_distance,
        );
    }
    for (entity, transform) in resources.iter() {
        update_nearest(
            point,
            entity,
            transform.translation,
            2.0,
            &mut nearest,
            &mut nearest_distance,
        );
    }
    for (entity, transform, zone) in zones.iter() {
        update_nearest(
            point,
            entity,
            transform.translation,
            zone.radius,
            &mut nearest,
            &mut nearest_distance,
        );
    }

    nearest
}

fn update_nearest(
    point: Vec3,
    entity: Entity,
    position: Vec3,
    max_distance: f32,
    nearest: &mut Option<Entity>,
    nearest_distance: &mut f32,
) {
    let distance = point.distance(position);
    if distance <= max_distance && distance < *nearest_distance {
        *nearest = Some(entity);
        *nearest_distance = distance;
    }
}
