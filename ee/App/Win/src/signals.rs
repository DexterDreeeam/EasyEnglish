//! Cross-thread coordination signals and shared window handles.

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// Global thread-safe state for wake up and exit coordination
pub(crate) static VISIBLE_REQUESTED: AtomicBool = AtomicBool::new(false);
pub(crate) static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
pub(crate) static EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);
static PENDING_SELECTED_TEXT: Mutex<Option<String>> = Mutex::new(None);

/// Request that the flyout (re)appear. Always accepted: the GUI thread decides
/// whether this is a fresh wake, a relocate to the cursor's monitor, or a no-op
/// refresh. Returns `true` so callers (tray / hotkey) always proceed to show the
/// window.
pub(crate) fn request_flyout_wakeup() -> bool {
    request_flyout_wakeup_with_selected_text(None)
}

/// Request that the flyout (re)appear with optional selected text.
///
/// The selected text is consumed by the GUI thread exactly once. Passing `None`
/// clears any stale selection from an earlier wake request.
pub(crate) fn request_flyout_wakeup_with_selected_text(selected_text: Option<String>) -> bool {
    *PENDING_SELECTED_TEXT.lock().unwrap() = selected_text;
    VISIBLE_REQUESTED.store(true, Ordering::SeqCst);
    if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
        ctx.request_repaint();
    }
    true
}

/// Take the selected text attached to the current wake request, if any.
pub(crate) fn take_pending_selected_text() -> Option<String> {
    PENDING_SELECTED_TEXT.lock().unwrap().take()
}

pub(crate) static MAIN_THREAD_ID: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

#[cfg(target_os = "windows")]
pub(crate) static FLYOUT_HWND: std::sync::atomic::AtomicIsize =
    std::sync::atomic::AtomicIsize::new(0);
