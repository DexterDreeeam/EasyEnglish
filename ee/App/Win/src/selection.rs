//! Selected-text capture through temporary Ctrl+C clipboard extraction.

use crate::logging::log_message;
use std::time::{Duration, Instant};

const SELECTED_TEXT_CHAR_LIMIT: usize = 512;
const CLIPBOARD_OPEN_RETRIES: usize = 12;
const CLIPBOARD_OPEN_DELAY: Duration = Duration::from_millis(10);
const COPY_POLL_TIMEOUT: Duration = Duration::from_millis(450);
const COPY_POLL_INTERVAL: Duration = Duration::from_millis(30);
const CF_TEXT: u32 = 1;
const CF_METAFILEPICT: u32 = 3;
const CF_SYLK: u32 = 4;
const CF_DIF: u32 = 5;
const CF_TIFF: u32 = 6;
const CF_OEMTEXT: u32 = 7;
const CF_DIB: u32 = 8;
const CF_PENDATA: u32 = 10;
const CF_RIFF: u32 = 11;
const CF_WAVE: u32 = 12;
const CF_UNICODETEXT_FORMAT: u32 = 13;
const CF_HDROP: u32 = 15;
const CF_LOCALE: u32 = 16;
const CF_DIBV5: u32 = 17;
const REGISTERED_CLIPBOARD_FORMAT_FIRST: u32 = 0xC000;
const REGISTERED_CLIPBOARD_FORMAT_LAST: u32 = 0xFFFF;

/// Read selected text by temporarily copying the source app's current selection.
///
/// This follows the common Windows dictionary pattern: save the current
/// clipboard, clear it, synthesize Ctrl+C, poll for copied text, then restore the
/// original clipboard data. If the source app does not copy text within the
/// timeout, the flyout falls back to normal empty-input wake.
pub(crate) fn read_selected_text() -> Option<String> {
    match read_selected_text_result() {
        Ok(text) => text,
        Err(err) => {
            log_message(&format!("[Selection] Failed to copy selected text: {err}"));
            None
        }
    }
}

/// Normalize selected text for the single-line flyout input.
pub(crate) fn normalize_selected_text(raw: &str) -> Option<String> {
    normalize_selected_text_with_limit(raw, SELECTED_TEXT_CHAR_LIMIT)
}

/// Normalize selected text with an explicit character limit.
pub(crate) fn normalize_selected_text_with_limit(raw: &str, limit: usize) -> Option<String> {
    if limit == 0 {
        return None;
    }

    let mut normalized = String::new();
    let mut chars_written = 0usize;
    for word in raw.split_whitespace() {
        if !normalized.is_empty() {
            if chars_written + 1 >= limit {
                break;
            }
            normalized.push(' ');
            chars_written += 1;
        }

        for ch in word.chars() {
            if chars_written == limit {
                break;
            }
            normalized.push(ch);
            chars_written += 1;
        }

        if chars_written == limit {
            break;
        }
    }

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Return whether clipboard data for `format` is documented as an HGLOBAL.
pub(crate) fn clipboard_format_uses_global_memory(format: u32) -> bool {
    matches!(
        format,
        CF_TEXT
            | CF_METAFILEPICT
            | CF_SYLK
            | CF_DIF
            | CF_TIFF
            | CF_OEMTEXT
            | CF_DIB
            | CF_PENDATA
            | CF_RIFF
            | CF_WAVE
            | CF_UNICODETEXT_FORMAT
            | CF_HDROP
            | CF_LOCALE
            | CF_DIBV5
    ) || (REGISTERED_CLIPBOARD_FORMAT_FIRST..=REGISTERED_CLIPBOARD_FORMAT_LAST).contains(&format)
}

fn read_selected_text_result() -> Result<Option<String>, String> {
    release_modifier_keys();
    wait_for_modifiers_released(Duration::from_millis(180));

    let backup = {
        let _clipboard = ClipboardLock::open()?;
        ClipboardBackup::capture()
    };

    {
        let _clipboard = ClipboardLock::open()?;
        unsafe {
            use windows_sys::Win32::System::DataExchange::EmptyClipboard;
            if EmptyClipboard() == 0 {
                return Err("EmptyClipboard failed".to_string());
            }
        }
    }

    send_ctrl_c()?;
    let copied = poll_for_copied_text();
    backup.restore()?;
    Ok(copied.and_then(|text| normalize_selected_text(&text)))
}

fn wait_for_modifiers_released(timeout: Duration) {
    let started = Instant::now();
    while modifiers_are_down() && started.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn modifiers_are_down() -> bool {
    unsafe {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        };
        [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
            .iter()
            .any(|key| (GetAsyncKeyState(*key as i32) as u16 & 0x8000) != 0)
    }
}

fn poll_for_copied_text() -> Option<String> {
    let started = Instant::now();
    while started.elapsed() < COPY_POLL_TIMEOUT {
        if let Ok(_clipboard) = ClipboardLock::open() {
            if let Ok(Some(text)) = read_unicode_clipboard_text() {
                if normalize_selected_text(&text).is_some() {
                    return Some(text);
                }
            }
        }
        std::thread::sleep(COPY_POLL_INTERVAL);
    }
    None
}

fn read_unicode_clipboard_text() -> Result<Option<String>, String> {
    unsafe {
        use windows_sys::Win32::System::DataExchange::{
            GetClipboardData, IsClipboardFormatAvailable,
        };
        use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
        use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

        if IsClipboardFormatAvailable(CF_UNICODETEXT as u32) == 0 {
            return Ok(None);
        }

        let handle = GetClipboardData(CF_UNICODETEXT as u32);
        if handle == 0 {
            return Ok(None);
        }

        let size_bytes = GlobalSize(handle as _);
        if size_bytes < 2 {
            return Ok(None);
        }

        let ptr = GlobalLock(handle as _);
        if ptr.is_null() {
            return Err("GlobalLock failed for clipboard text".to_string());
        }

        let wide_len = size_bytes / std::mem::size_of::<u16>();
        let wide = std::slice::from_raw_parts(ptr as *const u16, wide_len);
        let nul_pos = wide.iter().position(|ch| *ch == 0).unwrap_or(wide.len());
        let text = String::from_utf16(&wide[..nul_pos])
            .map_err(|_| "Clipboard text is not valid UTF-16".to_string())?;
        let _ = GlobalUnlock(handle as _);
        Ok(Some(text))
    }
}

struct ClipboardBackup {
    items: Vec<ClipboardItem>,
}

struct ClipboardItem {
    format: u32,
    bytes: Vec<u8>,
}

impl ClipboardBackup {
    fn capture() -> Self {
        unsafe {
            use windows_sys::Win32::System::DataExchange::{
                EnumClipboardFormats, GetClipboardData,
            };
            use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

            let mut items = Vec::new();
            let mut format = 0u32;
            loop {
                format = EnumClipboardFormats(format);
                if format == 0 {
                    break;
                }
                if !clipboard_format_uses_global_memory(format) {
                    continue;
                }

                let handle = GetClipboardData(format);
                if handle == 0 {
                    continue;
                }

                let size = GlobalSize(handle as _);
                if size == 0 {
                    continue;
                }

                let ptr = GlobalLock(handle as _);
                if ptr.is_null() {
                    continue;
                }

                let bytes = std::slice::from_raw_parts(ptr as *const u8, size).to_vec();
                let _ = GlobalUnlock(handle as _);
                items.push(ClipboardItem { format, bytes });
            }

            Self { items }
        }
    }

    fn restore(self) -> Result<(), String> {
        let _clipboard = ClipboardLock::open()?;
        unsafe {
            use windows_sys::Win32::System::DataExchange::{EmptyClipboard, SetClipboardData};

            if EmptyClipboard() == 0 {
                return Err("EmptyClipboard failed while restoring".to_string());
            }

            for item in self.items {
                let handle = allocate_global_copy(&item.bytes)?;
                if SetClipboardData(item.format, handle as isize) == 0 {
                    unsafe_free_global(handle);
                    return Err(format!(
                        "SetClipboardData failed while restoring format {}",
                        item.format
                    ));
                }
            }
        }
        Ok(())
    }
}

fn allocate_global_copy(bytes: &[u8]) -> Result<*mut std::ffi::c_void, String> {
    unsafe {
        use windows_sys::Win32::System::Memory::{
            GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
        };

        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if handle.is_null() {
            return Err("GlobalAlloc failed".to_string());
        }

        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            unsafe_free_global(handle);
            return Err("GlobalLock failed".to_string());
        }

        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
        let _ = GlobalUnlock(handle);
        Ok(handle)
    }
}

fn unsafe_free_global(handle: *mut std::ffi::c_void) {
    unsafe {
        extern "system" {
            fn GlobalFree(hmem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        }

        let _ = GlobalFree(handle);
    }
}

fn send_ctrl_c() -> Result<(), String> {
    unsafe {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
        };

        const VK_C: u16 = b'C' as u16;
        let inputs = [
            key_input(VK_CONTROL, 0),
            key_input(VK_C, 0),
            key_input(VK_C, KEYEVENTF_KEYUP),
            key_input(VK_CONTROL, KEYEVENTF_KEYUP),
        ];
        let sent = SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
        if sent != inputs.len() as u32 {
            return Err(format!("SendInput sent {sent}/{} events", inputs.len()));
        }

        fn key_input(vk: u16, flags: u32) -> INPUT {
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk,
                        wScan: 0,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }
        }
    }
    Ok(())
}

struct ClipboardLock;

impl ClipboardLock {
    fn open() -> Result<Self, String> {
        unsafe {
            use windows_sys::Win32::System::DataExchange::OpenClipboard;
            for _ in 0..CLIPBOARD_OPEN_RETRIES {
                if OpenClipboard(0) != 0 {
                    return Ok(Self);
                }
                std::thread::sleep(CLIPBOARD_OPEN_DELAY);
            }
        }
        Err("OpenClipboard failed".to_string())
    }
}

impl Drop for ClipboardLock {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::DataExchange::CloseClipboard();
        }
    }
}

fn release_modifier_keys() {
    unsafe {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            keybd_event, KEYEVENTF_KEYUP, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
            VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
        };
        for key in [
            VK_CONTROL,
            VK_LCONTROL,
            VK_RCONTROL,
            VK_SHIFT,
            VK_LSHIFT,
            VK_RSHIFT,
            VK_MENU,
            VK_LMENU,
            VK_RMENU,
            VK_LWIN,
            VK_RWIN,
        ] {
            keybd_event(key as u8, 0, KEYEVENTF_KEYUP, 0);
        }
    }
}
