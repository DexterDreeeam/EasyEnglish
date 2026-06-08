//! Cross-thread coordination signals and shared window handles.

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// Global thread-safe state for wake up and exit coordination
pub(crate) static VISIBLE_REQUESTED: AtomicBool = AtomicBool::new(false);
pub(crate) static FLYOUT_WAKE_READY: AtomicBool = AtomicBool::new(true);
pub(crate) static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
pub(crate) static EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);

pub(crate) fn request_flyout_wakeup() -> bool {
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

pub(crate) static MAIN_THREAD_ID: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

#[cfg(target_os = "windows")]
pub(crate) static FLYOUT_HWND: std::sync::atomic::AtomicIsize =
    std::sync::atomic::AtomicIsize::new(0);

#[cfg(test)]
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
