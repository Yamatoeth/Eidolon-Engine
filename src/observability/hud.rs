//! Bottom-screen metrics HUD.

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

use crate::engine::SimulationTime;
use crate::scenarios::loader::ActiveScenarioLabel;
use crate::simulation::SimulationMetrics;

const PANEL_ALPHA: u8 = 217;
const PANEL_PADDING_X: f32 = 24.0;
const PANEL_PADDING_Y: f32 = 12.0;
const BAR_WIDTH: f32 = 180.0;
const BAR_HEIGHT: f32 = 10.0;
const BAR_ROUNDING: f32 = 3.0;
const LEGEND_ALPHA: u8 = 230;
const LEGEND_OFFSET: f32 = 24.0;
const LEGEND_ICON_SIZE: f32 = 12.0;
const LEGEND_FONT_SIZE: f32 = 11.0;
const SCENARIO_FADE_IN_SECS: f32 = 0.5;
const SCENARIO_FULL_DISPLAY_SECS: f32 = 3.0;
const SCENARIO_DIM_ALPHA: f32 = 0.6;

/// Fade state for the active scenario indicator.
#[derive(Resource, Debug, Clone, Default)]
pub struct ScenarioIndicatorState {
    previous_name: String,
    elapsed_since_change: f32,
}

/// Render a read-only real-time metrics HUD along the bottom of the screen.
pub fn hud_metrics_system(
    mut contexts: EguiContexts,
    metrics: Res<SimulationMetrics>,
    sim_time: Res<SimulationTime>,
) {
    let ctx = contexts.ctx_mut();
    let screen_width = ctx.screen_rect().width();

    egui::Area::new(egui::Id::new("metrics_hud"))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::Vec2::ZERO)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgba_premultiplied(
                    0x0d,
                    0x11,
                    0x17,
                    PANEL_ALPHA,
                ))
                .inner_margin(egui::Margin::symmetric(PANEL_PADDING_X, PANEL_PADDING_Y))
                .show(ui, |ui| {
                    ui.set_width(screen_width);
                    ui.horizontal_centered(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Agents: {}", metrics.agent_count))
                                .color(agent_count_color(metrics.agent_count))
                                .strong(),
                        );
                        ui.separator();
                        metric_bar(ui, "Hunger", metrics.avg_hunger, GREEN, RED);
                        ui.separator();
                        metric_bar(ui, "Fatigue", metrics.avg_fatigue, GREEN, PURPLE);
                        ui.separator();
                        metric_bar(ui, "Energy", metrics.avg_energy, GREEN, ORANGE);
                        ui.separator();
                        ui.label(format!("t={:.1}s", sim_time.elapsed));
                        ui.label(format!("tick={:05}", sim_time.tick));
                    });
                });
        });
}

/// Render the always-visible world legend in the top-right corner.
pub fn legend_ui_system(mut contexts: EguiContexts) {
    egui::Area::new(egui::Id::new("world_legend"))
        .anchor(
            egui::Align2::RIGHT_TOP,
            egui::Vec2::new(-LEGEND_OFFSET, LEGEND_OFFSET),
        )
        .interactable(false)
        .show(contexts.ctx_mut(), |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgba_premultiplied(
                    0x0d,
                    0x11,
                    0x17,
                    LEGEND_ALPHA,
                ))
                .stroke(egui::Stroke::new(1.0, BORDER))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                .show(ui, |ui| {
                    section_title(ui, "ZONES");
                    legend_row(
                        ui,
                        IconKind::Circle,
                        GREEN,
                        "Resource Zone — harvestable food",
                    );
                    legend_row(
                        ui,
                        IconKind::Circle,
                        BLUE,
                        "Rest Zone — fatigue recovery ×1.5",
                    );
                    legend_row(ui, IconKind::Circle, GREY, "Neutral Zone — no modifier");

                    ui.add_space(8.0);
                    section_title(ui, "AGENT NEEDS");
                    legend_row(ui, IconKind::Rect, TEAL, "Healthy");
                    legend_row(ui, IconKind::Rect, ORANGE, "Hungry (>60%)");
                    legend_row(ui, IconKind::Rect, RED, "Starving (>85%)");
                    legend_row(ui, IconKind::Rect, PURPLE, "Fatigued (>70%)");
                });
        });
}

/// Render the active scenario name and description at top-center.
pub fn scenario_indicator_system(
    mut contexts: EguiContexts,
    time: Res<Time>,
    label: Option<Res<ActiveScenarioLabel>>,
    mut state: ResMut<ScenarioIndicatorState>,
) {
    let Some(label) = label else {
        return;
    };
    if label.name.is_empty() {
        return;
    }

    if state.previous_name != label.name {
        state.previous_name = label.name.clone();
        state.elapsed_since_change = 0.0;
    } else {
        state.elapsed_since_change += time.delta_secs();
    }

    let alpha = scenario_indicator_alpha(state.elapsed_since_change);
    let primary = alpha_color(TEXT_PRIMARY, alpha);
    let secondary = alpha_color(SECONDARY_TEXT, alpha);
    let border = alpha_color(BORDER, alpha);

    egui::Area::new(egui::Id::new("scenario_indicator"))
        .anchor(egui::Align2::CENTER_TOP, egui::Vec2::new(0.0, 18.0))
        .interactable(false)
        .show(contexts.ctx_mut(), |ui| {
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(12.0, 4.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(label.name.to_uppercase())
                                .size(13.0)
                                .color(primary)
                                .strong(),
                        );
                        if !label.description.is_empty() {
                            ui.label(
                                egui::RichText::new(&label.description)
                                    .size(11.0)
                                    .color(secondary),
                            );
                        }
                        let width = ui.available_width().clamp(160.0, 360.0);
                        let (rect, _) = ui
                            .allocate_exact_size(egui::Vec2::new(width, 1.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, border);
                    });
                });
        });
}

const GREEN: egui::Color32 = egui::Color32::from_rgb(0x23, 0x86, 0x36);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(0xf4, 0xd0, 0x3f);
const RED: egui::Color32 = egui::Color32::from_rgb(0xe6, 0x39, 0x46);
const PURPLE: egui::Color32 = egui::Color32::from_rgb(0x9b, 0x5d, 0xe5);
const ORANGE: egui::Color32 = egui::Color32::from_rgb(0xf4, 0xa2, 0x61);
const BLUE: egui::Color32 = egui::Color32::from_rgb(0x58, 0xa6, 0xff);
const GREY: egui::Color32 = egui::Color32::from_rgb(0x8b, 0x94, 0x9e);
const TEAL: egui::Color32 = egui::Color32::from_rgb(0x00, 0xb4, 0xd8);
const BORDER: egui::Color32 = egui::Color32::from_rgb(0x30, 0x36, 0x3d);
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(0xe6, 0xed, 0xf3);
const SECONDARY_TEXT: egui::Color32 = egui::Color32::from_rgb(0x8b, 0x94, 0x9e);
const BAR_BG: egui::Color32 = egui::Color32::from_rgb(0x16, 0x1b, 0x22);

#[derive(Clone, Copy)]
enum IconKind {
    Circle,
    Rect,
}

fn agent_count_color(count: u32) -> egui::Color32 {
    match count {
        0..=4 => RED,
        5..=10 => YELLOW,
        _ => GREEN,
    }
}

fn metric_bar(ui: &mut egui::Ui, label: &str, value: f32, from: egui::Color32, to: egui::Color32) {
    let value = value.clamp(0.0, 1.0);
    ui.label(label);
    let (rect, _) =
        ui.allocate_exact_size(egui::Vec2::new(BAR_WIDTH, BAR_HEIGHT), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::Rounding::same(BAR_ROUNDING), BAR_BG);

    let fill_width = rect.width() * value;
    if fill_width > 0.0 {
        let fill_rect =
            egui::Rect::from_min_size(rect.min, egui::Vec2::new(fill_width, rect.height()));
        ui.painter().rect_filled(
            fill_rect,
            egui::Rounding::same(BAR_ROUNDING),
            lerp_color(from, to, value),
        );
    }
    ui.label(format!("{:.0}%", value * 100.0));
}

fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(LEGEND_FONT_SIZE)
            .color(SECONDARY_TEXT)
            .strong(),
    );
}

fn legend_row(ui: &mut egui::Ui, icon: IconKind, color: egui::Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) =
            ui.allocate_exact_size(egui::Vec2::splat(LEGEND_ICON_SIZE), egui::Sense::hover());
        match icon {
            IconKind::Circle => {
                ui.painter()
                    .circle_filled(rect.center(), LEGEND_ICON_SIZE * 0.5, color);
            },
            IconKind::Rect => {
                ui.painter()
                    .rect_filled(rect, egui::Rounding::same(2.0), color);
            },
        }
        ui.label(egui::RichText::new(label).size(LEGEND_FONT_SIZE));
    });
}

fn scenario_indicator_alpha(elapsed: f32) -> f32 {
    if elapsed < SCENARIO_FADE_IN_SECS {
        (elapsed / SCENARIO_FADE_IN_SECS).clamp(0.0, 1.0)
    } else if elapsed < SCENARIO_FADE_IN_SECS + SCENARIO_FULL_DISPLAY_SECS {
        1.0
    } else {
        SCENARIO_DIM_ALPHA
    }
}

fn alpha_color(color: egui::Color32, alpha: f32) -> egui::Color32 {
    let [red, green, blue, original_alpha] = color.to_array();
    egui::Color32::from_rgba_premultiplied(
        red,
        green,
        blue,
        ((original_alpha as f32) * alpha.clamp(0.0, 1.0)).round() as u8,
    )
}

fn lerp_color(from: egui::Color32, to: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let [fr, fg, fb, fa] = from.to_array();
    let [tr, tg, tb, ta] = to.to_array();
    egui::Color32::from_rgba_premultiplied(
        lerp_u8(fr, tr, t),
        lerp_u8(fg, tg, t),
        lerp_u8(fb, tb, t),
        lerp_u8(fa, ta, t),
    )
}

fn lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * t).round() as u8
}
