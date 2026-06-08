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
