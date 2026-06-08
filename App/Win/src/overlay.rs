//! The egui search-overlay app: window lifecycle, layout, query, rendering.

use crate::dict::{load_highest_version_word_list, scan_for_highest_db_version};
use crate::focus::{evaluate_focus_hide, AnimationState, FocusHideDecision, WAKE_FOCUS_GRACE};
use crate::logging::log_message;
use crate::signals::{EGUI_CTX, EXIT_REQUESTED, FLYOUT_HWND, FLYOUT_WAKE_READY, VISIBLE_REQUESTED};
use crate::win32::{cursor_monitor_rect, find_flyout_window};
use ee_core::{Hub, Record, RecordModel, Storage};
use ee_utils::Signal;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub(crate) const FLYOUT_WINDOW_WIDTH: f32 = 550.0;
pub(crate) const FLYOUT_MAX_WINDOW_HEIGHT: f32 = 360.0;
const FLYOUT_INPUT_PANEL_HEIGHT: f32 = 56.0;
const FLYOUT_BOTTOM_MARGIN: f32 = 16.0;

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let custom_fonts = ["noto_sans", "noto_sans_sc"];

    fonts.font_data.insert(
        custom_fonts[0].to_owned(),
        egui::FontData::from_static(include_bytes!("../../Assets/NotoSans-Variable.ttf")),
    );
    fonts.font_data.insert(
        custom_fonts[1].to_owned(),
        egui::FontData::from_static(include_bytes!("../../Assets/NotoSansSC-Variable.ttf")),
    );

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        if let Some(family_fonts) = fonts.families.get_mut(&family) {
            for name in custom_fonts.into_iter().rev() {
                family_fonts.insert(0, name.to_owned());
            }
        }
    }

    ctx.set_fonts(fonts);
}

pub(crate) struct SearchOverlayApp {
    input: String,
    last_input: String,
    hub: Hub,
    current_query: Option<(u64, ee_utils::DynamicResult<Vec<Record>>)>,
    query_generation: u64,
    records: Vec<Record>,
    wake_at: Option<std::time::Instant>,
    was_focused: bool,
    target_monitor: Option<(f32, f32, f32, f32)>,

    animation_state: AnimationState,
    opacity: f32,
    last_frame: std::time::Instant,
    last_viewport_size: Option<egui::Vec2>,
    last_viewport_pos: Option<egui::Pos2>,
    focus_index: usize, // 0 = Input box, 1 = Exact Card, 2+ = Card Previews
    ime_composing: bool,
    word_list: Vec<String>,
}

impl SearchOverlayApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Save egui context to the global handle so the hook thread can trigger redraws
        *EGUI_CTX.lock().unwrap() = Some(cc.egui_ctx.clone());

        // Build the backend Hub and dynamically scan/load only the highest version dictionary database
        let mut hub = Hub::new();
        if let Some(highest_db) = scan_for_highest_db_version() {
            if let Ok(storage) = Storage::new(&highest_db) {
                hub.add_provider(Arc::new(storage));
            }
        }

        // Load the corresponding word list in memory for instantaneous fuzzy/prefix searches
        let word_list = load_highest_version_word_list();

        configure_fonts(&cc.egui_ctx);

        // Configure standard visuals to use 100% transparent fills for window/panel background
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::TRANSPARENT;
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        cc.egui_ctx.set_visuals(visuals);

        Self {
            input: String::new(),
            last_input: String::new(),
            hub,
            current_query: None,
            query_generation: 0,
            records: Vec::new(),
            wake_at: None,
            was_focused: false,
            target_monitor: None,
            animation_state: AnimationState::Hidden,
            opacity: 0.0,
            last_frame: std::time::Instant::now(),
            last_viewport_size: None,
            last_viewport_pos: None,
            focus_index: 0,
            ime_composing: false,
            word_list,
        }
    }

    fn update_ime_composition_state(&mut self, ctx: &egui::Context) -> bool {
        let mut saw_ime_event = false;
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Ime(ime) = event {
                    saw_ime_event = true;
                    match ime {
                        egui::ImeEvent::Preedit(text) => {
                            self.ime_composing = !text.is_empty();
                        }
                        egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => {
                            self.ime_composing = false;
                        }
                        egui::ImeEvent::Enabled => {}
                    }
                }
            }
        });
        saw_ime_event
    }

    fn cancel_current_query(&mut self) {
        if let Some((_, query_handle)) = &self.current_query {
            query_handle.cancel();
        }
        self.current_query = None;
    }

    fn next_query_generation(&mut self) -> u64 {
        self.query_generation = self.query_generation.wrapping_add(1);
        self.query_generation
    }

    fn apply_viewport_layout(&mut self, ctx: &egui::Context) {
        let scale = ctx.pixels_per_point();
        let (phys_left, phys_top, physical_w, physical_h) =
            self.target_monitor.unwrap_or_else(cursor_monitor_rect);
        // Work in the window's logical points. Dividing the monitor's physical
        // virtual-desktop rect by the window scale, then letting winit multiply
        // the OuterPosition back by the same scale, lands the flyout on the right
        // monitor regardless of per-monitor DPI.
        let mon_left = phys_left / scale;
        let mon_top = phys_top / scale;
        let screen_w = physical_w / scale;
        let screen_h = physical_h / scale;
        let (next_size, next_pos) = centered_on_monitor(mon_left, mon_top, screen_w, screen_h);

        if self
            .last_viewport_size
            .is_none_or(|size| (size - next_size).length_sq() > 0.25)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(next_size));
            self.last_viewport_size = Some(next_size);
        }

        if self.last_viewport_pos.is_none_or(|pos| {
            let dx = pos.x - next_pos.x;
            let dy = pos.y - next_pos.y;
            dx * dx + dy * dy > 0.25
        }) {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(next_pos));
            self.last_viewport_pos = Some(next_pos);
        }
    }
}

impl eframe::App for SearchOverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Compute precise delta time (dt) for high-performance fluid framerate independence
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        let ime_event_this_frame = self.update_ime_composition_state(ctx);
        let ime_input_active = self.ime_composing || ime_event_this_frame;
        if self.ime_composing {
            self.cancel_current_query();
            self.next_query_generation();
            self.records.clear();
            self.focus_index = 0;
        }

        // Instant search on typing: if the input has changed, trigger a fresh search immediately
        if !ime_input_active {
            let trimmed_input = self.input.trim().to_lowercase();
            if trimmed_input != self.last_input {
                self.last_input = trimmed_input.clone();
                let query_generation = self.next_query_generation();

                // Immediately cancel the previous query thread to release resources
                self.cancel_current_query();
                self.records.clear();
                self.focus_index = 0; // Reset focus to input box on new search

                if !trimmed_input.is_empty() {
                    log_message(&format!(
                        "[Query] Input changed to: '{}'. Finding suggestions...",
                        trimmed_input
                    ));
                    // Get the exact word and up to 5 best fuzzy/prefix candidates
                    let mut query_keys = vec![trimmed_input.clone()];
                    let candidates = ee_core::rank_candidates(
                        &trimmed_input,
                        &self
                            .word_list
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<&str>>(),
                        5,
                    );
                    log_message(&format!(
                        "[Query] Generated {} candidate keys: {:?}",
                        candidates.len(),
                        candidates
                    ));
                    for c in candidates {
                        if c != trimmed_input {
                            query_keys.push(c);
                        }
                    }

                    log_message(&format!(
                        "[Query] Dispatching multi-key query to Hub: {:?}",
                        query_keys
                    ));
                    let handle = self.hub.query(&query_keys);
                    self.current_query = Some((query_generation, handle));
                }
            }
        }

        // Handle ESC key to hide/close the flyout text box
        if !ime_input_active && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            log_message("[State] ESC pressed → FadingOut.");
            self.animation_state = AnimationState::FadingOut;
        }

        // Handle global wake-up requests from the hotkey/mouse hook only while fully hidden.
        if VISIBLE_REQUESTED.swap(false, Ordering::SeqCst) {
            if self.animation_state == AnimationState::Hidden {
                FLYOUT_WAKE_READY.store(false, Ordering::SeqCst);
                self.animation_state = AnimationState::FadingIn;
                self.opacity = 0.0;
                self.wake_at = Some(std::time::Instant::now());
                self.was_focused = false;
                // Show on whichever monitor the mouse cursor is on, decided once
                // at wake time so the flyout does not follow the cursor afterwards.
                let monitor = cursor_monitor_rect();
                self.target_monitor = Some(monitor);
                self.last_viewport_pos = None;
                log_message(&format!(
                    "[State] wake: Hidden → FadingIn on monitor at ({}, {}) {}x{} \
                     (focus grace timer started).",
                    monitor.0, monitor.1, monitor.2, monitor.3
                ));
                #[cfg(target_os = "windows")]
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        SetForegroundWindow, ShowWindow,
                    };
                    let mut hwnd = FLYOUT_HWND.load(Ordering::SeqCst);
                    if hwnd == 0 {
                        hwnd = find_flyout_window();
                        if hwnd != 0 {
                            FLYOUT_HWND.store(hwnd, Ordering::SeqCst);
                        }
                    }
                    if hwnd != 0 {
                        ShowWindow(hwnd, 5); // SW_SHOW = 5
                        SetForegroundWindow(hwnd);
                    }
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else {
                log_message("[Wake] Ignored wake request because flyout is not fully hidden.");
            }
        }

        if self.animation_state != AnimationState::Hidden {
            FLYOUT_WAKE_READY.store(false, Ordering::SeqCst);
        }

        // Handle global exit requests from Tray Icon menu
        if EXIT_REQUESTED.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Track focus acquisition and decide whether to auto-hide on focus loss.
        // Behaviour: once the flyout is "ready" (Visible), losing foreground focus
        // hides it. A short post-wake grace lets the freshly shown window actually
        // acquire focus before we judge it "unfocused"; once focus has ever been
        // acquired, any later focus loss hides immediately regardless of the grace.
        let viewport_focused = ctx.input(|i| i.viewport().focused);
        if viewport_focused == Some(true) {
            self.was_focused = true;
        }
        let grace_expired = self
            .wake_at
            .map(|started| started.elapsed() >= WAKE_FOCUS_GRACE)
            .unwrap_or(true);
        match evaluate_focus_hide(
            self.animation_state,
            viewport_focused,
            self.ime_composing,
            self.was_focused,
            grace_expired,
        ) {
            FocusHideDecision::Hide => {
                log_message(&format!(
                    "[Focus] visible & unfocused (focused={:?}, was_focused={}, grace_expired={}) \
                     → FadingOut.",
                    viewport_focused, self.was_focused, grace_expired
                ));
                self.animation_state = AnimationState::FadingOut;
            }
            // Not focused yet but still inside the grace window: keep repainting so
            // we promptly notice focus arriving (or the grace expiring) even while
            // the user provides no further input.
            FocusHideDecision::WaitForFocus => {
                ctx.request_repaint();
            }
            FocusHideDecision::Keep => {}
        }

        // Animation State Machine updates
        match self.animation_state {
            AnimationState::Hidden => {
                FLYOUT_WAKE_READY.store(true, Ordering::SeqCst);
                return;
            }
            AnimationState::FadingIn => {
                self.opacity = (self.opacity + dt * 6.0).min(1.0);

                if self.opacity >= 1.0 {
                    self.animation_state = AnimationState::Visible;
                    log_message("[State] FadingIn → Visible (ready).");
                }
                ctx.request_repaint();
            }
            AnimationState::Visible => {
                self.opacity = 1.0;
                // egui is reactive and would otherwise stop repainting while idle.
                // Keep a low-frequency repaint so a click into another window
                // reliably triggers the focus-loss auto-hide without further input.
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            AnimationState::FadingOut => {
                self.opacity = (self.opacity - dt * 6.0).max(0.0);

                if self.opacity <= 0.0 {
                    self.animation_state = AnimationState::Hidden;
                    self.wake_at = None;
                    self.was_focused = false;
                    log_message("[State] FadingOut → Hidden (flyout fully hidden).");
                    self.input.clear();
                    self.records.clear();
                    #[cfg(target_os = "windows")]
                    unsafe {
                        use windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow;
                        let mut hwnd = FLYOUT_HWND.load(Ordering::SeqCst);
                        if hwnd == 0 {
                            hwnd = find_flyout_window();
                            if hwnd != 0 {
                                FLYOUT_HWND.store(hwnd, Ordering::SeqCst);
                            }
                        }
                        if hwnd != 0 {
                            ShowWindow(hwnd, 0); // SW_HIDE = 0
                        }
                    }
                    self.cancel_current_query();
                    self.next_query_generation();
                    FLYOUT_WAKE_READY.store(true, Ordering::SeqCst);
                }
                ctx.request_repaint();
            }
        }

        // Non-blocking result polling: check if the async query has new updates
        let current_generation = self.query_generation;
        let mut clear_query = false;
        if let Some((query_generation, query_handle)) = &self.current_query {
            let is_current_query = *query_generation == current_generation;
            match query_handle.wait(Some(std::time::Duration::from_millis(0))) {
                Signal::Changed => {
                    if is_current_query {
                        self.records = query_handle.get();
                        log_message(&format!(
                            "[Result] Stream update: received {} records so far.",
                            self.records.len()
                        ));
                    } else {
                        clear_query = true;
                    }
                }
                Signal::Finished => {
                    if is_current_query {
                        self.records = query_handle.get();
                        log_message(&format!(
                            "[Result] Query finished: total {} records returned.",
                            self.records.len()
                        ));
                    }
                    clear_query = true;
                }
                Signal::Failed(err) => {
                    if is_current_query {
                        log_message(&format!("[Result] Query failed: {:?}", err));
                    }
                    clear_query = true;
                }
                Signal::TimedOut => {
                    if is_current_query {
                        self.records = query_handle.get();
                    } else {
                        clear_query = true;
                    }
                }
            }
            ctx.request_repaint();
        }
        if clear_query {
            self.current_query = None;
        }

        if !ime_input_active {
            self.apply_viewport_layout(ctx);
        }

        // Partition results into Exact Match and Previews
        let can_show_results = !ime_input_active;
        let exact_match = if can_show_results {
            self.records.iter().find(|rec| rec.key == self.last_input)
        } else {
            None
        };
        let previews: Vec<&Record> = if can_show_results {
            self.records
                .iter()
                .filter(|rec| rec.key != self.last_input)
                .take(3) // Cap at 3 items maximum!
                .collect()
        } else {
            Vec::new()
        };

        let has_exact = exact_match.is_some();
        let has_search_text = can_show_results && !self.input.trim().is_empty();
        let total_items = if self.records.is_empty() && has_search_text {
            2 // Input box (index 0) + "Search on Bing" card (index 1)
        } else {
            1 + (if has_exact { 1 } else { 0 }) + previews.len()
        };

        // Keyboard Arrow Focus Toggle Navigation
        if can_show_results && ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.focus_index = (self.focus_index + 1).min(total_items - 1);
        }
        if can_show_results
            && ctx.input(|i| i.key_pressed(egui::Key::ArrowUp))
            && self.focus_index > 0
        {
            self.focus_index -= 1;
        }

        let input_id = egui::Id::new("search_overlay_input");
        if self.focus_index != 0 {
            ctx.memory_mut(|mem| mem.surrender_focus(input_id));
        }

        egui::TopBottomPanel::top("flyout_input_panel")
            .exact_height(FLYOUT_INPUT_PANEL_HEIGHT)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(0.0)
                    .outer_margin(0.0),
            )
            .show(ctx, |ui| {
                ui.add_space(8.0);
                // Clean black rectangular search box with thin border (highlighted blue if focused!)
                let input_stroke = if self.focus_index == 0 {
                    egui::Stroke::new(
                        2.0,
                        fade_color(egui::Color32::from_rgb(0, 120, 215), self.opacity),
                    )
                } else {
                    egui::Stroke::new(1.5, fade_color(egui::Color32::from_gray(120), self.opacity))
                };

                let frame_response = egui::Frame::none()
                    .fill(fade_color(
                        egui::Color32::from_rgb(15, 15, 15),
                        self.opacity,
                    ))
                    .stroke(input_stroke)
                    .rounding(4.0)
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width()); // Force exact equal width to the results panel
                        let edit_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.input)
                                .id(input_id)
                                .hint_text("Enter word...")
                                .frame(false)
                                .text_color(fade_color(egui::Color32::WHITE, self.opacity)),
                        );

                        // Re-acquire focus dynamically if selected
                        if self.focus_index == 0 && !edit_resp.has_focus() {
                            edit_resp.request_focus();
                        }
                    });

                let frame_rect = frame_response.response.rect;
                let side_color = if self.focus_index == 0 {
                    fade_color(egui::Color32::from_rgb(0, 120, 215), self.opacity)
                } else {
                    fade_color(egui::Color32::from_gray(120), self.opacity)
                };
                let rounding = 4.0;
                let side_stroke = egui::Stroke::new(3.5, side_color);

                ui.painter().line_segment(
                    [
                        egui::pos2(frame_rect.left(), frame_rect.top() + rounding),
                        egui::pos2(frame_rect.left(), frame_rect.bottom() - rounding),
                    ],
                    side_stroke,
                );

                ui.painter().line_segment(
                    [
                        egui::pos2(frame_rect.right(), frame_rect.top() + rounding),
                        egui::pos2(frame_rect.right(), frame_rect.bottom() - rounding),
                    ],
                    side_stroke,
                );
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(0.0)
                    .outer_margin(0.0),
            )
            .show(ctx, |ui| {
                // Results Pane (shown below when we have active records or Search on Bing)
                if self.records.is_empty() && has_search_text {
                    ui.add_space(4.0);
                    egui::Frame::none()
                        .fill(fade_color(
                            egui::Color32::from_black_alpha(220),
                            self.opacity,
                        ))
                        .rounding(8.0)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.vertical(|ui| {
                                let is_focused = self.focus_index == 1;
                                let card_stroke = if is_focused {
                                    egui::Stroke::new(
                                        2.0,
                                        fade_color(
                                            egui::Color32::from_rgb(0, 120, 215),
                                            self.opacity,
                                        ),
                                    )
                                } else {
                                    egui::Stroke::new(
                                        1.0,
                                        fade_color(egui::Color32::from_gray(80), self.opacity),
                                    )
                                };

                                let bing_card = egui::Frame::none()
                                    .fill(fade_color(
                                        egui::Color32::from_rgb(20, 20, 20),
                                        self.opacity,
                                    ))
                                    .stroke(card_stroke)
                                    .rounding(6.0)
                                    .inner_margin(12.0);

                                let response = bing_card.show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("🔍 Search on Bing: ")
                                                .color(fade_color(
                                                    egui::Color32::LIGHT_BLUE,
                                                    self.opacity,
                                                ))
                                                .strong()
                                                .size(13.0),
                                        );
                                        ui.label(
                                            egui::RichText::new(&self.input)
                                                .color(fade_color(
                                                    egui::Color32::WHITE,
                                                    self.opacity,
                                                ))
                                                .size(13.0),
                                        );
                                    });
                                });

                                // Make the card clickable
                                let mut clicked = false;
                                let card_rect = response.response.rect;
                                let card_interaction =
                                    ui.allocate_rect(card_rect, egui::Sense::click());
                                if card_interaction.clicked() {
                                    clicked = true;
                                }

                                // Also highlight focus on hover
                                if card_interaction.hovered() {
                                    self.focus_index = 1;
                                }

                                if clicked
                                    || (ctx.input(|i| i.key_pressed(egui::Key::Enter))
                                        && is_focused)
                                {
                                    let query = self.input.trim();
                                    let mut encoded_query = String::new();
                                    for c in query.chars() {
                                        if c.is_ascii_alphanumeric() {
                                            encoded_query.push(c);
                                        } else if c == ' ' {
                                            encoded_query.push_str("%20");
                                        } else {
                                            for byte in c.to_string().bytes() {
                                                encoded_query.push_str(&format!("%{:02X}", byte));
                                            }
                                        }
                                    }
                                    let url = format!("https://dict.bing.com/w/{}", encoded_query);
                                    ctx.open_url(egui::OpenUrl::new_tab(url));
                                }
                            });
                        });
                } else if can_show_results && !self.records.is_empty() {
                    ui.add_space(4.0); // Reduced distance between input box and results list based on feedback
                    egui::Frame::none()
                        .fill(fade_color(
                            egui::Color32::from_black_alpha(220),
                            self.opacity,
                        ))
                        .rounding(8.0)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width()); // Force exact same width for perfect symmetry
                            ui.vertical(|ui| {
                                // 1. Draw Exact Match Card (Focus index 1)
                                if let Some(rec) = exact_match {
                                    let card_stroke = if self.focus_index == 1 {
                                        egui::Stroke::new(
                                            2.0,
                                            fade_color(
                                                egui::Color32::from_rgb(0, 120, 215),
                                                self.opacity,
                                            ),
                                        )
                                    } else {
                                        egui::Stroke::new(
                                            1.0,
                                            fade_color(egui::Color32::from_gray(80), self.opacity),
                                        )
                                    };

                                    egui::Frame::none()
                                        .fill(fade_color(
                                            egui::Color32::from_rgb(20, 20, 20),
                                            self.opacity,
                                        ))
                                        .stroke(card_stroke)
                                        .rounding(6.0)
                                        .inner_margin(12.0)
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            if let Ok(RecordModel::WordEn(word)) = rec.deserialize()
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.heading(
                                                        egui::RichText::new(&word.word).color(
                                                            fade_color(
                                                                egui::Color32::WHITE,
                                                                self.opacity,
                                                            ),
                                                        ),
                                                    );
                                                    if let Some(pron) = &word.pronunciation {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "US: {}",
                                                                pron.ipa
                                                            ))
                                                            .color(fade_color(
                                                                egui::Color32::LIGHT_BLUE,
                                                                self.opacity,
                                                            )),
                                                        );
                                                    }
                                                });

                                                if let Some(definitions) = &word.definitions {
                                                    for def in definitions {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "{} {}",
                                                                def.pos,
                                                                def.meanings.join(", ")
                                                            ))
                                                            .color(fade_color(
                                                                egui::Color32::from_rgb(
                                                                    225, 225, 225,
                                                                ),
                                                                self.opacity,
                                                            )),
                                                        );
                                                    }
                                                }

                                                if let Some(inf) = &word.inflections {
                                                    let mut infs = Vec::new();
                                                    if let Some(p) = &inf.plural {
                                                        infs.push(format!("pl. {}", p));
                                                    }
                                                    if let Some(pt) = &inf.past_tense {
                                                        infs.push(format!("past {}", pt));
                                                    }
                                                    if let Some(pp) = &inf.past_participle {
                                                        infs.push(format!("pp. {}", pp));
                                                    }
                                                    if let Some(prp) = &inf.present_participle {
                                                        infs.push(format!("pres.p. {}", prp));
                                                    }
                                                    if let Some(ts) = &inf.third_singular {
                                                        infs.push(format!("3sg. {}", ts));
                                                    }
                                                    if !infs.is_empty() {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "Inflections: {}",
                                                                infs.join(", ")
                                                            ))
                                                            .color(fade_color(
                                                                egui::Color32::from_rgb(
                                                                    140, 215, 140,
                                                                ),
                                                                self.opacity,
                                                            )),
                                                        );
                                                    }
                                                }

                                                if let Some(examples) = &word.examples {
                                                    for ex in examples {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "• {}: {}",
                                                                ex.en, ex.zh
                                                            ))
                                                            .color(fade_color(
                                                                egui::Color32::from_rgb(
                                                                    225, 215, 175,
                                                                ),
                                                                self.opacity,
                                                            )),
                                                        );
                                                    }
                                                }
                                            }
                                        });
                                    ui.add_space(8.0);
                                }

                                // 2. Draw Card Previews (Focus index 2+ if exact match exists, or 1+ if not)
                                if !previews.is_empty() {
                                    let start_preview_focus_idx = if has_exact { 2 } else { 1 };

                                    for (idx, rec) in previews.iter().enumerate() {
                                        let target_focus_idx = start_preview_focus_idx + idx;
                                        let is_focused = self.focus_index == target_focus_idx;

                                        let preview_frame = egui::Frame::none()
                                            .fill(if is_focused {
                                                fade_color(
                                                    egui::Color32::from_rgb(0, 80, 160),
                                                    self.opacity * 0.4,
                                                ) // Focused highlighted background!
                                            } else {
                                                egui::Color32::TRANSPARENT
                                            })
                                            .rounding(4.0)
                                            .inner_margin(egui::Margin::symmetric(10.0, 5.0));

                                        preview_frame.show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            if let Ok(RecordModel::WordEn(word)) = rec.deserialize()
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(&word.word)
                                                            .strong()
                                                            .color(fade_color(
                                                                egui::Color32::WHITE,
                                                                self.opacity,
                                                            ))
                                                            .size(13.0),
                                                    );

                                                    if let Some(major) = &word.major {
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                ": {}",
                                                                major
                                                            ))
                                                            .color(fade_color(
                                                                egui::Color32::from_gray(170),
                                                                self.opacity,
                                                            ))
                                                            .size(13.0),
                                                        );
                                                    }
                                                });
                                            }
                                        });
                                        ui.add_space(2.0);
                                    }
                                }
                            });
                        });
                }
            });

        if self.focus_index != 0 {
            ctx.memory_mut(|mem| mem.surrender_focus(input_id));
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // 100% transparent clear color!
    }
}

fn fade_color(color: egui::Color32, opacity: f32) -> egui::Color32 {
    let mut rgba = color.to_array();
    rgba[3] = (rgba[3] as f32 * opacity) as u8;
    egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

/// Compute the flyout window size and outer top-left position so it is
/// horizontally centred on the given monitor and vertically placed at the
/// monitor's mid-line. All inputs/outputs are in the window's logical points;
/// `mon_left`/`mon_top` are the monitor's origin so the result lands on that
/// monitor. Pure (no Win32 / egui context) so it is unit-testable.
fn centered_on_monitor(
    mon_left: f32,
    mon_top: f32,
    mon_w: f32,
    mon_h: f32,
) -> (egui::Vec2, egui::Pos2) {
    let top_y = (mon_h - FLYOUT_INPUT_PANEL_HEIGHT) / 2.0;
    let size = egui::vec2(
        FLYOUT_WINDOW_WIDTH,
        FLYOUT_MAX_WINDOW_HEIGHT
            .min((mon_h - top_y - FLYOUT_BOTTOM_MARGIN).max(FLYOUT_INPUT_PANEL_HEIGHT)),
    );
    let pos = egui::pos2(
        mon_left + (mon_w - FLYOUT_WINDOW_WIDTH) / 2.0,
        mon_top + top_y,
    );
    (size, pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_on_primary_origin_matches_legacy_formula() {
        // Monitor at the virtual-desktop origin reproduces the original
        // primary-monitor centering exactly.
        let (size, pos) = centered_on_monitor(0.0, 0.0, 1920.0, 1080.0);
        let top_y = (1080.0 - FLYOUT_INPUT_PANEL_HEIGHT) / 2.0;
        assert_eq!(size.x, FLYOUT_WINDOW_WIDTH);
        assert!((pos.x - (1920.0 - FLYOUT_WINDOW_WIDTH) / 2.0).abs() < f32::EPSILON);
        assert!((pos.y - top_y).abs() < f32::EPSILON);
    }

    #[test]
    fn centered_on_offset_monitor_shifts_by_origin() {
        // A secondary monitor to the right shifts the position by its origin,
        // keeping the same vertical placement.
        let (_, base) = centered_on_monitor(0.0, 0.0, 1920.0, 1080.0);
        let (_, shifted) = centered_on_monitor(1920.0, 0.0, 1920.0, 1080.0);
        assert!((shifted.x - (base.x + 1920.0)).abs() < f32::EPSILON);
        assert!((shifted.y - base.y).abs() < f32::EPSILON);
    }

    #[test]
    fn same_size_monitor_yields_same_window_size() {
        // Origin offset (including a monitor at negative coordinates) does not
        // change the computed window size.
        let (base_size, _) = centered_on_monitor(0.0, 0.0, 1920.0, 1080.0);
        let (offset_size, _) = centered_on_monitor(-1920.0, 200.0, 1920.0, 1080.0);
        assert_eq!(base_size, offset_size);
    }
}
