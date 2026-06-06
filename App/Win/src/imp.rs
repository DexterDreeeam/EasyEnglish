//! Windows Search Overlay implementation (Pure Compact Black Rectangular Box with Tray Icon).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::path::PathBuf;
use eframe::egui;
use ee_core::{Hub, Storage, Record, RecordModel};
use ee_utils::Signal;

// Global thread-safe state for wake up and exit coordination
static VISIBLE_REQUESTED: AtomicBool = AtomicBool::new(false);
static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
static EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimationState {
    Hidden,
    FadingIn,
    Visible,
    FadingOut,
}

/// Run the Windows Search Overlay App.
pub fn run() -> Result<(), String> {
    // 1. Spawn the background system tray and global mouse/keyboard hook thread
    std::thread::spawn(|| {
        if let Err(e) = run_background_win32_system() {
            eprintln!("Error in Win32 background system: {}", e);
        }
    });

    // 2. Start the eframe GUI application (hidden in tray initially)
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("flyout")    // Name specified as "flyout"
            .with_decorations(false) // Frameless
            .with_transparent(true)   // Transparent background
            .with_always_on_top()     // Always on top
            .with_visible(false)      // Start hidden in tray!
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

        // Configure Microsoft YaHei to support Chinese characters beautifully
        let mut fonts = egui::FontDefinitions::default();
        let font_path = "C:\\Windows\\Fonts\\msyh.ttc"; // Microsoft YaHei
        if std::path::Path::new(font_path).exists() {
            if let Ok(font_data) = std::fs::read(font_path) {
                fonts.font_data.insert(
                    "msyh".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "msyh".to_owned());
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("msyh".to_owned());
            }
        }
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
                println!("[Query] Input changed to: '{}'. Finding suggestions...", trimmed_input);
                // Get the exact word and up to 5 best fuzzy/prefix candidates
                let mut query_keys = vec![trimmed_input.clone()];
                let candidates = ee_core::rank_candidates(
                    &trimmed_input,
                    &self.word_list.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
                    5
                );
                println!("[Query] Generated {} candidate keys: {:?}", candidates.len(), candidates);
                for c in candidates {
                    if c != trimmed_input {
                        query_keys.push(c);
                    }
                }
                
                println!("[Query] Dispatching multi-key query to Hub: {:?}", query_keys);
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
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        // Handle global exit requests from Tray Icon menu
        if EXIT_REQUESTED.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Handle auto-close when the flyout window loses foreground focus
        if self.focus_grace_frames > 0 {
            self.focus_grace_frames -= 1;
        } else if self.animation_state == AnimationState::Visible && !ctx.input(|i| i.viewport().focused.unwrap_or(true)) {
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
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
                ctx.request_repaint();
            }
        }

        // Non-blocking result polling: check if the async query has new updates
        if let Some(query_handle) = &self.current_query {
            match query_handle.wait(Some(std::time::Duration::from_millis(0))) {
                Signal::Changed => {
                    self.records = query_handle.get();
                    println!("[Result] Stream update: received {} records so far.", self.records.len());
                }
                Signal::Finished => {
                    self.records = query_handle.get();
                    println!("[Result] Query finished: total {} records returned.", self.records.len());
                    self.current_query = None;
                }
                Signal::Failed(err) => {
                    println!("[Result] Query failed: {:?}", err);
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
        let previews: Vec<&Record> = self.records.iter().filter(|rec| rec.key != self.last_input).collect();
        
        let has_exact = exact_match.is_some();
        let total_items = 1 + (if has_exact { 1 } else { 0 }) + previews.len();

        // Keyboard Arrow Focus Toggle Navigation
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.focus_index = (self.focus_index + 1).min(total_items - 1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if self.focus_index > 0 {
                self.focus_index -= 1;
            }
        }

        // Dynamic Height Calculation based on split layout
        let mut desired_height = 56.0; // Base: input box
        if !self.records.is_empty() {
            let mut results_height = 16.0; // padding
            
            // Exact match Card height
            if let Some(rec) = exact_match {
                results_height += 36.0;
                if let Ok(model) = rec.deserialize() {
                    if let RecordModel::WordEn(word) = model {
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
            }
            
            // Previews lines height
            if !previews.is_empty() {
                results_height += 12.0;
                results_height += (previews.len() * 26) as f32;
            }

            let results_height = results_height.min(300.0);
            desired_height += results_height + 14.0;
        }

        // Apply window resize and center command dynamically on the main screen (stabilized centering during animation)
        let window_width = 550.0;
        let anim_padding = if self.animation_state == AnimationState::Visible { 0.0 } else { 30.0 };
        let physical_height = desired_height + anim_padding;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(window_width, physical_height)));

        let scale = ctx.pixels_per_point();
        let (physical_w, physical_h) = get_screen_dimensions();
        let screen_w = physical_w / scale;
        let screen_h = physical_h / scale;
        let x = (screen_w - window_width) / 2.0;
        let y = (screen_h - desired_height) / 2.0;
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));

        // Translucent container with NO window background
        let transparent_panel = egui::CentralPanel::default().frame(
            egui::Frame::none().fill(egui::Color32::TRANSPARENT)
        );

        transparent_panel.show(ctx, |ui| {
            ui.add_space(8.0 + self.offset_y);

            // Clean black rectangular search box with thin border (highlighted blue if focused!)
            let input_stroke = if self.focus_index == 0 {
                egui::Stroke::new(2.0, fade_color(egui::Color32::from_rgb(0, 120, 215), self.opacity))
            } else {
                egui::Stroke::new(1.5, fade_color(egui::Color32::from_gray(120), self.opacity))
            };

            egui::Frame::none()
                .fill(fade_color(egui::Color32::from_rgb(15, 15, 15), self.opacity))
                .stroke(input_stroke)
                .rounding(4.0)
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    let edit_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .hint_text("Enter word...")
                            .frame(false)
                            .text_color(fade_color(egui::Color32::WHITE, self.opacity))
                    );

                    // Re-acquire focus dynamically if selected
                    if self.focus_index == 0 {
                        edit_resp.request_focus();
                    }
                });

            // Results Pane (shown below when we have active records)
            if !self.records.is_empty() {
                ui.add_space(10.0);
                egui::Frame::none()
                    .fill(fade_color(egui::Color32::from_black_alpha(220), self.opacity))
                    .rounding(8.0)
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            // 1. Draw Exact Match Card (Focus index 1)
                            if let Some(rec) = exact_match {
                                let card_stroke = if self.focus_index == 1 {
                                    egui::Stroke::new(2.0, fade_color(egui::Color32::from_rgb(0, 120, 215), self.opacity))
                                } else {
                                    egui::Stroke::new(1.0, fade_color(egui::Color32::from_gray(80), self.opacity))
                                };

                                egui::Frame::none()
                                    .fill(fade_color(egui::Color32::from_rgb(20, 20, 20), self.opacity))
                                    .stroke(card_stroke)
                                    .rounding(6.0)
                                    .inner_margin(12.0)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        if let Ok(model) = rec.deserialize() {
                                            if let RecordModel::WordEn(word) = model {
                                                ui.horizontal(|ui| {
                                                    ui.heading(egui::RichText::new(&word.word).color(fade_color(egui::Color32::WHITE, self.opacity)));
                                                    if let Some(pron) = &word.pronunciation {
                                                        ui.label(egui::RichText::new(format!("US: {}", pron.ipa))
                                                            .color(fade_color(egui::Color32::LIGHT_BLUE, self.opacity)));
                                                    }
                                                });
                                                
                                                if let Some(definitions) = &word.definitions {
                                                    for def in definitions {
                                                        ui.label(egui::RichText::new(format!("{} {}", def.pos, def.meanings.join(", ")))
                                                            .color(fade_color(egui::Color32::from_rgb(225, 225, 225), self.opacity)));
                                                    }
                                                }

                                                if let Some(inf) = &word.inflections {
                                                    let mut infs = Vec::new();
                                                    if let Some(p) = &inf.plural { infs.push(format!("pl. {}", p)); }
                                                    if let Some(pt) = &inf.past_tense { infs.push(format!("past {}", pt)); }
                                                    if let Some(pp) = &inf.past_participle { infs.push(format!("pp. {}", pp)); }
                                                    if let Some(prp) = &inf.present_participle { infs.push(format!("pres.p. {}", prp)); }
                                                    if let Some(ts) = &inf.third_singular { infs.push(format!("3sg. {}", ts)); }
                                                    if !infs.is_empty() {
                                                        ui.label(egui::RichText::new(format!("Inflections: {}", infs.join(", ")))
                                                            .color(fade_color(egui::Color32::from_rgb(140, 215, 140), self.opacity)));
                                                    }
                                                }

                                                if let Some(examples) = &word.examples {
                                                    for ex in examples {
                                                        ui.label(egui::RichText::new(format!("• {}: {}", ex.en, ex.zh))
                                                            .color(fade_color(egui::Color32::from_rgb(225, 215, 175), self.opacity)));
                                                    }
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
                                            fade_color(egui::Color32::from_rgb(0, 80, 160), self.opacity * 0.4) // Focused highlighted background!
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        })
                                        .rounding(4.0)
                                        .inner_margin(egui::Margin::symmetric(10.0, 5.0));

                                    preview_frame.show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        if let Ok(model) = rec.deserialize() {
                                            if let RecordModel::WordEn(word) = model {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&word.word)
                                                        .strong()
                                                        .color(fade_color(egui::Color32::WHITE, self.opacity))
                                                        .size(13.0));
                                                    
                                                    if let Some(major) = &word.major {
                                                        ui.label(egui::RichText::new(format!(": {}", major))
                                                            .color(fade_color(egui::Color32::from_gray(170), self.opacity))
                                                            .size(13.0));
                                                    }
                                                });
                                            }
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
unsafe extern "system" fn mouse_hook_proc(code: i32, w_param: usize, l_param: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{HC_ACTION, WM_MOUSEWHEEL, MSLLHOOKSTRUCT, CallNextHookEx, FindWindowW, ShowWindow, SetForegroundWindow};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LSHIFT, VK_LMENU};

    if code == HC_ACTION as i32 {
        if w_param as u32 == WM_MOUSEWHEEL {
            let mouse_info = &*(l_param as *const MSLLHOOKSTRUCT);
            let delta = ((mouse_info.mouseData >> 16) & 0xFFFF) as i16;

            if delta > 0 { // Scroll Up (上滑轮)
                let left_shift_pressed = (GetAsyncKeyState(VK_LSHIFT as i32) as u16 & 0x8000) != 0;
                let left_alt_pressed = (GetAsyncKeyState(VK_LMENU as i32) as u16 & 0x8000) != 0;

                if left_shift_pressed && left_alt_pressed {
                    // Wake up & show the Flyout search overlay using native Win32 API
                    let title = "flyout\0".encode_utf16().collect::<Vec<u16>>();
                    let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
                    if hwnd != 0 {
                        ShowWindow(hwnd, 5); // SW_SHOW = 5
                        SetForegroundWindow(hwnd);
                    }

                    VISIBLE_REQUESTED.store(true, Ordering::SeqCst);
                    if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
                        ctx.request_repaint();
                    }
                }
            }
        }
    }
    CallNextHookEx(0, code, w_param, l_param)
}

/// Global keyboard hook to capture LeftAlt+LeftShift+UpArrow.
#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_hook_proc(code: i32, w_param: usize, l_param: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{HC_ACTION, WM_KEYDOWN, WM_SYSKEYDOWN, KBDLLHOOKSTRUCT, CallNextHookEx, FindWindowW, ShowWindow, SetForegroundWindow};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LSHIFT, VK_LMENU, VK_UP};

    if code == HC_ACTION as i32 {
        let msg = w_param as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kbd_info = &*(l_param as *const KBDLLHOOKSTRUCT);
            if kbd_info.vkCode == VK_UP as u32 {
                let left_shift_pressed = (GetAsyncKeyState(VK_LSHIFT as i32) as u16 & 0x8000) != 0;
                let left_alt_pressed = (GetAsyncKeyState(VK_LMENU as i32) as u16 & 0x8000) != 0;

                if left_shift_pressed && left_alt_pressed {
                    // Wake up & show the Flyout search overlay using native Win32 API
                    let title = "flyout\0".encode_utf16().collect::<Vec<u16>>();
                    let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
                    if hwnd != 0 {
                        ShowWindow(hwnd, 5); // SW_SHOW = 5
                        SetForegroundWindow(hwnd);
                    }

                    VISIBLE_REQUESTED.store(true, Ordering::SeqCst);
                    if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
                        ctx.request_repaint();
                    }
                }
            }
        }
    }
    CallNextHookEx(0, code, w_param, l_param)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn tray_wnd_proc(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::Foundation::POINT;

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
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::UI::Shell::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

    unsafe {
        let h_instance = GetModuleHandleW(std::ptr::null());
        
        let class_name = "EasyEnglishTrayWndClass\0".encode_utf16().collect::<Vec<u16>>();
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

        let window_title = "EasyEnglishTrayWindow\0".encode_utf16().collect::<Vec<u16>>();
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            0,
            0, 0, 0, 0,
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
        for i in 0..len {
            nid.szTip[i] = tooltip[i];
        }

        if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
            return Err("Failed to create tray icon".to_string());
        }

        let mouse_hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook_proc),
            h_instance,
            0,
        );

        if mouse_hook == 0 {
            Shell_NotifyIconW(NIM_DELETE, &nid);
            return Err("Failed to set global mouse hook".to_string());
        }

        let kbd_hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook_proc),
            h_instance,
            0,
        );

        if kbd_hook == 0 {
            UnhookWindowsHookEx(mouse_hook);
            Shell_NotifyIconW(NIM_DELETE, &nid);
            return Err("Failed to set global keyboard hook".to_string());
        }

        let mut msg = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut msg, 0, 0, 0) != 0 {
            TranslateMessage(&mut msg);
            DispatchMessageW(&mut msg);
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
    println!("[Scan] Scanning directory: {:?}", dict_dir);
    let mut highest_version = 0;
    let mut highest_path = None;

    if let Ok(entries) = std::fs::read_dir(&dict_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if filename.starts_with("word_en_v") && filename.ends_with(".sqlite") {
                    let version_part = &filename["word_en_v".len()..(filename.len() - ".sqlite".len())];
                    if let Ok(v) = version_part.parse::<usize>() {
                        println!("[Scan] Found database: {} (v{})", filename, v);
                        if v > highest_version {
                            highest_version = v;
                            highest_path = Some(path);
                        }
                    }
                }
            }
        }
    }
    println!("[Scan] Selected highest database: {:?}", highest_path);
    highest_path
}

fn load_highest_version_word_list() -> Vec<String> {
    let dict_dir = get_db_path(""); // Get Dict/ folder
    println!("[List] Scanning directory for word list: {:?}", dict_dir);
    let mut highest_version = 0;
    let mut highest_file = None;

    if let Ok(entries) = std::fs::read_dir(&dict_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if filename.starts_with("word_list_v") {
                    let version_part = &filename["word_list_v".len()..];
                    if let Ok(v) = version_part.parse::<usize>() {
                        println!("[List] Found word list: {} (v{})", filename, v);
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
        println!("[List] Loading selected word list: {:?}", path);
        if let Ok(file) = std::fs::File::open(&path) {
            let reader = std::io::BufReader::new(file);
            use std::io::BufRead;
            let list: Vec<String> = reader.lines().flatten().collect();
            println!("[List] Loaded {} words successfully.", list.len());
            return list;
        }
    }
    println!("[List] No word list loaded!");
    Vec::new()
}

fn get_db_path(filename: &str) -> PathBuf {
    let path = std::env::current_dir().unwrap_or_default().join("Dict").join(filename);
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
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
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
