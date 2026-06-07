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
static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
static EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);

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

fn log_message(msg: &str) {
    #[cfg(debug_assertions)]
    {
        println!("{}", msg);
        if let Ok(mut guard) = LOG_FILE.lock() {
            if let Some(file) = guard.as_mut() {
                use std::io::Write;
                let _ = writeln!(file, "{}", msg);
                let _ = file.flush();
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimationState {
    Hidden,
    FadingIn,
    Visible,
    FadingOut,
}

/// Run the Windows Search Overlay App.
pub fn run() -> Result<(), String> {
    #[cfg(debug_assertions)]
    init_debug_logging();

    log_message("Initializing EasyEnglish Windows Search Overlay...");

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
            .with_inner_size([550.0, 56.0]),
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
    current_query: Option<ee_utils::DynamicResult<Vec<Record>>>,
    records: Vec<Record>,
    focus_grace_frames: usize,

    animation_state: AnimationState,
    opacity: f32,
    offset_y: f32,
    last_frame: std::time::Instant,
    focus_index: usize, // 0 = Input box, 1 = Exact Card, 2+ = Card Previews
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

        // Configure Segoe UI (pristine English & IPA) and Microsoft YaHei (Chinese fallback)
        let mut fonts = egui::FontDefinitions::default();

        let segoe_data = include_bytes!("../../Assets/segoeui.ttf");
        let msyh_data = include_bytes!("../../Assets/msyh.ttc");

        fonts
            .font_data
            .insert("segoe".to_owned(), egui::FontData::from_static(segoe_data));
        fonts
            .font_data
            .insert("msyh".to_owned(), egui::FontData::from_static(msyh_data));

        let proportional = fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap();
        proportional.clear(); // Clear default fallback fonts
        proportional.push("segoe".to_owned());
        proportional.push("msyh".to_owned());

        let monospace = fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap();
        monospace.clear();
        monospace.push("segoe".to_owned());
        monospace.push("msyh".to_owned());

        cc.egui_ctx.set_fonts(fonts);

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
            records: Vec::new(),
            focus_grace_frames: 15,
            animation_state: AnimationState::Hidden,
            opacity: 0.0,
            offset_y: 30.0,
            last_frame: std::time::Instant::now(),
            focus_index: 0,
            word_list,
        }
    }
}

impl eframe::App for SearchOverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Compute precise delta time (dt) for high-performance fluid framerate independence
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        // Instant search on typing: if the input has changed, trigger a fresh search immediately
        let trimmed_input = self.input.trim().to_lowercase();
        if trimmed_input != self.last_input {
            self.last_input = trimmed_input.clone();

            // Immediately cancel the previous query thread to release resources
            if let Some(query_handle) = &self.current_query {
                query_handle.cancel();
            }
            self.records.clear();
            self.current_query = None;
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
                self.current_query = Some(handle);
            }
        }

        // Handle ESC key to hide/close the flyout text box
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.animation_state = AnimationState::FadingOut;
        }

        // Handle global wake-up requests from Mouse Scroll Hook
        if VISIBLE_REQUESTED.swap(false, Ordering::SeqCst) {
            self.animation_state = AnimationState::FadingIn;
            self.opacity = 0.0;
            self.offset_y = 30.0; // Start 30px lower to float upwards!
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
        }

        // Handle global exit requests from Tray Icon menu
        if EXIT_REQUESTED.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Handle auto-close when the flyout window loses foreground focus
        if self.focus_grace_frames > 0 {
            self.focus_grace_frames -= 1;
        } else if self.animation_state == AnimationState::Visible
            && !ctx.input(|i| i.viewport().focused.unwrap_or(true))
        {
            self.animation_state = AnimationState::FadingOut;
        }

        // Animation State Machine updates
        match self.animation_state {
            AnimationState::Hidden => {
                return;
            }
            AnimationState::FadingIn => {
                self.opacity = (self.opacity + dt * 6.0).min(1.0);
                self.offset_y = (self.offset_y - dt * 180.0).max(0.0);

                if self.opacity >= 1.0 && self.offset_y <= 0.0 {
                    self.animation_state = AnimationState::Visible;
                }
                ctx.request_repaint();
            }
            AnimationState::Visible => {
                self.opacity = 1.0;
                self.offset_y = 0.0;
            }
            AnimationState::FadingOut => {
                self.opacity = (self.opacity - dt * 6.0).max(0.0);
                self.offset_y = (self.offset_y + dt * 120.0).min(30.0);

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
                }
                ctx.request_repaint();
            }
        }

        // Non-blocking result polling: check if the async query has new updates
        if let Some(query_handle) = &self.current_query {
            match query_handle.wait(Some(std::time::Duration::from_millis(0))) {
                Signal::Changed => {
                    self.records = query_handle.get();
                    log_message(&format!(
                        "[Result] Stream update: received {} records so far.",
                        self.records.len()
                    ));
                }
                Signal::Finished => {
                    self.records = query_handle.get();
                    log_message(&format!(
                        "[Result] Query finished: total {} records returned.",
                        self.records.len()
                    ));
                    self.current_query = None;
                }
                Signal::Failed(err) => {
                    log_message(&format!("[Result] Query failed: {:?}", err));
                    self.current_query = None;
                }
                Signal::TimedOut => {
                    self.records = query_handle.get();
                }
            }
            ctx.request_repaint();
        }

        // Partition results into Exact Match and Previews
        let exact_match = self.records.iter().find(|rec| rec.key == self.last_input);
        let previews: Vec<&Record> = self
            .records
            .iter()
            .filter(|rec| rec.key != self.last_input)
            .take(3) // Cap at 3 items maximum!
            .collect();

        let has_exact = exact_match.is_some();
        let total_items = if self.records.is_empty() && !self.input.trim().is_empty() {
            2 // Input box (index 0) + "Search on Bing" card (index 1)
        } else {
            1 + (if has_exact { 1 } else { 0 }) + previews.len()
        };

        // Keyboard Arrow Focus Toggle Navigation
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.focus_index = (self.focus_index + 1).min(total_items - 1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) && self.focus_index > 0 {
            self.focus_index -= 1;
        }

        // Dynamic Height Calculation based on split layout
        let mut desired_height = 56.0; // Base: input box
        if self.records.is_empty() && !self.input.trim().is_empty() {
            // Height for "Search on Bing" card
            let results_height = 16.0 + 36.0 + 12.0; // container padding + card height
            desired_height += results_height + 14.0;
        } else if !self.records.is_empty() {
            let mut results_height = 16.0; // padding

            // Exact match Card height
            if let Some(rec) = exact_match {
                results_height += 36.0;
                if let Ok(RecordModel::WordEn(word)) = rec.deserialize() {
                    results_height += 28.0;
                    if let Some(definitions) = &word.definitions {
                        results_height += (definitions.len() * 22) as f32;
                    }
                    if let Some(_inf) = &word.inflections {
                        results_height += 22.0;
                    }
                    if let Some(examples) = &word.examples {
                        results_height += (examples.len() * 22) as f32;
                    }
                }
            }

            // Previews lines height
            if !previews.is_empty() {
                results_height += 12.0;
                results_height += (previews.len() * 26) as f32;
            }

            desired_height += results_height + 14.0;
        }

        // Apply window resize and center command dynamically on the main screen (stabilized centering during animation)
        let window_width = 550.0;
        let anim_padding = if self.animation_state == AnimationState::Visible {
            0.0
        } else {
            30.0
        };
        let physical_height = desired_height + anim_padding;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            window_width,
            physical_height,
        )));

        let scale = ctx.pixels_per_point();
        let (physical_w, physical_h) = get_screen_dimensions();
        let screen_w = physical_w / scale;
        let screen_h = physical_h / scale;
        let x = (screen_w - window_width) / 2.0;
        let base_height = 56.0;
        let y = (screen_h - base_height) / 2.0; // Keep the top of the input box stationary, allowing the list to grow purely downwards!
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));

        // Translucent container with NO window background and zero margins for perfect left-right alignment
        let transparent_panel = egui::CentralPanel::default().frame(
            egui::Frame::none()
                .fill(egui::Color32::TRANSPARENT)
                .inner_margin(0.0)
                .outer_margin(0.0),
        );

        transparent_panel.show(ctx, |ui| {
            ui.add_space(8.0 + self.offset_y);

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
                            .hint_text("Enter word...")
                            .frame(false)
                            .text_color(fade_color(egui::Color32::WHITE, self.opacity)),
                    );

                    // Re-acquire focus dynamically if selected
                    if self.focus_index == 0 {
                        edit_resp.request_focus();
                    }
                });

            // Extra left/right blue borders if focused to make them wider while keeping top/bottom same!
            if self.focus_index == 0 {
                let frame_rect = frame_response.response.rect;
                let blue_color = fade_color(egui::Color32::from_rgb(0, 120, 215), self.opacity);
                let rounding = 4.0;
                let extra_stroke = egui::Stroke::new(3.5, blue_color);

                // Left thick border
                ui.painter().line_segment(
                    [
                        egui::pos2(frame_rect.left(), frame_rect.top() + rounding),
                        egui::pos2(frame_rect.left(), frame_rect.bottom() - rounding),
                    ],
                    extra_stroke,
                );

                // Right thick border
                ui.painter().line_segment(
                    [
                        egui::pos2(frame_rect.right(), frame_rect.top() + rounding),
                        egui::pos2(frame_rect.right(), frame_rect.bottom() - rounding),
                    ],
                    extra_stroke,
                );
            }

            // Results Pane (shown below when we have active records or Search on Bing)
            if self.records.is_empty() && !self.input.trim().is_empty() {
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
                                    fade_color(egui::Color32::from_rgb(0, 120, 215), self.opacity),
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
                                            .color(fade_color(egui::Color32::WHITE, self.opacity))
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
                                || (ctx.input(|i| i.key_pressed(egui::Key::Enter)) && is_focused)
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
            } else if !self.records.is_empty() {
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
                                        if let Ok(RecordModel::WordEn(word)) = rec.deserialize() {
                                            ui.horizontal(|ui| {
                                                ui.heading(egui::RichText::new(&word.word).color(
                                                    fade_color(egui::Color32::WHITE, self.opacity),
                                                ));
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
                                                            egui::Color32::from_rgb(225, 225, 225),
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
                                                            egui::Color32::from_rgb(140, 215, 140),
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
                                                            egui::Color32::from_rgb(225, 215, 175),
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
                                        if let Ok(RecordModel::WordEn(word)) = rec.deserialize() {
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
                                                        egui::RichText::new(format!(": {}", major))
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
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // 100% transparent clear color!
    }
}

// ---------------------------------------------------------------------------
// Win32 Background Low-Level Systems: System Tray & Global Mouse Wheel Hook
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
const WM_TRAYICON: u32 = 0x0400 + 1; // WM_USER + 1
#[cfg(target_os = "windows")]
const ID_TRAY_SHOW: usize = 1001;
#[cfg(target_os = "windows")]
const ID_TRAY_EXIT: usize = 1002;

#[cfg(target_os = "windows")]
static FLYOUT_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg(target_os = "windows")]
static LEFT_ALT_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static LEFT_SHIFT_DOWN: AtomicBool = AtomicBool::new(false);

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
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumWindows;
    let mut found_hwnd = 0isize;
    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            &mut found_hwnd as *mut isize as isize,
        );
    }
    found_hwnd
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn mouse_hook_proc(code: i32, w_param: usize, l_param: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, SetForegroundWindow, ShowWindow, HC_ACTION, MSLLHOOKSTRUCT, WM_MOUSEWHEEL,
    };

    if code == HC_ACTION as i32 && w_param as u32 == WM_MOUSEWHEEL {
        let mouse_info = &*(l_param as *const MSLLHOOKSTRUCT);
        let delta = ((mouse_info.mouseData >> 16) & 0xFFFF) as i16;

        if delta > 0 {
            // Scroll Up (上滑轮)
            let left_shift_pressed = LEFT_SHIFT_DOWN.load(Ordering::SeqCst);
            let left_alt_pressed = LEFT_ALT_DOWN.load(Ordering::SeqCst);

            if left_shift_pressed && left_alt_pressed {
                // Wake up & show the Flyout search overlay using native Win32 API
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

                VISIBLE_REQUESTED.store(true, Ordering::SeqCst);
                if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
                    ctx.request_repaint();
                }
                return 1; // Consume mouse scroll! Block it from target window!
            }
        }
    }
    CallNextHookEx(0, code, w_param, l_param)
}

/// Global keyboard hook to capture LeftAlt+LeftShift+UpArrow.
#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_hook_proc(code: i32, w_param: usize, l_param: isize) -> isize {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_LMENU, VK_LSHIFT, VK_UP};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, SetForegroundWindow, ShowWindow, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN,
        WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    if code == HC_ACTION as i32 {
        let msg = w_param as u32;
        let kbd_info = &*(l_param as *const KBDLLHOOKSTRUCT);

        // 1. Track Key State Transitions independently of external windows
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            if kbd_info.vkCode == VK_LMENU as u32 {
                LEFT_ALT_DOWN.store(true, Ordering::SeqCst);
            } else if kbd_info.vkCode == VK_LSHIFT as u32 {
                LEFT_SHIFT_DOWN.store(true, Ordering::SeqCst);
            }
        } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
            if kbd_info.vkCode == VK_LMENU as u32 {
                LEFT_ALT_DOWN.store(false, Ordering::SeqCst);
            } else if kbd_info.vkCode == VK_LSHIFT as u32 {
                LEFT_SHIFT_DOWN.store(false, Ordering::SeqCst);
            }
        }

        // 2. Intercept Up Arrow combinations in real-time
        if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && kbd_info.vkCode == VK_UP as u32 {
            let left_shift_pressed = LEFT_SHIFT_DOWN.load(Ordering::SeqCst);
            let left_alt_pressed = LEFT_ALT_DOWN.load(Ordering::SeqCst);

            if left_shift_pressed && left_alt_pressed {
                // Wake up & show the Flyout search overlay using native Win32 API
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

                VISIBLE_REQUESTED.store(true, Ordering::SeqCst);
                if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
                    ctx.request_repaint();
                }
                return 1; // Consume Up Arrow! Block it from propagating to active window!
            }
        }
    }
    CallNextHookEx(0, code, w_param, l_param)
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
                    let title = "flyout\0".encode_utf16().collect::<Vec<u16>>();
                    let flyout_hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
                    if flyout_hwnd != 0 {
                        ShowWindow(flyout_hwnd, 5); // SW_SHOW = 5
                        SetForegroundWindow(flyout_hwnd);
                    }

                    VISIBLE_REQUESTED.store(true, Ordering::SeqCst);
                    if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
                        ctx.request_repaint();
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

        let class_name = "EasyEnglishTrayWndClass\0"
            .encode_utf16()
            .collect::<Vec<u16>>();
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

        let window_title = "EasyEnglishTrayWindow\0"
            .encode_utf16()
            .collect::<Vec<u16>>();
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

        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), h_instance, 0);

        if mouse_hook == 0 {
            Shell_NotifyIconW(NIM_DELETE, &nid);
            return Err("Failed to set global mouse hook".to_string());
        }

        let kbd_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), h_instance, 0);

        if kbd_hook == 0 {
            UnhookWindowsHookEx(mouse_hook);
            Shell_NotifyIconW(NIM_DELETE, &nid);
            return Err("Failed to set global keyboard hook".to_string());
        }

        let mut msg = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut msg, 0, 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnhookWindowsHookEx(kbd_hook);
        UnhookWindowsHookEx(mouse_hook);
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
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_LMENU, VK_LSHIFT, VK_UP,
    };

    fn send_key(vk: u16, keyup: bool) {
        unsafe {
            let mut input = std::mem::zeroed::<INPUT>();
            input.r#type = INPUT_KEYBOARD;
            input.Anonymous.ki = KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if keyup { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: 0,
            };
            SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
        }
    }

    #[test]
    fn test_global_keyboard_hook_wakeup() {
        // Clear any previous state
        VISIBLE_REQUESTED.store(false, Ordering::SeqCst);

        // Spawn win32 background system thread
        let _handle = std::thread::spawn(|| {
            let _ = run_background_win32_system();
        });

        // Let the hooks register (sleep 300ms)
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Simulate pressing LeftAlt + LeftShift + UpArrow
        send_key(VK_LMENU, false); // Left Alt Down
        send_key(VK_LSHIFT, false); // Left Shift Down
        send_key(VK_UP, false); // Up Arrow Down

        std::thread::sleep(std::time::Duration::from_millis(50));

        send_key(VK_UP, true); // Up Arrow Up
        send_key(VK_LSHIFT, true); // Left Shift Up
        send_key(VK_LMENU, true); // Left Alt Up

        // Wait a bit for hook to process and set flag
        std::thread::sleep(std::time::Duration::from_millis(150));

        // Assert that VISIBLE_REQUESTED is indeed true!
        let triggered = VISIBLE_REQUESTED.load(Ordering::SeqCst);
        assert!(triggered, "Hotkeys did NOT trigger VISIBLE_REQUESTED!");
    }
}
