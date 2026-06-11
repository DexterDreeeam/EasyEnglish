//! The egui search-overlay app: window lifecycle, layout, query, rendering.

use crate::dict::{load_highest_version_word_list, scan_for_highest_db_version};
use crate::focus::{
    evaluate_focus_hide, AnimationState, FocusHideDecision, HIDE_DEBOUNCE, WAKE_FOCUS_GRACE,
};
use crate::logging::log_message;
use crate::signals::{EGUI_CTX, EXIT_REQUESTED, FLYOUT_HWND, VISIBLE_REQUESTED};
use crate::win32::{cursor_monitor_rect, find_flyout_window, flyout_is_foreground};
use ee_core::{Hub, Record, RecordModel, Storage};
use ee_utils::Signal;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub(crate) const FLYOUT_WINDOW_WIDTH: f32 = 550.0;
/// Maximum flyout height. The effective height is the smaller of this and the
/// space available on the monitor below the (top-locked) input box. It is
/// deliberately generous so a rich dictionary Card is shown in full instead of
/// being truncated at the bottom; any unused space is fully transparent and a
/// click there dismisses the flyout. The window size is fixed (not resized per
/// frame) because issuing viewport resize commands while the flyout is focused
/// drops keyboard input on Windows.
pub(crate) const FLYOUT_MAX_WINDOW_HEIGHT: f32 = 900.0;
pub(crate) const FLYOUT_INPUT_PANEL_HEIGHT: f32 = 56.0;
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
    search_key: String,
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
    focus_index: usize,   // 0 = Input box, 1 = Exact Card, 2+ = Card Previews
    arm_card_focus: bool, // set by a Card Preview jump so the next query focuses the Card
    ime_composing: bool,
    word_list: Vec<String>,
    word_list_cn: Vec<String>,
    // When the current results are Chinese (`search_is_chinese`), this holds the
    // index of the focused English button within the focused preview row, or
    // `None` when the whole row is selected.
    cn_active_button: Option<usize>,
    search_is_chinese: bool,
    // When the flyout first became non-foreground (reset whenever it is foreground),
    // used to debounce the auto-hide against transient focus blips (e.g. the IME
    // candidate window).
    unfocused_since: Option<std::time::Instant>,
    // Height animation for the dark results panel. `display` is the currently
    // shown (clipped) height, `target` the last-measured natural content height,
    // and `velocity` the per-second rate carried by the critically-damped spring
    // (`smooth_damp`) that drives `display` toward `target`. Tracking velocity
    // keeps both position AND speed continuous when `target` jumps as a new Card
    // Preview streams in, so each card slides open from ~0 to full height without
    // the jerk a fixed-duration tween produces at every retarget. This is pure
    // egui painting (the OS window is never resized), so it does not affect
    // keyboard input.
    results_pane_display: f32,
    results_pane_target: f32,
    results_pane_velocity: f32,
}

impl SearchOverlayApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Save egui context to the global handle so the hook thread can trigger redraws
        *EGUI_CTX.lock().unwrap() = Some(cc.egui_ctx.clone());

        // Build the backend Hub and dynamically scan/load only the highest version dictionary database.
        // Providers are queried in registration order, which is also the display
        // priority: Note > Offline Dict > Search. The first provider that returns
        // a record for the search key is shown as the Card, the rest as Previews.
        let mut hub = Hub::new();
        if let Some(highest_db) = scan_for_highest_db_version("word_en") {
            if let Ok(storage) = Storage::new(&highest_db) {
                hub.add_provider(Arc::new(storage));
            }
        }
        // Separate Chinese → English provider. Latin keys only hit the English
        // database and Chinese keys only hit this one, so a single hub serves both.
        if let Some(cn_db) = scan_for_highest_db_version("word_cn") {
            if let Ok(storage) = Storage::new(&cn_db) {
                hub.add_provider(Arc::new(storage));
            }
        }

        // Load the corresponding word lists in memory for instantaneous fuzzy/prefix searches
        let word_list = load_highest_version_word_list("word_en");
        let word_list_cn = load_highest_version_word_list("word_cn");

        configure_fonts(&cc.egui_ctx);

        // Configure standard visuals to use 100% transparent fills for window/panel background
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::TRANSPARENT;
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        cc.egui_ctx.set_visuals(visuals);

        Self {
            input: String::new(),
            last_input: String::new(),
            search_key: String::new(),
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
            arm_card_focus: false,
            ime_composing: false,
            word_list,
            word_list_cn,
            cn_active_button: None,
            search_is_chinese: false,
            unfocused_since: None,
            results_pane_display: 0.0,
            results_pane_target: 0.0,
            results_pane_velocity: 0.0,
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
        let (max_size, next_pos) = centered_on_monitor(mon_left, mon_top, screen_w, screen_h);
        // Use a fixed, monitor-bounded size. The window is intentionally NOT
        // resized to fit its content: sending viewport resize commands while the
        // flyout holds focus drops keyboard input on Windows, leaving the flyout
        // unresponsive after wake. A generous height (see FLYOUT_MAX_WINDOW_HEIGHT)
        // keeps rich Cards from being truncated; unused area is transparent.
        let next_size = max_size;

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

        #[cfg(all(target_os = "windows", debug_assertions))]
        {
            use std::sync::atomic::{AtomicI8, Ordering as O};
            static LAST_FOCUSED: AtomicI8 = AtomicI8::new(-1);
            let f = ctx.input(|i| i.focused) as i8;
            if LAST_FOCUSED.swap(f, O::Relaxed) != f {
                log_message(&format!("[egui] viewport focused changed -> {}", f == 1));
            }
        }

        let ime_event_this_frame = self.update_ime_composition_state(ctx);
        let ime_input_active = self.ime_composing || ime_event_this_frame;
        if self.ime_composing {
            self.cancel_current_query();
            self.next_query_generation();
            self.records.clear();
            self.focus_index = 0;
            self.cn_active_button = None;
        }

        // Instant search on typing: if the input has changed, trigger a fresh search immediately
        if !ime_input_active {
            let trimmed = self.input.trim().to_string();
            let raw_input = trimmed.to_lowercase();
            if raw_input != self.last_input {
                self.last_input = raw_input.clone();
                let query_generation = self.next_query_generation();

                // Immediately cancel the previous query thread to release resources
                self.cancel_current_query();
                self.records.clear();
                // A Card Preview jump auto-selects the resulting exact Card (index 1);
                // every other (manual) query focuses the input box (index 0). The arm
                // is single-shot: a later manual keystroke clears it and focuses input.
                self.focus_index = focus_for_new_query(self.arm_card_focus);
                self.arm_card_focus = false;
                self.cn_active_button = None;
                self.search_is_chinese = input_is_chinese(&trimmed);

                if self.search_is_chinese {
                    // Chinese → English: exact + prefix only (no fuzzy edit distance).
                    // `prefix_candidates` already returns the exact term first, so its
                    // output is used directly as the query keys.
                    self.search_key = raw_input.clone();
                    if !raw_input.is_empty() {
                        let query_keys = ee_core::prefix_candidates(
                            &raw_input,
                            &self
                                .word_list_cn
                                .iter()
                                .map(|s| s.as_str())
                                .collect::<Vec<&str>>(),
                            5,
                        );
                        log_message(&format!(
                            "[Query] Chinese input '{}' → {} term(s): {:?}",
                            raw_input,
                            query_keys.len(),
                            query_keys
                        ));
                        if !query_keys.is_empty() {
                            let handle = self.hub.query(&query_keys);
                            self.current_query = Some((query_generation, handle));
                        }
                    }
                } else {
                    // `!Xxx` requests an exact-only lookup; otherwise we also gather
                    // fuzzy/prefix candidates. Either way the effective key is stored
                    // in `search_key` and used to recognise exact matches when
                    // partitioning results into the Card and the Card Previews.
                    let (search_key, exact_lookup) = parse_query_input(&raw_input);
                    self.search_key = search_key.clone();

                    if !search_key.is_empty() {
                        if exact_lookup {
                            log_message(&format!(
                                "[Query] Exact lookup for '{}' (no fuzzy candidates).",
                                search_key
                            ));
                            let handle = self.hub.query(&[search_key]);
                            self.current_query = Some((query_generation, handle));
                        } else {
                            log_message(&format!(
                                "[Query] Input changed to: '{}'. Finding suggestions...",
                                search_key
                            ));
                            // Exact key first, then up to 5 best fuzzy/prefix candidates.
                            let mut query_keys = vec![search_key.clone()];
                            let candidates = ee_core::rank_candidates(
                                &search_key,
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
                                if c != search_key {
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
            }
        }

        // Handle ESC key to hide/close the flyout text box
        if !ime_input_active && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            log_message("[State] ESC pressed → FadingOut.");
            self.animation_state = AnimationState::FadingOut;
        }

        // Handle global wake-up requests from the hotkey / tray. The flyout always
        // (re)appears on the monitor under the mouse cursor. A fresh wake fades it
        // in there; pressing the hotkey again while it is already shown on another
        // monitor relocates it — it leaves the old monitor and reappears under the
        // cursor. There is only ever one flyout, on the active monitor.
        if VISIBLE_REQUESTED.swap(false, Ordering::SeqCst) {
            let monitor = cursor_monitor_rect();
            let was_hidden = self.animation_state == AnimationState::Hidden;
            let relocating = !was_hidden
                && self
                    .target_monitor
                    .is_none_or(|current| !same_monitor(current, monitor));

            if was_hidden || relocating {
                self.target_monitor = Some(monitor);
                self.last_viewport_pos = None; // force reposition onto the target monitor
            }
            if was_hidden {
                self.opacity = 0.0;
            }
            if self.animation_state != AnimationState::Visible {
                // From Hidden or FadingOut, (re)appear by fading in from the current
                // opacity; if already Visible we keep it and just relocate/refocus.
                self.animation_state = AnimationState::FadingIn;
            }
            // (Re)start the focus grace: we are re-acquiring foreground, so the
            // window must not be judged "unfocused" during the transition or move.
            self.wake_at = Some(std::time::Instant::now());
            self.was_focused = false;

            log_message(&format!(
                "[State] wake ({}) → {:?} on monitor at ({}, {}) {}x{}.",
                if was_hidden {
                    "fresh"
                } else if relocating {
                    "relocate"
                } else {
                    "refresh"
                },
                self.animation_state,
                monitor.0,
                monitor.1,
                monitor.2,
                monitor.3
            ));

            #[cfg(target_os = "windows")]
            unsafe {
                let mut hwnd = FLYOUT_HWND.load(Ordering::SeqCst);
                if hwnd == 0 {
                    hwnd = find_flyout_window();
                    if hwnd != 0 {
                        FLYOUT_HWND.store(hwnd, Ordering::SeqCst);
                    }
                }
                if hwnd != 0 {
                    use windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow;
                    ShowWindow(hwnd, 5); // SW_SHOW = 5
                    crate::win32::focus_flyout_and_clear_alt(hwnd);
                }
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            #[cfg(all(target_os = "windows", debug_assertions))]
            log_message(&format!(
                "[Focus] post-wake {}",
                crate::win32::focus_debug_snapshot()
            ));
        }

        // Handle global exit requests from Tray Icon menu
        if EXIT_REQUESTED.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Track foreground acquisition and decide whether to auto-hide. We query
        // the OS foreground window (GetForegroundWindow) rather than egui/winit
        // focus events: the flyout is created hidden and shown via raw Win32, so
        // winit does not track its focus on the first show — the OS query is
        // authoritative and is ready before the flyout is even visible. A short
        // post-wake grace lets SetForegroundWindow take effect before we judge
        // the window "unfocused"; once it has ever been foreground, any later loss
        // hides immediately regardless of the grace.
        let is_foreground = flyout_is_foreground();
        if is_foreground == Some(true) {
            self.was_focused = true;
        }

        // Track how long the flyout has been continuously non-foreground so the
        // auto-hide can debounce against transient blips. `Some(true)`/`None`
        // (foreground or unknown) clears the timer; `Some(false)` starts it.
        match is_foreground {
            Some(false) => {
                if self.unfocused_since.is_none() {
                    self.unfocused_since = Some(std::time::Instant::now());
                }
            }
            _ => self.unfocused_since = None,
        }
        let unfocused_long_enough = self
            .unfocused_since
            .map(|t| t.elapsed() >= HIDE_DEBOUNCE)
            .unwrap_or(false);

        // Keep egui's viewport-focus flag in sync with the real OS focus. The flyout
        // is shown via raw Win32, so winit can miss the focus transition and leave
        // `i.focused` false even though the OS foreground IS the flyout. egui gates
        // the text caret *and* IME enablement on `i.focused`, so a stale-false flag
        // means no cursor and no Chinese input. When the OS says we are foreground
        // but egui disagrees, re-assert focus through winit so it catches up.
        if is_foreground == Some(true)
            && !ctx.input(|i| i.focused)
            && self.animation_state != AnimationState::Hidden
        {
            #[cfg(debug_assertions)]
            log_message(
                "[Focus] re-assert: OS fg==flyout but egui unfocused → ViewportCommand::Focus",
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.request_repaint();
        }

        let grace_expired = self
            .wake_at
            .map(|started| started.elapsed() >= WAKE_FOCUS_GRACE)
            .unwrap_or(true);
        match evaluate_focus_hide(
            self.animation_state,
            is_foreground,
            self.ime_composing,
            self.was_focused,
            grace_expired,
            unfocused_long_enough,
        ) {
            FocusHideDecision::Hide => {
                log_message(&format!(
                    "[Focus] visible & not foreground (foreground={:?}, was_focused={}, \
                     grace_expired={}) → FadingOut.",
                    is_foreground, self.was_focused, grace_expired
                ));
                self.animation_state = AnimationState::FadingOut;
            }
            // Not foreground yet but still inside the grace window: keep repainting
            // so we promptly notice it becoming foreground (or the grace expiring)
            // even while the user provides no further input.
            FocusHideDecision::WaitForFocus => {
                ctx.request_repaint();
            }
            FocusHideDecision::Keep => {}
        }

        // Animation State Machine updates
        match self.animation_state {
            AnimationState::Hidden => {
                // Ensure the window is actually hidden while logically Hidden.
                // eframe shows the window once rendering starts (regardless of
                // with_visible(false)); a transparent, always-on-top, non
                // click-through window would otherwise silently swallow every
                // mouse click over its footprint. A real wake sets
                // VISIBLE_REQUESTED before showing the window, so it has already
                // become FadingIn above and never reaches here — only the
                // unwanted eframe/startup show does. Re-check next frame in case
                // eframe re-shows it during startup; once it stays hidden the
                // event loop goes idle as before.
                #[cfg(target_os = "windows")]
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        IsWindowVisible, ShowWindow,
                    };
                    let mut hwnd = FLYOUT_HWND.load(Ordering::SeqCst);
                    if hwnd == 0 {
                        hwnd = find_flyout_window();
                        if hwnd != 0 {
                            FLYOUT_HWND.store(hwnd, Ordering::SeqCst);
                        }
                    }
                    if hwnd != 0 && IsWindowVisible(hwnd) != 0 {
                        ShowWindow(hwnd, 0); // SW_HIDE = 0
                        ctx.request_repaint();
                    }
                }
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
                    // Snap the height spring to a fully-collapsed, at-rest state
                    // so the next open grows from 0 rather than animating down
                    // from a leftover height if the flyout was hidden mid-growth.
                    self.results_pane_display = 0.0;
                    self.results_pane_target = 0.0;
                    self.results_pane_velocity = 0.0;
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

        // Partition results into the exact-match Card and the Card Previews.
        // Providers are queried in priority order (Note > Offline Dict > Search),
        // so the first record whose key equals `search_key` is the highest
        // priority exact match and becomes the Card; any further same-key records
        // from lower-priority providers, followed by fuzzy candidates, become
        // Previews. This holds in both normal and `!Xxx` exact mode.
        let can_show_results = !ime_input_active;
        let exact_idx = if can_show_results {
            self.records
                .iter()
                .position(|rec| rec.key == self.search_key)
        } else {
            None
        };
        let exact_match = exact_idx.map(|i| &self.records[i]);
        let previews: Vec<&Record> = if can_show_results {
            self.records
                .iter()
                .enumerate()
                .filter(|(i, _)| Some(*i) != exact_idx)
                .map(|(_, rec)| rec)
                .take(5) // Cap at 5 preview items maximum.
                .collect()
        } else {
            Vec::new()
        };

        let has_exact = exact_match.is_some();
        let has_search_text = can_show_results && !self.search_key.is_empty();

        // Chinese → English mode: every matched record is a preview row with up to
        // three English buttons; there is no exact "Card".
        let chinese_mode = self.search_is_chinese && can_show_results && !self.records.is_empty();
        let cn_rows: Vec<(String, Vec<String>)> = if chinese_mode {
            self.records
                .iter()
                .filter_map(|rec| {
                    if let Ok(RecordModel::WordCn(w)) = rec.deserialize() {
                        Some((w.word, w.english.into_iter().take(3).collect()))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        // The "Search on Bing" entry is always the last focusable item whenever
        // there is query text (even with zero dictionary results).
        let total_items = if !has_search_text {
            1 // Input box only
        } else if self.records.is_empty() {
            2 // Input box (0) + "Search on Bing" (1)
        } else if chinese_mode {
            // Input box + Chinese preview rows + "Search on Bing"
            1 + cn_rows.len() + 1
        } else {
            // Input box + (Card?) + Previews + "Search on Bing"
            1 + (if has_exact { 1 } else { 0 }) + previews.len() + 1
        };

        // Keyboard Arrow Focus Toggle Navigation
        if chinese_mode {
            let nav = ctx.input(|i| {
                if i.key_pressed(egui::Key::ArrowDown) {
                    Some(CnNavKey::Down)
                } else if i.key_pressed(egui::Key::ArrowUp) {
                    Some(CnNavKey::Up)
                } else if i.key_pressed(egui::Key::ArrowLeft) {
                    Some(CnNavKey::Left)
                } else if i.key_pressed(egui::Key::ArrowRight) {
                    Some(CnNavKey::Right)
                } else {
                    None
                }
            });
            if let Some(key) = nav {
                // Buttons available in the currently-focused row (0 when not on a row).
                let buttons = if self.focus_index >= 1 && self.focus_index <= cn_rows.len() {
                    cn_rows[self.focus_index - 1].1.len()
                } else {
                    0
                };
                let (fi, btn) = cn_focus_step(
                    key,
                    self.focus_index,
                    self.cn_active_button,
                    cn_rows.len(),
                    buttons,
                );
                self.focus_index = fi;
                self.cn_active_button = btn;
            }
        } else {
            if can_show_results && ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.focus_index = (self.focus_index + 1).min(total_items - 1);
            }
            if can_show_results
                && ctx.input(|i| i.key_pressed(egui::Key::ArrowUp))
                && self.focus_index > 0
            {
                self.focus_index -= 1;
            }
        }

        let input_id = egui::Id::new("search_overlay_input");
        if self.focus_index != 0 {
            ctx.memory_mut(|mem| mem.surrender_focus(input_id));
        }

        egui::TopBottomPanel::top("flyout_input_panel")
            .exact_height(FLYOUT_INPUT_PANEL_HEIGHT)
            .show_separator_line(false)
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
                        // Force exact equal width to the results panel.
                        ui.set_width(ui.available_width());
                        // `cursor_at_end` (egui default) places the caret at the
                        // end of the text the frame focus is gained, so the caret
                        // is visible immediately without any manual TextEditState
                        // manipulation. Overwriting the stored state after `show`
                        // clobbered egui's own IME preedit bookkeeping and broke
                        // Chinese (IME) input, so we must NOT touch the state here.
                        let edit_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.input)
                                .id(input_id)
                                .hint_text("Enter word...")
                                .frame(false)
                                .text_color(fade_color(egui::Color32::WHITE, self.opacity)),
                        );

                        // Re-acquire focus dynamically if the input box is the
                        // selected element but egui has not granted it focus yet.
                        if self.focus_index == 0 && !edit_resp.has_focus() {
                            edit_resp.request_focus();
                        }
                        // Clicking the input box (while a card/preview was
                        // selected) returns selection to the input box.
                        if edit_resp.gained_focus() || edit_resp.clicked() {
                            self.focus_index = 0;
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

        // A click on the flyout's empty (transparent) area should dismiss it,
        // like clicking outside — that area is part of the window so it does not
        // trigger the foreground-loss auto-hide on its own.
        let mut dismiss_clicked = false;
        // Advance the results-panel height spring toward the height measured last
        // frame so rows are revealed gradually as the query streams in. A
        // critically-damped spring keeps both the height AND its velocity
        // continuous when `target` jumps (each streamed Card Preview steps it up),
        // so the panel glides open instead of jerking at every new card. egui's
        // `stable_dt` is used (a smoothed frame delta) rather than a raw clock
        // delta, which keeps the motion even under frame-time jitter. `new_target`
        // is recomputed below from the freshly laid-out content (0 when no panel
        // is shown, so the panel springs closed). Pure egui painting — the OS
        // window is never resized.
        let stable_dt = ctx.input(|i| i.stable_dt).clamp(1.0 / 1000.0, 0.1);
        self.results_pane_display = smooth_damp(
            self.results_pane_display,
            self.results_pane_target,
            &mut self.results_pane_velocity,
            RESULTS_ANIM_SMOOTH_TIME,
            stable_dt,
        );
        let panel_display = self.results_pane_display;
        let mut new_target = 0.0_f32;
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
                    new_target =
                        draw_growing_results_panel(ui, self.opacity, panel_display, |ui| {
                            ui.set_width(ui.available_width());
                            ui.vertical(|ui| {
                                let bing_hovered = render_bing_entry(
                                    ui,
                                    ctx,
                                    &self.search_key,
                                    self.opacity,
                                    self.focus_index == 1,
                                );
                                if bing_hovered {
                                    self.focus_index = 1;
                                }
                            });
                        });
                } else if chinese_mode {
                    ui.add_space(4.0);
                    new_target =
                        draw_growing_results_panel(ui, self.opacity, panel_display, |ui| {
                            ui.set_width(ui.available_width());
                            ui.vertical(|ui| {
                                // One preview row per matched Chinese term: left the
                                // Chinese label, right up to three English buttons.
                                for (row_idx, (term, english)) in cn_rows.iter().enumerate() {
                                    let focus_row = row_idx + 1; // row 0 is the input box
                                    let row_selected = self.focus_index == focus_row
                                        && self.cn_active_button.is_none();
                                    let active_button = if self.focus_index == focus_row {
                                        self.cn_active_button
                                    } else {
                                        None
                                    };
                                    if let Some(action) = render_cn_preview_row(
                                        ui,
                                        ctx,
                                        term,
                                        english,
                                        self.opacity,
                                        row_selected,
                                        active_button,
                                    ) {
                                        match action {
                                            CnRowAction::HoverRow => {
                                                self.focus_index = focus_row;
                                                self.cn_active_button = None;
                                            }
                                            CnRowAction::HoverButton(b) => {
                                                self.focus_index = focus_row;
                                                self.cn_active_button = Some(b);
                                            }
                                            CnRowAction::Activate(word) => {
                                                self.input = exact_query_for(&word);
                                                self.arm_card_focus = true;
                                                log_message(&format!(
                                                    "[Select] cn '{}' → exact English '{}'.",
                                                    term, self.input
                                                ));
                                                ctx.request_repaint();
                                            }
                                        }
                                    }
                                    ui.add_space(2.0);
                                }

                                // "Search on Bing" — always the bottom row.
                                let bing_focus_idx = cn_rows.len() + 1;
                                ui.add_space(6.0);
                                let bing_hovered = render_bing_entry(
                                    ui,
                                    ctx,
                                    &self.search_key,
                                    self.opacity,
                                    self.focus_index == bing_focus_idx,
                                );
                                if bing_hovered {
                                    self.focus_index = bing_focus_idx;
                                    self.cn_active_button = None;
                                }
                            });
                        });
                } else if can_show_results && !self.records.is_empty() {
                    ui.add_space(4.0); // Reduced distance between input box and results list based on feedback
                    new_target =
                        draw_growing_results_panel(ui, self.opacity, panel_display, |ui| {
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
                                                                "[{}]",
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

                                // 2. Draw Card Previews (Focus index 2+ if exact match exists, or 1+ if not).
                                //    Each preview is selectable (click / Enter / Space) to run an exact lookup.
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

                                        let preview_response = preview_frame.show(ui, |ui| {
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
                                        // Selecting a preview entry runs an exact
                                        // lookup of that word: a mouse click, or
                                        // pressing Enter / Space while it is the
                                        // keyboard-focused item, fills the search
                                        // box with `! <word>` (note the space) so
                                        // the next frame's instant-search resolves
                                        // it as an exact-only query.
                                        let preview_rect = preview_response.response.rect;
                                        let preview_interaction =
                                            ui.allocate_rect(preview_rect, egui::Sense::click());
                                        if preview_interaction.hovered()
                                            && ui.input(|i| i.pointer.is_moving())
                                        {
                                            self.focus_index = target_focus_idx;
                                        }
                                        let selected = preview_interaction.clicked()
                                            || (is_focused
                                                && ctx.input(|i| {
                                                    i.key_pressed(egui::Key::Enter)
                                                        || i.key_pressed(egui::Key::Space)
                                                }));
                                        if selected {
                                            self.input = exact_query_for(&rec.key);
                                            // Jumping from a preview auto-selects the
                                            // exact Card on the next frame's search.
                                            self.arm_card_focus = true;
                                            log_message(&format!(
                                                "[Select] preview '{}' → exact lookup '{}'.",
                                                rec.key, self.input
                                            ));
                                            ctx.request_repaint();
                                        }

                                        ui.add_space(2.0);
                                    }
                                }

                                // 3. "Search on Bing" entry — always the bottom row
                                //    of the results pane whenever there is query text.
                                let bing_focus_idx =
                                    (if has_exact { 2 } else { 1 }) + previews.len();
                                if has_exact || !previews.is_empty() {
                                    ui.add_space(6.0);
                                }
                                let bing_hovered = render_bing_entry(
                                    ui,
                                    ctx,
                                    &self.search_key,
                                    self.opacity,
                                    self.focus_index == bing_focus_idx,
                                );
                                if bing_hovered {
                                    self.focus_index = bing_focus_idx;
                                }
                            });
                        });
                }

                // Catch clicks on the empty area below the content (not on the
                // input box or a result card) and dismiss, like clicking outside.
                let remaining = ui.available_size();
                if remaining.y > 1.0 {
                    let resp = ui.allocate_response(remaining, egui::Sense::click());
                    if resp.clicked() {
                        dismiss_clicked = true;
                    }
                }
            });

        // Persist the freshly measured content height. While the height spring is
        // still moving — either its position has not reached the target or it
        // still carries velocity — request an *immediate* repaint every frame so
        // the panel is redrawn at the full display refresh rate. The Visible
        // state otherwise throttles to a 100 ms idle tick; without this explicit
        // per-frame request the growth drops to ~10 fps and looks choppy. Pure
        // egui painting — the OS window is never resized.
        self.results_pane_target = new_target;
        if (panel_display - self.results_pane_target).abs() > 0.5
            || self.results_pane_velocity.abs() > 1.0
        {
            ctx.request_repaint();
        }

        if dismiss_clicked {
            log_message("[Click] empty flyout area clicked → FadingOut.");
            self.animation_state = AnimationState::FadingOut;
            ctx.request_repaint();
        }

        if self.focus_index != 0 {
            ctx.memory_mut(|mem| mem.surrender_focus(input_id));
        }

        // Per-change focus diagnostic (debug builds only; compiled out of release).
        #[cfg(all(target_os = "windows", debug_assertions))]
        log_focus_diag(
            ctx.input(|i| i.focused),
            is_foreground,
            self.animation_state,
            ctx.memory(|m| m.has_focus(input_id)),
            ctx.wants_keyboard_input(),
            self.opacity,
        );
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

/// Approximate time (seconds) the height spring takes to substantially close
/// the gap to a new target. Smaller is snappier; this value glides a step in
/// roughly a tenth of a second. Used as the `smooth_time` of [`smooth_damp`].
pub(crate) const RESULTS_ANIM_SMOOTH_TIME: f32 = 0.09;

/// Critically-damped spring step (Game Programming Gems 4 / Unity's
/// `Mathf.SmoothDamp`). Moves `current` toward `target` while carrying `vel`
/// across frames so both the position and its velocity stay continuous even
/// when `target` changes abruptly — exactly what keeps the results panel from
/// jerking each time a streamed Card Preview steps the target height up.
/// `smooth_time` is the approximate time to reach the target and `dt` the frame
/// delta, both in seconds. Pure (no egui), so it is unit-testable.
pub(crate) fn smooth_damp(
    current: f32,
    target: f32,
    vel: &mut f32,
    smooth_time: f32,
    dt: f32,
) -> f32 {
    let smooth_time = smooth_time.max(1e-4);
    let omega = 2.0 / smooth_time;
    let x = omega * dt;
    // Padé-style approximation of exp(-x), matching Unity's implementation.
    let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let change = current - target;
    let temp = (*vel + omega * change) * dt;
    *vel = (*vel - omega * temp) * exp;
    let output = target + (change + temp) * exp;
    // Snap and stop the spring once it is within a sub-pixel of the target so it
    // settles instead of creeping (and so repaints can stop).
    if (target - output).abs() < 0.5 && vel.abs() < 1.0 {
        *vel = 0.0;
        target
    } else {
        output
    }
}

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

/// Compute the flyout window size and outer top-left position so it is
/// horizontally centred on the given monitor and vertically placed at the
/// monitor's mid-line. All inputs/outputs are in the window's logical points;
/// `mon_left`/`mon_top` are the monitor's origin so the result lands on that
/// monitor. Pure (no Win32 / egui context) so it is unit-testable.
pub(crate) fn centered_on_monitor(
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

/// Whether two monitor rects (physical left/top/width/height) refer to the same
/// monitor. Comparing the origin is sufficient because distinct monitors never
/// share a top-left corner; the small tolerance absorbs any rounding.
pub(crate) fn same_monitor(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1.0 && (a.1 - b.1).abs() < 1.0
}

/// Parse the (already trimmed, lower-cased) overlay input into the effective
/// search key and whether an exact-only lookup was requested. A leading `!`
/// requests exact mode: `!apple` → ("apple", true); `apple` → ("apple", false).
pub(crate) fn parse_query_input(raw: &str) -> (String, bool) {
    if let Some(rest) = raw.strip_prefix('!') {
        (rest.trim().to_string(), true)
    } else {
        (raw.to_string(), false)
    }
}

/// Build the overlay input that selects `word` for an exact lookup.
///
/// Selecting a card preview fills the search box with `! <word>` (note the
/// space after `!`). The next frame's instant-search feeds this back through
/// [`parse_query_input`], which strips the `!` and surrounding space to yield
/// an exact-only query for `word`.
pub(crate) fn exact_query_for(word: &str) -> String {
    format!("! {}", word.trim())
}

/// Whether the input should be treated as a Chinese → English search: true when
/// it contains at least one CJK ideograph.
pub(crate) fn input_is_chinese(input: &str) -> bool {
    input
        .chars()
        .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

/// Debug-only focus diagnostic: logs the (egui-focus, OS-foreground, animation,
/// input-widget-focus) state once per change so a repro produces a compact
/// timeline. Entirely compiled out of release builds; keeps all diagnostic state
/// in a thread-local rather than on `SearchOverlayApp`.
#[cfg(all(target_os = "windows", debug_assertions))]
fn log_focus_diag(
    focused: bool,
    os_fg: Option<bool>,
    anim: AnimationState,
    input_focus: bool,
    wants_kb: bool,
    opacity: f32,
) {
    use std::cell::RefCell;
    type FocusDiagState = (bool, Option<bool>, AnimationState, bool);
    thread_local! {
        static LAST: RefCell<Option<FocusDiagState>> = const { RefCell::new(None) };
    }
    let st: FocusDiagState = (focused, os_fg, anim, input_focus);
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        if *last != Some(st) {
            *last = Some(st);
            log_message(&format!(
                "[Diag] egui_focused={} os_fg={:?} anim={:?} input_focus={} wants_kb={} opacity={:.2}",
                focused, os_fg, anim, input_focus, wants_kb, opacity
            ));
        }
    });
}

/// Focus index to land on after dispatching a fresh query. A Card Preview jump
/// arms `arm_card_focus` so focus lands directly on the exact-match Card (index
/// 1); every other (manual) query focuses the input box (index 0).
pub(crate) fn focus_for_new_query(arm_card_focus: bool) -> usize {
    if arm_card_focus {
        1
    } else {
        0
    }
}

/// Arrow keys handled by the two-level Chinese focus model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CnNavKey {
    Up,
    Down,
    Left,
    Right,
}

/// Pure transition for the Chinese results' two-level focus.
///
/// Focus index layout: `0` = input box, `1..=num_rows` = Chinese preview rows,
/// `num_rows + 1` = the "Search on Bing" entry. `active_button` is the focused
/// English button within the focused row, or `None` when the whole row is
/// selected. Up/Down move between rows and always drop back to row selection;
/// Right enters / advances the English buttons; Left retreats and, from the
/// first button, returns to row selection.
pub(crate) fn cn_focus_step(
    key: CnNavKey,
    focus_index: usize,
    active_button: Option<usize>,
    num_rows: usize,
    buttons_in_focused_row: usize,
) -> (usize, Option<usize>) {
    let last = num_rows + 1; // "Search on Bing" index
    let on_row = focus_index >= 1 && focus_index <= num_rows;
    match key {
        CnNavKey::Down => ((focus_index + 1).min(last), None),
        CnNavKey::Up => (focus_index.saturating_sub(1), None),
        CnNavKey::Right => {
            if on_row && buttons_in_focused_row > 0 {
                let next = match active_button {
                    None => 0,
                    Some(b) => (b + 1).min(buttons_in_focused_row - 1),
                };
                (focus_index, Some(next))
            } else {
                (focus_index, active_button)
            }
        }
        CnNavKey::Left => {
            if on_row {
                match active_button {
                    Some(b) if b > 0 => (focus_index, Some(b - 1)),
                    _ => (focus_index, None),
                }
            } else {
                (focus_index, active_button)
            }
        }
    }
}

/// Decide which English candidate a Space/Enter keypress should activate on the
/// currently focused Chinese row.
///
/// When a specific English button is focused (`active_button = Some(b)`), that
/// word is activated. As a shortcut, when the whole row is selected (no button
/// focused) and the row exposes exactly one candidate, that lone word is
/// activated directly — pressing Space/Enter behaves as if the user had first
/// stepped onto the single candidate. Returns `None` when there is nothing to
/// activate.
pub(crate) fn cn_row_activation_index(
    active_button: Option<usize>,
    row_selected: bool,
    num_english: usize,
) -> Option<usize> {
    match active_button {
        Some(b) if b < num_english => Some(b),
        None if row_selected && num_english == 1 => Some(0),
        _ => None,
    }
}

/// Outcome of interacting with a Chinese preview row.
enum CnRowAction {
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
fn render_cn_preview_row(
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
                            && hit.hovered()
                            && ui.input(|i| i.pointer.is_moving())
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
        if row_hit.hovered() && ui.input(|i| i.pointer.is_moving()) {
            action = Some(CnRowAction::HoverRow);
        }
    }

    action
}

/// Render the always-present "Search on Bing" entry (the bottom row of the
/// results pane). Returns true when a moving pointer hovers it, so the caller
/// can move keyboard focus onto it. Opening the Bing URL (mouse click, or
/// Enter/Space while the entry is focused) is handled internally.
fn render_bing_entry(
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
                    egui::RichText::new("🔍 Search on Bing: ")
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

    interaction.hovered() && ui.input(|i| i.pointer.is_moving())
}
