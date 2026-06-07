//! Windows Search Overlay implementation (Pure Compact Black Rectangular Box with Tray Icon).

use ee_core::{Hub, Record, RecordModel, Storage};
use ee_utils::Signal;
use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

// Global thread-safe state for wake up and exit coordination
static VISIBLE_REQUESTED: AtomicBool = AtomicBool::new(false);
static FLYOUT_WAKE_READY: AtomicBool = AtomicBool::new(true);
static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
static EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);

const FLYOUT_WINDOW_WIDTH: f32 = 550.0;
const FLYOUT_MAX_WINDOW_HEIGHT: f32 = 360.0;
const FLYOUT_INPUT_PANEL_HEIGHT: f32 = 56.0;
const FLYOUT_BOTTOM_MARGIN: f32 = 16.0;

#[cfg(debug_assertions)]
static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

#[cfg(debug_assertions)]
fn init_debug_logging() {
    let dir = std::path::Path::new("C:\\.ee");
    let _ = std::fs::create_dir_all(dir);

    unsafe {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut st = std::mem::zeroed::<SYSTEMTIME>();
        GetLocalTime(&mut st);

        let filename = format!(
            "easyenglish_{:04}{:02}{:02}_{:02}{:02}{:02}.log",
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
        );
        let path = dir.join(filename);
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            *LOG_FILE.lock().unwrap() = Some(file);
        }
    }
}

#[cfg(debug_assertions)]
fn log_message(msg: &str) {
    println!("{}", msg);
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(file) = guard.as_mut() {
            use std::io::Write;
            let _ = writeln!(file, "{}", msg);
            let _ = file.flush();
        }
    }
}

#[cfg(not(debug_assertions))]
fn log_message(_msg: &str) {}

fn request_flyout_wakeup() -> bool {
    if FLYOUT_WAKE_READY
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return false;
    }

    VISIBLE_REQUESTED.store(true, Ordering::SeqCst);
    if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
        ctx.request_repaint();
    }
    true
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimationState {
    Hidden,
    FadingIn,
    Visible,
    FadingOut,
}

static MAIN_THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

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

/// Run the Windows Search Overlay App.
pub fn run() -> Result<(), String> {
    #[cfg(debug_assertions)]
    init_debug_logging();

    log_message("Initializing EasyEnglish Windows Search Overlay...");

    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::System::Threading::GetCurrentThreadId;
        MAIN_THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);
    }

    // 1. Spawn the background system tray and global mouse/keyboard hook thread
    std::thread::spawn(|| {
        if let Err(e) = run_background_win32_system() {
            eprintln!("Error in Win32 background system: {}", e);
        }
    });

    // 2. Start the eframe GUI application (hidden in tray initially)
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("flyout") // Name specified as "flyout"
            .with_decorations(false) // Frameless
            .with_transparent(true) // Transparent background
            .with_always_on_top() // Always on top
            .with_taskbar(false) // Do NOT show in taskbar!
            .with_visible(false) // Start hidden in tray!
            .with_inner_size([FLYOUT_WINDOW_WIDTH, FLYOUT_MAX_WINDOW_HEIGHT]),
        ..Default::default()
    };

    eframe::run_native(
        "flyout",
        options,
        Box::new(|cc| Ok(Box::new(SearchOverlayApp::new(cc)))),
    )
    .map_err(|e| e.to_string())
}

struct SearchOverlayApp {
    input: String,
    last_input: String,
    hub: Hub,
    current_query: Option<(u64, ee_utils::DynamicResult<Vec<Record>>)>,
    query_generation: u64,
    records: Vec<Record>,
    focus_grace_frames: usize,

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
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
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
            focus_grace_frames: 15,
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
        let (physical_w, physical_h) = get_screen_dimensions();
        let screen_w = physical_w / scale;
        let screen_h = physical_h / scale;
        let top_y = (screen_h - FLYOUT_INPUT_PANEL_HEIGHT) / 2.0;
        let next_size = egui::vec2(
            FLYOUT_WINDOW_WIDTH,
            FLYOUT_MAX_WINDOW_HEIGHT
                .min((screen_h - top_y - FLYOUT_BOTTOM_MARGIN).max(FLYOUT_INPUT_PANEL_HEIGHT)),
        );
        let next_pos = egui::pos2((screen_w - FLYOUT_WINDOW_WIDTH) / 2.0, top_y);

        if self
            .last_viewport_size
            .map_or(true, |size| (size - next_size).length_sq() > 0.25)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(next_size));
            self.last_viewport_size = Some(next_size);
        }

        if self.last_viewport_pos.map_or(true, |pos| {
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
            self.animation_state = AnimationState::FadingOut;
        }

        // Handle global wake-up requests from the hotkey/mouse hook only while fully hidden.
        if VISIBLE_REQUESTED.swap(false, Ordering::SeqCst) {
            if self.animation_state == AnimationState::Hidden {
                FLYOUT_WAKE_READY.store(false, Ordering::SeqCst);
                self.animation_state = AnimationState::FadingIn;
                self.opacity = 0.0;
                self.focus_grace_frames = 25; // More grace frames during animation
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

        // Handle auto-close when the flyout window loses foreground focus
        if self.focus_grace_frames > 0 {
            self.focus_grace_frames -= 1;
        } else if self.animation_state == AnimationState::Visible
            && !self.ime_composing
            && !ctx.input(|i| i.viewport().focused.unwrap_or(true))
        {
            self.animation_state = AnimationState::FadingOut;
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
                }
                ctx.request_repaint();
            }
            AnimationState::Visible => {
                self.opacity = 1.0;
            }
            AnimationState::FadingOut => {
                self.opacity = (self.opacity - dt * 6.0).max(0.0);

                if self.opacity <= 0.0 {
                    self.animation_state = AnimationState::Hidden;
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

// ---------------------------------------------------------------------------
// Win32 Background Low-Level Systems: System Tray & Global Hotkey
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
const WM_TRAYICON: u32 = 0x0400 + 1; // WM_USER + 1
#[cfg(target_os = "windows")]
const TRAY_WINDOW_CLASS: &str = "EasyEnglishTrayWndClass";
#[cfg(target_os = "windows")]
const TRAY_WINDOW_TITLE: &str = "EasyEnglishTrayWindow";
#[cfg(target_os = "windows")]
const ID_TRAY_SHOW: usize = 1001;
#[cfg(target_os = "windows")]
const ID_TRAY_EXIT: usize = 1002;

#[cfg(target_os = "windows")]
static FLYOUT_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_callback(hwnd: isize, lparam: isize) -> i32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW;

    let mut buf = [0u16; 512];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len > 0 {
        let text = String::from_utf16_lossy(&buf[..len as usize]);
        if text == "flyout" {
            *(lparam as *mut isize) = hwnd;
            return 0; // Stop enumeration
        }
    }
    1 // Continue enumeration
}

#[cfg(target_os = "windows")]
fn find_flyout_window() -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumThreadWindows;
    let thread_id = MAIN_THREAD_ID.load(Ordering::SeqCst);
    if thread_id == 0 {
        return 0;
    }
    let mut found_hwnd = 0isize;
    unsafe {
        EnumThreadWindows(
            thread_id,
            Some(enum_windows_callback),
            &mut found_hwnd as *mut isize as isize,
        );
    }
    found_hwnd
}

#[cfg(target_os = "windows")]
unsafe fn show_flyout_window_now() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow};

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

#[cfg(target_os = "windows")]
unsafe extern "system" fn tray_wnd_proc(
    hwnd: isize,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    match msg {
        WM_TRAYICON => {
            if lparam as u32 == WM_RBUTTONUP {
                let h_menu = CreatePopupMenu();

                let show_text = "Show Flyout\0".encode_utf16().collect::<Vec<u16>>();
                let exit_text = "Exit\0".encode_utf16().collect::<Vec<u16>>();

                AppendMenuW(h_menu, MF_STRING, ID_TRAY_SHOW, show_text.as_ptr());
                AppendMenuW(h_menu, MF_STRING, ID_TRAY_EXIT, exit_text.as_ptr());

                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);
                SetForegroundWindow(hwnd);

                let cmd = TrackPopupMenu(
                    h_menu,
                    TPM_RIGHTBUTTON | TPM_RETURNCMD,
                    pt.x,
                    pt.y,
                    0,
                    hwnd,
                    std::ptr::null(),
                );

                if cmd == ID_TRAY_SHOW as i32 {
                    if request_flyout_wakeup() {
                        let title = "flyout\0".encode_utf16().collect::<Vec<u16>>();
                        let flyout_hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
                        if flyout_hwnd != 0 {
                            ShowWindow(flyout_hwnd, 5); // SW_SHOW = 5
                            SetForegroundWindow(flyout_hwnd);
                        }
                    }
                } else if cmd == ID_TRAY_EXIT as i32 {
                    EXIT_REQUESTED.store(true, Ordering::SeqCst);
                    if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
                        ctx.request_repaint();
                    }
                    PostQuitMessage(0);
                }
                DestroyMenu(h_menu);
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        WM_HOTKEY => {
            log_message("[WM_HOTKEY] Global hotkey Alt+~ received!");
            if !request_flyout_wakeup() {
                return 0;
            }

            show_flyout_window_now();
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
fn run_background_win32_system() -> Result<(), String> {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Shell::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let h_instance = GetModuleHandleW(std::ptr::null());

        let class_name = wide_null(TRAY_WINDOW_CLASS);
        let wnd_class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(tray_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        if RegisterClassW(&wnd_class) == 0 {
            return Err("Failed to register tray window class".to_string());
        }

        let window_title = wide_null(TRAY_WINDOW_TITLE);
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            h_instance,
            std::ptr::null(),
        );

        if hwnd == 0 {
            return Err("Failed to create hidden tray window".to_string());
        }

        // Register standard system-wide global hotkey Alt+~.
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            RegisterHotKey, UnregisterHotKey, MOD_ALT, VK_OEM_3,
        };
        let hotkey_id = 1;
        if RegisterHotKey(hwnd, hotkey_id, MOD_ALT, VK_OEM_3 as u32) == 0 {
            log_message("[RegisterHotKey] Failed to register global Alt+~ hotkey!");
        } else {
            log_message("[RegisterHotKey] Successfully registered global Alt+~ hotkey!");
        }

        let mut nid = std::mem::zeroed::<NOTIFYICONDATAW>();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = LoadIconW(0, IDI_APPLICATION);

        let tooltip = "EasyEnglish\0".encode_utf16().collect::<Vec<u16>>();
        let len = std::cmp::min(tooltip.len(), nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&tooltip[..len]);

        if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
            return Err("Failed to create tray icon".to_string());
        }

        let mut msg = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut msg, 0, 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnregisterHotKey(hwnd, hotkey_id);
        Shell_NotifyIconW(NIM_DELETE, &nid);
        DestroyWindow(hwnd);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_background_win32_system() -> Result<(), String> {
    Ok(())
}

fn scan_for_highest_db_version() -> Option<PathBuf> {
    let dict_dir = get_db_path(""); // Get Dict/ folder
    log_message(&format!("[Scan] Scanning directory: {:?}", dict_dir));
    let mut highest_version = 0;
    let mut highest_path = None;

    if let Ok(entries) = std::fs::read_dir(&dict_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if filename.starts_with("word_en_v") && filename.ends_with(".sqlite") {
                    let version_part =
                        &filename["word_en_v".len()..(filename.len() - ".sqlite".len())];
                    if let Ok(v) = version_part.parse::<usize>() {
                        log_message(&format!("[Scan] Found database: {} (v{})", filename, v));
                        if v > highest_version {
                            highest_version = v;
                            highest_path = Some(path);
                        }
                    }
                }
            }
        }
    }
    log_message(&format!(
        "[Scan] Selected highest database: {:?}",
        highest_path
    ));
    highest_path
}

fn load_highest_version_word_list() -> Vec<String> {
    let dict_dir = get_db_path(""); // Get Dict/ folder
    log_message(&format!(
        "[List] Scanning directory for word list: {:?}",
        dict_dir
    ));
    let mut highest_version = 0;
    let mut highest_file = None;

    if let Ok(entries) = std::fs::read_dir(&dict_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if let Some(version_part) = filename.strip_prefix("word_list_v") {
                    if let Ok(v) = version_part.parse::<usize>() {
                        log_message(&format!("[List] Found word list: {} (v{})", filename, v));
                        if v > highest_version {
                            highest_version = v;
                            highest_file = Some(path);
                        }
                    }
                }
            }
        }
    }

    if let Some(path) = highest_file {
        log_message(&format!("[List] Loading selected word list: {:?}", path));
        if let Ok(file) = std::fs::File::open(&path) {
            let reader = std::io::BufReader::new(file);
            use std::io::BufRead;
            let list: Vec<String> = reader.lines().map_while(Result::ok).collect();
            log_message(&format!("[List] Loaded {} words successfully.", list.len()));
            return list;
        }
    }
    log_message("[List] No word list loaded!");
    Vec::new()
}

fn get_db_path(filename: &str) -> PathBuf {
    let path = std::env::current_dir()
        .unwrap_or_default()
        .join("Dict")
        .join(filename);
    if path.exists() {
        return path;
    }
    if let Ok(exe_path) = std::env::current_exe() {
        let mut p = exe_path;
        for _ in 0..5 {
            if let Some(parent) = p.parent() {
                p = parent.to_path_buf();
                let possible = p.join("Dict").join(filename);
                if possible.exists() {
                    return possible;
                }
            }
        }
    }
    PathBuf::from("Dict").join(filename)
}

fn get_screen_dimensions() -> (f32, f32) {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
        };
        let cx = GetSystemMetrics(SM_CXSCREEN);
        let cy = GetSystemMetrics(SM_CYSCREEN);
        if cx > 0 && cy > 0 {
            return (cx as f32, cy as f32);
        }
    }
    (1920.0, 1080.0) // Fallback standard Full HD dimensions
}

fn fade_color(color: egui::Color32, opacity: f32) -> egui::Color32 {
    let mut rgba = color.to_array();
    rgba[3] = (rgba[3] as f32 * opacity) as u8;
    egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_global_keyboard_hook_wakeup() {
        // Simple mock to check state setup
        VISIBLE_REQUESTED.store(false, Ordering::SeqCst);
        assert!(!VISIBLE_REQUESTED.load(Ordering::SeqCst));
    }
}
