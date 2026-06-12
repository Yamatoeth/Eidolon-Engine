//! Bottom-screen metrics HUD.

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

use crate::engine::SimulationTime;
use crate::simulation::SimulationMetrics;

const PANEL_ALPHA: u8 = 217;
const PANEL_PADDING_X: f32 = 24.0;
const PANEL_PADDING_Y: f32 = 12.0;
const BAR_WIDTH: f32 = 180.0;
const BAR_HEIGHT: f32 = 10.0;
const BAR_ROUNDING: f32 = 3.0;

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

const GREEN: egui::Color32 = egui::Color32::from_rgb(0x23, 0x86, 0x36);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(0xf4, 0xd0, 0x3f);
const RED: egui::Color32 = egui::Color32::from_rgb(0xe6, 0x39, 0x46);
const PURPLE: egui::Color32 = egui::Color32::from_rgb(0x9b, 0x5d, 0xe5);
const ORANGE: egui::Color32 = egui::Color32::from_rgb(0xf4, 0xa2, 0x61);
const BAR_BG: egui::Color32 = egui::Color32::from_rgb(0x16, 0x1b, 0x22);

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
