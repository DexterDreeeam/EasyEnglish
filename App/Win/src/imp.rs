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

/// Run the Windows Search Overlay App.
pub fn run() -> Result<(), String> {
    // 1. Spawn the background system tray and global mouse hook thread
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
            .with_inner_size([460.0, 56.0]),
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
    hub: Hub,
    current_query: Option<ee_utils::DynamicResult<Vec<Record>>>,
    records: Vec<Record>,
}

impl SearchOverlayApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Save egui context to the global handle so the hook thread can trigger redraws
        *EGUI_CTX.lock().unwrap() = Some(cc.egui_ctx.clone());

        // Build the backend Hub with the three real databases
        let mut hub = Hub::new();

        let v1_path = get_db_path("word_en_v1.sqlite");
        let v2_path = get_db_path("word_en_v2.sqlite");
        let v3_path = get_db_path("word_en_v3.sqlite");

        if let Ok(s1) = Storage::new(&v1_path) {
            hub.add_provider(Arc::new(s1));
        }
        if let Ok(s2) = Storage::new(&v2_path) {
            hub.add_provider(Arc::new(s2));
        }
        if let Ok(s3) = Storage::new(&v3_path) {
            hub.add_provider(Arc::new(s3));
        }

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

        Self {
            input: String::new(),
            hub,
            current_query: None,
            records: Vec::new(),
        }
    }

    fn trigger_search(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }

        self.records.clear();
        
        // Launch multi-source async streaming lookup
        let handle = self.hub.query(trimmed);
        self.current_query = Some(handle);
    }
}

impl eframe::App for SearchOverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle ESC key to hide/close the flyout text box
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.input.clear();
            self.records.clear();
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        // Handle global wake-up requests from Mouse Scroll Hook
        if VISIBLE_REQUESTED.swap(false, Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        // Handle global exit requests from Tray Icon menu
        if EXIT_REQUESTED.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Non-blocking result polling: check if the async query has new updates
        if let Some(query_handle) = &self.current_query {
            match query_handle.wait(Some(std::time::Duration::from_millis(0))) {
                Signal::Changed => {
                    self.records = query_handle.get();
                }
                Signal::Finished => {
                    self.records = query_handle.get();
                    self.current_query = None;
                }
                Signal::Failed(_err) => {
                    self.current_query = None;
                }
                Signal::TimedOut => {
                    self.records = query_handle.get();
                }
            }
            ctx.request_repaint();
        }

        // Dynamic Height Calculation: automatically resize the OS window height based on displayed content
        let mut desired_height = 56.0; // Base: input box + vertical margin (no status message)
        if !self.records.is_empty() {
            let mut results_height = 16.0; // padding
            for rec in &self.records {
                results_height += 36.0; // Group header & gap
                if let Ok(model) = rec.deserialize() {
                    match model {
                        RecordModel::WordEn(word) => {
                            results_height += 28.0; // Word title + IPA
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
                        _ => {
                            results_height += 30.0;
                        }
                    }
                }
            }
            let results_height = results_height.min(300.0);
            desired_height += results_height + 14.0; // Spacing & results panel margin
        }

        // Apply window resize command dynamically
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(460.0, desired_height)));

        // Translucent container with NO window background
        let transparent_panel = egui::CentralPanel::default().frame(
            egui::Frame::none().fill(egui::Color32::TRANSPARENT)
        );

        transparent_panel.show(ctx, |ui| {
            ui.add_space(8.0);

            // Clean black rectangular search box with thin border (matching user's request)
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(15, 15, 15)) // Pure deep black
                .stroke(egui::Stroke::new(1.5, egui::Color32::from_gray(120))) // Precise gray border
                .rounding(4.0) // Compact rectangle
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    let edit_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .hint_text("Enter word...")
                            .frame(false)
                            .text_color(egui::Color32::WHITE)
                    );

                    // Re-acquire focus dynamically
                    edit_resp.request_focus();

                    if edit_resp.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.trigger_search();
                    }
                });

            // Results Pane (shown below when we have active records)
            if !self.records.is_empty() {
                ui.add_space(10.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_black_alpha(220)) // Dark semi-transparent container
                    .rounding(8.0)
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            for (i, rec) in self.records.iter().enumerate() {
                                ui.group(|ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(format!("Source DB #{}", i + 1))
                                            .color(egui::Color32::from_gray(130))
                                            .size(11.0));
                                    });
                                    
                                    if let Ok(model) = rec.deserialize() {
                                        match model {
                                            RecordModel::WordEn(word) => {
                                                ui.horizontal(|ui| {
                                                    ui.heading(egui::RichText::new(&word.word).color(egui::Color32::WHITE));
                                                    if let Some(pron) = &word.pronunciation {
                                                        ui.label(egui::RichText::new(format!("US: {}", pron.ipa))
                                                            .color(egui::Color32::LIGHT_BLUE));
                                                    }
                                                });
                                                
                                                if let Some(definitions) = &word.definitions {
                                                    for def in definitions {
                                                        ui.label(egui::RichText::new(format!("{} {}", def.pos, def.meanings.join(", ")))
                                                            .color(egui::Color32::from_rgb(225, 225, 225)));
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
                                                            .color(egui::Color32::from_rgb(140, 215, 140)));
                                                    }
                                                }

                                                if let Some(examples) = &word.examples {
                                                    for ex in examples {
                                                        ui.label(egui::RichText::new(format!("• {}: {}", ex.en, ex.zh))
                                                            .color(egui::Color32::from_rgb(225, 215, 175)));
                                                    }
                                                }
                                            }
                                            _ => {
                                                ui.label(format!("Raw: {}", rec.value));
                                            }
                                        }
                                    } else {
                                        ui.label(format!("Raw Payload: {}", rec.value));
                                    }
                                });
                                ui.add_space(6.0);
                            }
                        });
                    });
            }
        });
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
    use windows_sys::Win32::UI::WindowsAndMessaging::{HC_ACTION, WM_MOUSEWHEEL, MSLLHOOKSTRUCT, CallNextHookEx};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_SHIFT};

    if code == HC_ACTION as i32 {
        if w_param as u32 == WM_MOUSEWHEEL {
            let mouse_info = &*(l_param as *const MSLLHOOKSTRUCT);
            let delta = ((mouse_info.mouseData >> 16) & 0xFFFF) as i16;

            if delta > 0 { // Scroll Up (上滑轮)
                let ctrl_pressed = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;
                let shift_pressed = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;

                if ctrl_pressed && shift_pressed {
                    // Wake up & show the Flyout search overlay
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
            // Right button click (WM_RBUTTONUP in lparam)
            if lparam as u32 == WM_RBUTTONUP {
                let h_menu = CreatePopupMenu();
                
                // Add option to Show and Exit
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
        
        // 1. Register a lightweight, hidden window class to receive Tray Messages
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

        // 2. Create the hidden window
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

        // 3. Setup and register the System Tray Icon
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

        // 4. Register the global low-level Mouse Hook to capture Ctrl+Shift+MouseWheelUp
        let hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook_proc),
            h_instance,
            0,
        );

        if hook == 0 {
            // Clean up tray icon if hook fails
            Shell_NotifyIconW(NIM_DELETE, &nid);
            return Err("Failed to set global mouse hook".to_string());
        }

        // 5. Message Loop to keep the Hook and Tray callbacks active
        let mut msg = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut msg, 0, 0, 0) != 0 {
            TranslateMessage(&mut msg);
            DispatchMessageW(&mut msg);
        }

        // 6. Clean up resources upon exit
        UnhookWindowsHookEx(hook);
        Shell_NotifyIconW(NIM_DELETE, &nid);
        DestroyWindow(hwnd);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_background_win32_system() -> Result<(), String> {
    Ok(())
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
