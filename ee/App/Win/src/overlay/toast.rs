//! Toast rendering helpers for the overlay.

use super::fade_color;
use eframe::egui;

pub(super) fn render_update_toast(ui: &mut egui::Ui, message: &str, opacity: f32) {
    ui.horizontal_centered(|ui| {
        egui::Frame::none()
            .fill(fade_color(
                egui::Color32::from_rgb(64, 48, 16),
                opacity * 0.95,
            ))
            .stroke(egui::Stroke::new(
                1.0,
                fade_color(egui::Color32::from_rgb(220, 170, 70), opacity),
            ))
            .rounding(6.0)
            .inner_margin(egui::Margin::symmetric(10.0, 4.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(message)
                        .color(fade_color(egui::Color32::from_rgb(255, 235, 190), opacity))
                        .size(12.0),
                );
            });
    });
}
