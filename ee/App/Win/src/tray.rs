//! System tray icon, global hotkey registration, and the Win32 message loop.

use crate::logging::log_message;
use crate::signals::{request_flyout_wakeup, EGUI_CTX, EXIT_REQUESTED};
use crate::startup;
use crate::win32::{show_flyout_window_now, wide_null};
use std::sync::atomic::Ordering;

// ---------------------------------------------------------------------------
// Win32 Background Low-Level Systems: System Tray & Global Hotkey
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
const WM_TRAYICON: u32 = 0x0400 + 1; // WM_USER + 1
#[cfg(target_os = "windows")]
const WM_SHOW_FLYOUT: u32 = 0x0400 + 2; // WM_USER + 2
#[cfg(target_os = "windows")]
const TRAY_WINDOW_CLASS: &str = "EasyEnglishTrayWndClass";
#[cfg(target_os = "windows")]
const TRAY_WINDOW_TITLE: &str = "EasyEnglishTrayWindow";
#[cfg(target_os = "windows")]
const ID_TRAY_SHOW: usize = 1001;
#[cfg(target_os = "windows")]
const ID_TRAY_STARTUP: usize = 1002;
#[cfg(target_os = "windows")]
const ID_TRAY_EXIT: usize = 1003;

#[cfg(target_os = "windows")]
unsafe fn show_flyout_from_tray() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, ShowWindow};

    if request_flyout_wakeup() {
        let title = "flyout\0".encode_utf16().collect::<Vec<u16>>();
        let flyout_hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if flyout_hwnd != 0 {
            ShowWindow(flyout_hwnd, 5); // SW_SHOW = 5
            crate::win32::focus_flyout_and_clear_alt(flyout_hwnd);
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn show_flyout_message() -> u32 {
    WM_SHOW_FLYOUT
}

/// Ask an already-running EasyEnglish tray instance to wake the flyout.
#[cfg(target_os = "windows")]
pub fn wake_existing_instance() -> Result<(), String> {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};

        let class_name = wide_null(TRAY_WINDOW_CLASS);
        let title = wide_null(TRAY_WINDOW_TITLE);
        let hwnd = FindWindowW(class_name.as_ptr(), title.as_ptr());
        if hwnd == 0 {
            return Err("existing tray window not found".to_string());
        }
        if PostMessageW(hwnd, show_flyout_message(), 0, 0) == 0 {
            return Err("failed to post wake message".to_string());
        }
    }
    Ok(())
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
        WM_SHOW_FLYOUT => {
            log_message("[Tray] Duplicate launch received → show flyout.");
            show_flyout_from_tray();
            0
        }
        WM_TRAYICON => {
            let tray_event = lparam as u32;
            if tray_event == WM_LBUTTONDBLCLK {
                log_message("[Tray] Double-click received → show flyout.");
                show_flyout_from_tray();
            } else if tray_event == WM_RBUTTONUP {
                let h_menu = CreatePopupMenu();

                let show_text = "Show Flyout\0".encode_utf16().collect::<Vec<u16>>();
                let startup_text = "Launch on Startup\0".encode_utf16().collect::<Vec<u16>>();
                let exit_text = "Exit\0".encode_utf16().collect::<Vec<u16>>();
                let startup_flags = if startup::launch_on_startup_enabled() {
                    MF_STRING | MF_CHECKED
                } else {
                    MF_STRING | MF_UNCHECKED
                };

                AppendMenuW(h_menu, MF_STRING, ID_TRAY_SHOW, show_text.as_ptr());
                AppendMenuW(
                    h_menu,
                    startup_flags,
                    ID_TRAY_STARTUP,
                    startup_text.as_ptr(),
                );
                AppendMenuW(h_menu, MF_SEPARATOR, 0, std::ptr::null());
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
                    show_flyout_from_tray();
                } else if cmd == ID_TRAY_STARTUP as i32 {
                    match startup::toggle_launch_on_startup() {
                        Ok(enabled) => {
                            log_message(&format!("[Startup] Launch on startup set to {}.", enabled))
                        }
                        Err(err) => log_message(&format!(
                            "[Startup] Failed to toggle launch on startup: {}",
                            err
                        )),
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
        WM_HOTKEY => {
            log_message("[WM_HOTKEY] Global hotkey Alt+~ received!");
            if !request_flyout_wakeup() {
                return 0;
            }

            show_flyout_window_now();
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn run_background_win32_system() -> Result<(), String> {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Shell::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let h_instance = GetModuleHandleW(std::ptr::null());

        let class_name = wide_null(TRAY_WINDOW_CLASS);
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

        let window_title = wide_null(TRAY_WINDOW_TITLE);
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

        if let Err(err) = startup::initialize_launch_on_startup_default() {
            log_message(&format!(
                "[Startup] Failed to initialize launch on startup: {}",
                err
            ));
        }

        // Register standard system-wide global hotkey Alt+~.
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            RegisterHotKey, UnregisterHotKey, MOD_ALT, VK_OEM_3,
        };
        let hotkey_id = 1;
        if RegisterHotKey(hwnd, hotkey_id, MOD_ALT, VK_OEM_3 as u32) == 0 {
            log_message("[RegisterHotKey] Failed to register global Alt+~ hotkey!");
        } else {
            log_message("[RegisterHotKey] Successfully registered global Alt+~ hotkey!");
        }

        let mut nid = std::mem::zeroed::<NOTIFYICONDATAW>();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        // Load the embedded application icon (resource ID 1, see build.rs).
        // Fall back to the stock application icon if it cannot be found.
        let app_icon = LoadIconW(h_instance, 1 as *const u16);
        nid.hIcon = if app_icon != 0 {
            app_icon
        } else {
            LoadIconW(0, IDI_APPLICATION)
        };

        let tooltip = "EasyEnglish\0".encode_utf16().collect::<Vec<u16>>();
        let len = std::cmp::min(tooltip.len(), nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&tooltip[..len]);

        if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
            return Err("Failed to create tray icon".to_string());
        }

        let mut msg = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut msg, 0, 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnregisterHotKey(hwnd, hotkey_id);
        Shell_NotifyIconW(NIM_DELETE, &nid);
        DestroyWindow(hwnd);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn run_background_win32_system() -> Result<(), String> {
    Ok(())
}
