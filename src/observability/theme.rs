//! Shared egui theme for observability panels.

use bevy_inspector_egui::bevy_egui::{egui, EguiContexts};

/// Apply the observability egui theme before panel systems draw.
pub fn apply_egui_theme_system(mut contexts: EguiContexts) {
    apply_egui_theme(contexts.ctx_mut());
}

/// Apply the custom dark observability theme to the current egui context.
pub fn apply_egui_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = hex_color(0x0d, 0x11, 0x17);
    visuals.window_fill = hex_color(0x0d, 0x11, 0x17);
    visuals.faint_bg_color = hex_color(0x16, 0x1b, 0x22);
    visuals.extreme_bg_color = hex_color(0x0d, 0x11, 0x17);
    visuals.code_bg_color = hex_color(0x16, 0x1b, 0x22);

    visuals.override_text_color = None;
    visuals.warn_fg_color = hex_color(0xf4, 0xa2, 0x61);
    visuals.error_fg_color = hex_color(0xe6, 0x39, 0x46);
    visuals.hyperlink_color = hex_color(0x1f, 0x6f, 0xeb);
    visuals.selection.bg_fill = hex_color(0x23, 0x86, 0x36);
    visuals.selection.stroke = egui::Stroke::new(1.0, hex_color(0xe6, 0xed, 0xf3));
    visuals.window_shadow = egui::epaint::Shadow::NONE;
    visuals.window_stroke = egui::Stroke::new(1.0, hex_color(0x30, 0x36, 0x3d));
    visuals.window_rounding = egui::Rounding::same(6.0);

    visuals.widgets.noninteractive.bg_fill = hex_color(0x0d, 0x11, 0x17);
    visuals.widgets.noninteractive.weak_bg_fill = hex_color(0x16, 0x1b, 0x22);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, hex_color(0xe6, 0xed, 0xf3));
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, hex_color(0x30, 0x36, 0x3d));
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);

    visuals.widgets.inactive.bg_fill = hex_color(0x23, 0x86, 0x36);
    visuals.widgets.inactive.weak_bg_fill = hex_color(0x16, 0x1b, 0x22);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, hex_color(0xe6, 0xed, 0xf3));
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, hex_color(0x30, 0x36, 0x3d));
    visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);

    visuals.widgets.hovered.bg_fill = hex_color(0x2e, 0xa0, 0x43);
    visuals.widgets.hovered.weak_bg_fill = hex_color(0x21, 0x26, 0x2d);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, hex_color(0xe6, 0xed, 0xf3));
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, hex_color(0x2e, 0xa0, 0x43));
    visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);

    visuals.widgets.active.bg_fill = hex_color(0x30, 0x36, 0x3d);
    visuals.widgets.active.weak_bg_fill = hex_color(0x1f, 0x6f, 0xeb);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, hex_color(0xe6, 0xed, 0xf3));
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, hex_color(0x2e, 0xa0, 0x43));
    visuals.widgets.active.rounding = egui::Rounding::same(4.0);

    visuals.widgets.open.bg_fill = hex_color(0x23, 0x86, 0x36);
    visuals.widgets.open.weak_bg_fill = hex_color(0x16, 0x1b, 0x22);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, hex_color(0xe6, 0xed, 0xf3));
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, hex_color(0x2e, 0xa0, 0x43));
    visuals.widgets.open.rounding = egui::Rounding::same(4.0);

    visuals.slider_trailing_fill = true;

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.slider_width = 140.0;
    ctx.set_style(style);
}

fn hex_color(red: u8, green: u8, blue: u8) -> egui::Color32 {
    egui::Color32::from_rgb(red, green, blue)
}
