//! Low-level Win32 helpers: window lookup, screen metrics, show/foreground.

use crate::signals::{FLYOUT_HWND, MAIN_THREAD_ID};
use std::sync::atomic::Ordering;

#[cfg(target_os = "windows")]
pub(crate) fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn get_screen_dimensions() -> (f32, f32) {
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

/// Physical bounds (left, top, width, height) of the monitor that currently
/// contains the mouse cursor. Falls back to the primary monitor at the origin
/// if the query fails (or on non-Windows hosts).
pub(crate) fn cursor_monitor_rect() -> (f32, f32, f32, f32) {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) != 0 {
            let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(monitor, &mut info) != 0 {
                let r = info.rcMonitor;
                return (
                    r.left as f32,
                    r.top as f32,
                    (r.right - r.left) as f32,
                    (r.bottom - r.top) as f32,
                );
            }
        }
    }
    let (w, h) = get_screen_dimensions();
    (0.0, 0.0, w, h)
}

/// Whether the flyout window is currently the OS foreground window.
///
/// Uses `GetForegroundWindow`, which is authoritative and available immediately.
/// We deliberately avoid winit/egui focus events here: the flyout is created
/// hidden and shown via raw Win32, so winit does not reliably track its focus on
/// the first show (its `viewport().focused` stays `None`, which the auto-hide
/// logic treats as "focused" and never hides). Returns `None` only if the flyout
/// handle has not been resolved yet.
pub(crate) fn flyout_is_foreground() -> Option<bool> {
    let flyout = FLYOUT_HWND.load(Ordering::SeqCst);
    if flyout == 0 {
        return None;
    }
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let fg = GetForegroundWindow();
        if fg == 0 {
            // Briefly there can be no foreground window during a switch; treat as
            // unknown (the caller keeps the flyout) rather than a spurious hide.
            None
        } else {
            Some(fg == flyout)
        }
    }
}

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
pub(crate) fn find_flyout_window() -> isize {
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
pub(crate) unsafe fn show_flyout_window_now() {
    use crate::overlay::{FLYOUT_INPUT_PANEL_HEIGHT, FLYOUT_WINDOW_WIDTH};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsWindowVisible, SetForegroundWindow, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOSIZE,
        SWP_NOZORDER,
    };

    let mut hwnd = FLYOUT_HWND.load(Ordering::SeqCst);
    if hwnd == 0 {
        hwnd = find_flyout_window();
        if hwnd != 0 {
            FLYOUT_HWND.store(hwnd, Ordering::SeqCst);
        }
    }
    if hwnd == 0 {
        return;
    }

    // When the flyout is hidden (a fresh wake) move it onto the monitor under the
    // cursor *before* making it visible. Otherwise the OS shows it at its previous
    // location and egui only relocates it a frame later, which makes the window
    // flash on the old monitor the first time it appears on a new one. While
    // already visible (a relocate) we leave the move to egui's layout.
    if IsWindowVisible(hwnd) == 0 {
        let (left, top, w, h) = cursor_monitor_rect();
        let x = (left + (w - FLYOUT_WINDOW_WIDTH) / 2.0).round() as i32;
        let y = (top + (h - FLYOUT_INPUT_PANEL_HEIGHT) / 2.0).round() as i32;
        SetWindowPos(
            hwnd,
            0,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        ShowWindow(hwnd, 5); // SW_SHOW = 5
    }
    SetForegroundWindow(hwnd);
}
