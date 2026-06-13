//! Overlay result-panel rendering helpers.

use super::{
    cn_row_activation_index, fade_color, should_focus_on_pointer_hover, FLYOUT_MAX_WINDOW_HEIGHT,
};
use eframe::egui;

/// Draw the dark results panel at an animated (clipped) height so it grows and
/// shrinks gradually as result rows stream in, instead of jumping each time a
/// Card Preview is added. `display_height` is the current (eased) visible
/// height; the content is laid out at its natural height but clipped to the
/// visible panel rect, so rows are revealed as the panel grows. Returns the
/// natural content height (including the panel's inner margins) so the caller
/// can ease `display_height` toward it on the next frame. This is pure egui
/// painting — the OS window is never resized — so it does not affect keyboard
/// input.
pub(crate) fn draw_growing_results_panel(
    ui: &mut egui::Ui,
    opacity: f32,
    display_height: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> f32 {
    const ROUNDING: f32 = 8.0;
    const MARGIN: f32 = 14.0;

    let width = ui.available_width();
    let (bg_rect, _) = ui.allocate_exact_size(
        egui::vec2(width, display_height.max(0.0)),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(
        bg_rect,
        ROUNDING,
        fade_color(egui::Color32::from_black_alpha(220), opacity),
    );

    let content_rect = egui::Rect::from_min_size(
        bg_rect.min + egui::vec2(MARGIN, MARGIN),
        // egui's layout requires a finite max_rect, so the child Ui is given a
        // generous finite height bound rather than INFINITY. A top-down layout
        // still reports the true natural height via `min_rect()` even when the
        // content exceeds this bound, so measurement stays correct.
        egui::vec2(
            (width - 2.0 * MARGIN).max(0.0),
            FLYOUT_MAX_WINDOW_HEIGHT * 4.0,
        ),
    );
    let mut child = ui.child_ui(content_rect, egui::Layout::top_down(egui::Align::Min), None);
    child.set_clip_rect(bg_rect.intersect(ui.clip_rect()));
    add_contents(&mut child);
    child.min_rect().height() + 2.0 * MARGIN
}

/// Outcome of interacting with a Chinese preview row.
pub(super) enum CnRowAction {
    /// A moving pointer hovered the row body (select the whole row).
    HoverRow,
    /// A moving pointer hovered the `n`-th English button.
    HoverButton(usize),
    /// An English word was activated (click, or Enter/Space on the focused button).
    Activate(String),
}

/// Render one Chinese preview row: the Chinese term on the left, up to three
/// frequency-ordered English word buttons on the right. Highlights the row when
/// `row_selected`, and the `active_button`-th button when set. Returns the
/// highest-priority interaction this frame (activation > button hover > row hover).
pub(super) fn render_cn_preview_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    term: &str,
    english: &[String],
    opacity: f32,
    row_selected: bool,
    active_button: Option<usize>,
) -> Option<CnRowAction> {
    let mut action: Option<CnRowAction> = None;

    let row_fill = if row_selected {
        fade_color(egui::Color32::from_rgb(0, 80, 160), opacity * 0.4)
    } else {
        egui::Color32::TRANSPARENT
    };

    let frame_resp = egui::Frame::none()
        .fill(row_fill)
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(term)
                        .strong()
                        .color(fade_color(egui::Color32::WHITE, opacity))
                        .size(14.0),
                );

                // Buttons laid out from the right; add highest index first so the
                // most-frequent word (index 0) ends up left-most in the cluster.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for idx in (0..english.len()).rev() {
                        let focused = active_button == Some(idx);
                        let stroke = if focused {
                            egui::Stroke::new(
                                2.0,
                                fade_color(egui::Color32::from_rgb(0, 120, 215), opacity),
                            )
                        } else {
                            egui::Stroke::new(
                                1.0,
                                fade_color(egui::Color32::from_gray(90), opacity),
                            )
                        };
                        let fill = if focused {
                            fade_color(egui::Color32::from_rgb(0, 80, 160), opacity * 0.5)
                        } else {
                            fade_color(egui::Color32::from_rgb(30, 30, 30), opacity)
                        };
                        let btn = egui::Frame::none()
                            .fill(fill)
                            .stroke(stroke)
                            .rounding(5.0)
                            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(&english[idx])
                                        .color(fade_color(egui::Color32::WHITE, opacity))
                                        .size(13.0),
                                );
                            });
                        let hit = ui.allocate_rect(btn.response.rect, egui::Sense::click());
                        if hit.clicked() {
                            action = Some(CnRowAction::Activate(english[idx].clone()));
                        } else if action.is_none()
                            && should_focus_on_pointer_hover(
                                hit.hovered(),
                                ui.input(|i| i.pointer.is_moving()),
                            )
                        {
                            action = Some(CnRowAction::HoverButton(idx));
                        }
                    }
                });
            });
        });

    // Enter / Space activates the focused English word. With a button focused
    // its word is used; when the whole row is selected and there is a single
    // candidate, that lone word is activated directly.
    if let Some(idx) = cn_row_activation_index(active_button, row_selected, english.len()) {
        if ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) {
            action = Some(CnRowAction::Activate(english[idx].clone()));
        }
    }

    // Whole-row hover is the lowest-priority interaction.
    if action.is_none() {
        let row_hit = ui.interact(
            frame_resp.response.rect,
            egui::Id::new(("cn_row", term)),
            egui::Sense::hover(),
        );
        if should_focus_on_pointer_hover(row_hit.hovered(), ui.input(|i| i.pointer.is_moving())) {
            action = Some(CnRowAction::HoverRow);
        }
    }

    action
}

/// Render the always-present "Search on Bing" entry (the bottom row of the
/// results pane). Returns true when a moving pointer hovers it, so the caller
/// can move keyboard focus onto it. Opening the Bing URL (mouse click, or
/// Enter/Space while the entry is focused) is handled internally.
pub(super) fn render_bing_entry(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    query: &str,
    opacity: f32,
    is_focused: bool,
) -> bool {
    let card_stroke = if is_focused {
        egui::Stroke::new(
            2.0,
            fade_color(egui::Color32::from_rgb(0, 120, 215), opacity),
        )
    } else {
        egui::Stroke::new(1.0, fade_color(egui::Color32::from_gray(80), opacity))
    };

    let response = egui::Frame::none()
        .fill(fade_color(egui::Color32::from_rgb(20, 20, 20), opacity))
        .stroke(card_stroke)
        .rounding(6.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("ðŸ” Search on Bing: ")
                        .color(fade_color(egui::Color32::LIGHT_BLUE, opacity))
                        .strong()
                        .size(13.0),
                );
                ui.label(
                    egui::RichText::new(query)
                        .color(fade_color(egui::Color32::WHITE, opacity))
                        .size(13.0),
                );
            });
        });

    let interaction = ui.allocate_rect(response.response.rect, egui::Sense::click());
    let activated = interaction.clicked()
        || (is_focused
            && ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)));
    if activated {
        let mut encoded = String::new();
        for c in query.chars() {
            if c.is_ascii_alphanumeric() {
                encoded.push(c);
            } else if c == ' ' {
                encoded.push_str("%20");
            } else {
                for byte in c.to_string().bytes() {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        ctx.open_url(egui::OpenUrl::new_tab(format!(
            "https://www.bing.com/search?q={}",
            encoded
        )));
    }

    should_focus_on_pointer_hover(interaction.hovered(), ui.input(|i| i.pointer.is_moving()))
}
