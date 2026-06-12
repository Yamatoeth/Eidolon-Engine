//! Live entity/component browser with egui.

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

use crate::ai::actions::ActionKind;
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
        ui.separator();
        ui.heading("Needs");
        draw_need_bar_row(ui, "Hunger", needs.hunger, NeedBarKind::Hunger);
        draw_need_bar_row(ui, "Fatigue", needs.fatigue, NeedBarKind::Fatigue);
        draw_need_bar_row(ui, "Energy", needs.energy, NeedBarKind::Energy);
        ui.separator();
        ui.heading("AI Decision");
        if let Some(decision) = decision {
            ui.horizontal(|ui| {
                draw_action_badge(ui, decision.action);
                ui.label(format!("score: {:.2}", decision.score));
            });
            if let Some(debug) = debug {
                let mut last_scores = debug.last_scores.clone();
                last_scores.sort_by(|a, b| b.1.total_cmp(&a.1));
                ui.horizontal_wrapped(|ui| {
                    for (index, (action, score)) in last_scores.iter().take(6).enumerate() {
                        if index > 0 {
                            ui.label(" ");
                        }
                        ui.small(format!("{}: {:.2}", action_name(*action), score));
                    }
                });
            }
        } else {
            ui.label("No decision output available");
        }
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

#[derive(Clone, Copy, Debug)]
enum NeedBarKind {
    Hunger,
    Fatigue,
    Energy,
}

fn draw_need_bar_row(ui: &mut egui::Ui, label: &str, value: f32, kind: NeedBarKind) {
    const LABEL_WIDTH: f32 = 80.0;
    const BAR_WIDTH: f32 = 200.0;
    const BAR_HEIGHT: f32 = 14.0;
    const VALUE_WIDTH: f32 = 44.0;
    const MARKERS: [f32; 2] = [0.6, 0.85];

    let visuals = ui.visuals();
    let text_color = visuals.widgets.noninteractive.fg_stroke.color;
    let track_color = visuals.faint_bg_color;
    let border_color = visuals.widgets.noninteractive.bg_stroke.color;
    let value = value.clamp(0.0, 1.0);
    let fill_color = need_bar_color(kind, value);
    let fill_ratio = match kind {
        NeedBarKind::Energy => value,
        NeedBarKind::Hunger | NeedBarKind::Fatigue => value,
    };

    ui.horizontal(|ui| {
        ui.add_sized(
            [LABEL_WIDTH, BAR_HEIGHT],
            egui::Label::new(egui::RichText::new(label).color(text_color)),
        );

        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(BAR_WIDTH, BAR_HEIGHT), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let rounding = egui::Rounding::same(3.0);
        painter.rect_filled(rect, rounding, track_color);

        let fill_width = (rect.width() * fill_ratio).clamp(0.0, rect.width());
        if fill_width > 0.0 {
            let fill_rect = egui::Rect::from_min_max(
                rect.min,
                egui::pos2(rect.min.x + fill_width, rect.max.y),
            );
            painter.rect_filled(fill_rect, rounding, fill_color);
        }

        for marker in MARKERS {
            let x = rect.left() + rect.width() * marker;
            painter.line_segment(
                [egui::pos2(x, rect.top() + 1.0), egui::pos2(x, rect.bottom() - 1.0)],
                egui::Stroke::new(1.0, egui::Color32::from_white_alpha(128)),
            );
        }
        painter.rect_stroke(
            rect,
            rounding,
            egui::Stroke::new(1.0, border_color),
        );

        ui.allocate_ui_with_layout(
            egui::vec2(VALUE_WIDTH, BAR_HEIGHT),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(format!("{:.0}%", value * 100.0));
            },
        );
    });
}

fn draw_action_badge(ui: &mut egui::Ui, action: ActionKind) {
    let visuals = ui.visuals();
    let fill = action_badge_color(action, visuals);
    let stroke = visuals.widgets.noninteractive.bg_stroke.color;
    let text_color = visuals.widgets.noninteractive.fg_stroke.color;
    let label = action_name(action);
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(11.0),
        text_color,
    );
    let size = galley.size() + egui::vec2(12.0, 6.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, egui::Rounding::same(3.0), fill);
    painter.rect_stroke(
        rect,
        egui::Rounding::same(3.0),
        egui::Stroke::new(1.0, stroke),
    );
    painter.galley(
        rect.center() - galley.size() * 0.5,
        galley,
        text_color,
    );
}

fn need_bar_color(kind: NeedBarKind, value: f32) -> egui::Color32 {
    match kind {
        NeedBarKind::Hunger => {
            let green = egui::Color32::from_rgb(0x23, 0x86, 0x36);
            let orange = egui::Color32::from_rgb(0xf4, 0xa2, 0x61);
            let red = egui::Color32::from_rgb(0xe6, 0x39, 0x46);
            if value <= 0.6 {
                lerp_color(green, orange, value / 0.6)
            } else {
                lerp_color(orange, red, ((value - 0.6) / 0.25).clamp(0.0, 1.0))
            }
        }
        NeedBarKind::Fatigue => {
            let green = egui::Color32::from_rgb(0x23, 0x86, 0x36);
            let purple = egui::Color32::from_rgb(0x99, 0x5d, 0xe5);
            lerp_color(green, purple, value)
        }
        NeedBarKind::Energy => {
            let orange = egui::Color32::from_rgb(0xf4, 0xa2, 0x61);
            let green = egui::Color32::from_rgb(0x23, 0x86, 0x36);
            lerp_color(orange, green, value)
        }
    }
}

fn action_badge_color(action: ActionKind, visuals: &egui::Visuals) -> egui::Color32 {
    match action {
        ActionKind::Eat => visuals.warn_fg_color,
        ActionKind::Rest => egui::Color32::from_rgb(0x99, 0x5d, 0xe5),
        ActionKind::Explore => egui::Color32::from_rgb(0xf0, 0xc7, 0x32),
        ActionKind::Idle => visuals.faint_bg_color,
        ActionKind::MoveTo => visuals.hyperlink_color,
        ActionKind::Collect => visuals.selection.bg_fill,
        ActionKind::Deliver => egui::Color32::from_rgb(0x1f, 0x6f, 0xeb),
    }
}

fn action_name(action: ActionKind) -> &'static str {
    match action {
        ActionKind::Idle => "Idle",
        ActionKind::MoveTo => "MoveTo",
        ActionKind::Eat => "Eat",
        ActionKind::Rest => "Rest",
        ActionKind::Deliver => "Deliver",
        ActionKind::Explore => "Explore",
        ActionKind::Collect => "Collect",
    }
}

fn lerp_color(from: egui::Color32, to: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let from = egui::Rgba::from(from);
    let to = egui::Rgba::from(to);
    egui::Color32::from(egui::Rgba::from_rgba_unmultiplied(
        from.r() + (to.r() - from.r()) * t,
        from.g() + (to.g() - from.g()) * t,
        from.b() + (to.b() - from.b()) * t,
        from.a() + (to.a() - from.a()) * t,
    ))
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
