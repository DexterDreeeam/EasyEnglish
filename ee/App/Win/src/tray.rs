//! System tray icon, global hotkey registration, and the Win32 message loop.

use crate::logging::log_message;
use crate::signals::{request_flyout_wakeup, EGUI_CTX, EXIT_REQUESTED, FLYOUT_HWND};
use crate::startup;
use crate::win32::{find_flyout_window, show_flyout_window_now, wide_null};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Win32 Background Low-Level Systems: System Tray & Global Hotkey
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
const WM_TRAYICON: u32 = 0x0400 + 1; // WM_USER + 1
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
pub(crate) const EXIT_WATCHDOG_DELAY: Duration = Duration::from_millis(750);
#[cfg(target_os = "windows")]
static EXIT_WATCHDOG_STARTED: AtomicBool = AtomicBool::new(false);

/// Extract the tray command identifier from a Win32 `WM_COMMAND` `wparam`.
#[cfg(target_os = "windows")]
pub(crate) fn tray_command_id_from_wparam(wparam: usize) -> usize {
    wparam & 0xffff
}

#[cfg(target_os = "windows")]
fn resolve_flyout_hwnd() -> isize {
    let mut hwnd = FLYOUT_HWND.load(Ordering::SeqCst);
    if hwnd == 0 {
        hwnd = find_flyout_window();
        if hwnd != 0 {
            FLYOUT_HWND.store(hwnd, Ordering::SeqCst);
        }
    }
    hwnd
}

#[cfg(target_os = "windows")]
fn spawn_exit_watchdog() {
    if EXIT_WATCHDOG_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        std::thread::sleep(EXIT_WATCHDOG_DELAY);
        if EXIT_REQUESTED.load(Ordering::SeqCst) {
            std::process::exit(0);
        }
    });
}

#[cfg(target_os = "windows")]
unsafe fn request_process_exit() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

    EXIT_REQUESTED.store(true, Ordering::SeqCst);
    if let Some(ctx) = EGUI_CTX.lock().unwrap().as_ref() {
        ctx.request_repaint();
    }

    let flyout_hwnd = resolve_flyout_hwnd();
    if flyout_hwnd != 0 {
        PostMessageW(flyout_hwnd, WM_CLOSE, 0, 0);
    }
    spawn_exit_watchdog();
}

#[cfg(target_os = "windows")]
unsafe fn handle_tray_command(cmd: usize) {
    if cmd == ID_TRAY_SHOW {
        if request_flyout_wakeup() {
            show_flyout_window_now();
        }
    } else if cmd == ID_TRAY_STARTUP {
        match startup::toggle_launch_on_startup() {
            Ok(enabled) => {
                log_message(&format!("[Startup] Launch on startup set to {}.", enabled))
            }
            Err(err) => log_message(&format!(
                "[Startup] Failed to toggle launch on startup: {}",
                err
            )),
        }
    } else if cmd == ID_TRAY_EXIT {
        request_process_exit();
    }
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
        WM_TRAYICON => {
            if lparam as u32 == WM_RBUTTONUP {
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

                if cmd != 0 {
                    handle_tray_command(cmd as usize);
                }
                if cmd == ID_TRAY_EXIT as i32 {
                    PostQuitMessage(0);
                }
                DestroyMenu(h_menu);
            }
            0
        }
        WM_COMMAND => {
            let cmd = tray_command_id_from_wparam(wparam);
            handle_tray_command(cmd);
            if cmd == ID_TRAY_EXIT {
                PostQuitMessage(0);
            }
            0
        }
        WM_CLOSE => {
            request_process_exit();
            PostQuitMessage(0);
            0
        }
        WM_DESTROY => {
            request_process_exit();
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
