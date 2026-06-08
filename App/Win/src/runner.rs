//! Process entry point: spawns the tray thread and runs the eframe app.

#[cfg(debug_assertions)]
use crate::logging::init_debug_logging;
use crate::logging::log_message;
use crate::overlay::{SearchOverlayApp, FLYOUT_MAX_WINDOW_HEIGHT, FLYOUT_WINDOW_WIDTH};
use crate::signals::MAIN_THREAD_ID;
use crate::tray::run_background_win32_system;
use eframe::egui;
use std::sync::atomic::Ordering;

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
