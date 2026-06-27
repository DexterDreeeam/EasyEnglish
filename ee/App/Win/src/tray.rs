//! System tray icon, global hotkey registration, and the Win32 message loop.

use crate::logging::log_message;
use crate::signals::{request_flyout_wakeup, EGUI_CTX, EXIT_REQUESTED, FLYOUT_HWND};
use crate::startup;
use crate::win32::{find_flyout_window, show_flyout_window_now, wide_null};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Win32 Background Low-Level Systems: System Tray & Global Hotkey
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
const WM_TRAYICON: u32 = 0x0400 + 1; // WM_USER + 1
#[cfg(target_os = "windows")]
const WM_HOTKEY_FALLBACK_DIAGNOSTIC: u32 = 0x0400 + 2; // WM_USER + 2
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
const HOTKEY_ID: i32 = 1;
#[cfg(target_os = "windows")]
const HOTKEY_DEBOUNCE_MS: u64 = 150;
#[cfg(target_os = "windows")]
const BACKTICK_SCAN_CODE: u32 = 0x29;
#[cfg(target_os = "windows")]
pub(crate) const EXIT_WATCHDOG_DELAY: Duration = Duration::from_millis(750);
#[cfg(target_os = "windows")]
static EXIT_WATCHDOG_STARTED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static HOTKEY_FALLBACK_HWND: AtomicIsize = AtomicIsize::new(0);
#[cfg(target_os = "windows")]
static HOTKEY_FALLBACK_ALT_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static HOTKEY_FALLBACK_OEM3_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static LAST_HOTKEY_WAKE_MS: AtomicU64 = AtomicU64::new(0);

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
fn tick_ms() -> u64 {
    unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() }
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
            Ok(enabled) => log_message(&format!("[Startup] Launch on startup set to {}.", enabled)),
            Err(err) => log_message(&format!(
                "[Startup] Failed to toggle launch on startup: {}",
                err
            )),
        }
    } else if cmd == ID_TRAY_EXIT {
        request_process_exit();
    }
}

/// Request a hotkey-triggered flyout wake without reading selected text.
///
/// Clipboard-based selected-text capture is intentionally disabled for now: the
/// hotkey message path should stay fast and should not depend on clipboard
/// ownership, clipboard size, or the foreground application's Ctrl+C behavior.
#[cfg(target_os = "windows")]
pub(crate) fn request_hotkey_flyout_wakeup() -> bool {
    let now = tick_ms();
    let previous = LAST_HOTKEY_WAKE_MS.swap(now, Ordering::SeqCst);
    if previous != 0 && now.saturating_sub(previous) < HOTKEY_DEBOUNCE_MS {
        log_message("[WM_HOTKEY] Duplicate hotkey wake ignored.");
        return false;
    }

    log_message("[WM_HOTKEY] Global hotkey Alt+~ received.");
    log_message("[Selection] Clipboard selection capture is disabled for hotkey wake.");
    request_flyout_wakeup()
}

#[cfg(target_os = "windows")]
unsafe fn handle_hotkey_message() {
    if request_hotkey_flyout_wakeup() {
        show_flyout_window_now();
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn hotkey_fallback_keyboard_proc(
    n_code: i32,
    wparam: usize,
    lparam: isize,
) -> isize {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_LMENU, VK_MENU, VK_OEM_3, VK_RMENU,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, PostMessageW, KBDLLHOOKSTRUCT, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    if n_code >= 0 {
        let event = wparam as u32;
        let keyboard = &*(lparam as *const KBDLLHOOKSTRUCT);
        let is_key_down = event == WM_KEYDOWN || event == WM_SYSKEYDOWN;
        let is_key_up = event == WM_KEYUP || event == WM_SYSKEYUP;
        let is_alt_key = keyboard.vkCode == VK_MENU as u32
            || keyboard.vkCode == VK_LMENU as u32
            || keyboard.vkCode == VK_RMENU as u32;

        if is_alt_key {
            HOTKEY_FALLBACK_ALT_DOWN.store(is_key_down && !is_key_up, Ordering::SeqCst);
        }

        let async_alt_down = (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000) != 0;
        let alt_down = HOTKEY_FALLBACK_ALT_DOWN.load(Ordering::SeqCst) || async_alt_down;
        let is_backtick_key =
            keyboard.vkCode == VK_OEM_3 as u32 || keyboard.scanCode == BACKTICK_SCAN_CODE;

        if is_key_down && alt_down {
            let hwnd = HOTKEY_FALLBACK_HWND.load(Ordering::SeqCst);
            if hwnd != 0 {
                let diagnostic_lparam =
                    keyboard.scanCode as isize | ((keyboard.flags as isize) << 16);
                PostMessageW(
                    hwnd,
                    WM_HOTKEY_FALLBACK_DIAGNOSTIC,
                    keyboard.vkCode as usize,
                    diagnostic_lparam,
                );
            }
        }

        if is_backtick_key {
            if event == WM_KEYUP || event == WM_SYSKEYUP {
                HOTKEY_FALLBACK_OEM3_DOWN.store(false, Ordering::SeqCst);
            } else if event == WM_KEYDOWN || event == WM_SYSKEYDOWN {
                if alt_down && !HOTKEY_FALLBACK_OEM3_DOWN.swap(true, Ordering::SeqCst) {
                    let hwnd = HOTKEY_FALLBACK_HWND.load(Ordering::SeqCst);
                    if hwnd != 0 {
                        let modifiers = windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_ALT;
                        let lparam = ((VK_OEM_3 as isize) << 16) | modifiers as isize;
                        PostMessageW(hwnd, WM_HOTKEY, HOTKEY_ID as usize, lparam);
                    }
                }
            }
        }
    }

    CallNextHookEx(0, n_code, wparam, lparam)
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
        WM_HOTKEY_FALLBACK_DIAGNOSTIC => {
            let scan_code = (lparam as u32) & 0xffff;
            let flags = ((lparam as u32) >> 16) & 0xffff;
            log_message(&format!(
                "[HotkeyFallback] Alt keydown vk=0x{:x} scan=0x{:x} flags=0x{:x}.",
                wparam, scan_code, flags
            ));
            0
        }
        WM_HOTKEY => {
            // Clipboard selected-text capture is temporarily disabled. Keep the
            // hotkey path equivalent to tray Show Flyout for stability.
            handle_hotkey_message();
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn run_background_win32_system() -> Result<(), String> {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::MOD_NOREPEAT;
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
        HOTKEY_FALLBACK_HWND.store(hwnd, Ordering::SeqCst);

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
        if RegisterHotKey(hwnd, HOTKEY_ID, MOD_ALT | MOD_NOREPEAT, VK_OEM_3 as u32) == 0 {
            log_message("[RegisterHotKey] Failed to register global Alt+~ hotkey!");
        } else {
            log_message("[RegisterHotKey] Successfully registered global Alt+~ hotkey!");
        }

        let keyboard_hook =
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(hotkey_fallback_keyboard_proc), 0, 0);
        if keyboard_hook == 0 {
            log_message("[HotkeyFallback] Failed to install low-level Alt+~ fallback hook.");
        } else {
            log_message("[HotkeyFallback] Low-level Alt+~ fallback hook installed.");
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
            if msg.message == WM_HOTKEY {
                handle_hotkey_message();
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if keyboard_hook != 0 {
            UnhookWindowsHookEx(keyboard_hook);
        }
        HOTKEY_FALLBACK_HWND.store(0, Ordering::SeqCst);
        UnregisterHotKey(hwnd, HOTKEY_ID);
        Shell_NotifyIconW(NIM_DELETE, &nid);
        DestroyWindow(hwnd);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn run_background_win32_system() -> Result<(), String> {
    Ok(())
}
