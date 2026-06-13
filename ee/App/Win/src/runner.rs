//! Process entry point: spawns the tray thread and runs the eframe app.

#[cfg(debug_assertions)]
use crate::logging::init_debug_logging;
use crate::logging::log_message;
use crate::overlay::{SearchOverlayApp, FLYOUT_MAX_WINDOW_HEIGHT, FLYOUT_WINDOW_WIDTH};
use crate::signals::{request_flyout_wakeup, MAIN_THREAD_ID};
use crate::tray::run_background_win32_system;
use eframe::egui;
use std::sync::atomic::Ordering;

/// Run the Windows Search Overlay App.
pub fn run(show_on_start: bool) -> Result<(), String> {
    #[cfg(debug_assertions)]
    init_debug_logging();

    install_crash_hook();

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

    if show_on_start {
        log_message("[Startup] --show requested; waking flyout on first frame.");
        request_flyout_wakeup();
    }

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

/// Install a panic hook that appends the panic payload + location to
/// `C:\.ee\crash.txt`, so a crash that happens while the GUI message loop is
/// running (where stderr is not visible under `windows_subsystem = "windows"`)
/// can still be diagnosed. Chained after the default hook.
fn install_crash_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::fs::create_dir_all("C:\\.ee");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("C:\\.ee\\crash.txt")
        {
            use std::io::Write;
            let loc = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "<unknown>".to_string());
            let msg = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "<non-string panic payload>".to_string());
            let _ = writeln!(f, "PANIC at {loc}: {msg}");
        }
        prev(info);
    }));
}
