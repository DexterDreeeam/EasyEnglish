//! Cross-thread coordination signals and shared window handles.

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// Global thread-safe state for wake up and exit coordination
pub(crate) static VISIBLE_REQUESTED: AtomicBool = AtomicBool::new(false);
pub(crate) static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
pub(crate) static EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);

/// Request that the flyout (re)appear. Always accepted: the GUI thread decides
/// whether this is a fresh wake, a relocate to the cursor's monitor, or a no-op
/// refresh. Returns `true` so callers (tray / hotkey) always proceed to show the
/// window.
pub(crate) fn request_flyout_wakeup() -> bool {
    VISIBLE_REQUESTED.store(true, Ordering::SeqCst);
    if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
        ctx.request_repaint();
    }
    true
}

pub(crate) static MAIN_THREAD_ID: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

#[cfg(target_os = "windows")]
pub(crate) static FLYOUT_HWND: std::sync::atomic::AtomicIsize =
    std::sync::atomic::AtomicIsize::new(0);
