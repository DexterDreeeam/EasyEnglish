//! Current-user launch-on-startup preference and HKCU Run registration.

use crate::win32::wide_null;
use std::path::Path;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW, HKEY_CURRENT_USER,
    KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_DWORD,
};

const APP_NAME: &str = "EasyEnglish";
const PREF_SUBKEY: &str = "Software\\EasyEnglish";
const PREF_VALUE: &str = "LaunchOnStartup";
const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Return the effective launch-on-startup preference. Missing means enabled.
pub(crate) fn launch_on_startup_enabled() -> bool {
    read_launch_on_startup_preference()
        .ok()
        .flatten()
        .unwrap_or(true)
}

/// Ensure the default preference is materialized and the HKCU Run entry matches it.
pub(crate) fn initialize_launch_on_startup_default() -> Result<(), String> {
    let preference = read_launch_on_startup_preference()?;
    let enabled = preference.unwrap_or(true);
    if preference.is_none() {
        write_launch_on_startup_preference(true)?;
    }
    apply_launch_on_startup(enabled)
}

/// Set the launch-on-startup preference and update the HKCU Run entry.
pub(crate) fn set_launch_on_startup_enabled(enabled: bool) -> Result<(), String> {
    write_launch_on_startup_preference(enabled)?;
    apply_launch_on_startup(enabled)
}

/// Toggle launch-on-startup and return the new state.
pub(crate) fn toggle_launch_on_startup() -> Result<bool, String> {
    let enabled = !launch_on_startup_enabled();
    set_launch_on_startup_enabled(enabled)?;
    Ok(enabled)
}

/// Build the command written to HKCU Run for a given executable path.
pub(crate) fn launch_on_startup_run_value(exe_path: &Path) -> String {
    format!("\"{}\"", exe_path.display())
}

fn apply_launch_on_startup(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
        write_registry_string(RUN_SUBKEY, APP_NAME, &launch_on_startup_run_value(&exe))
    } else {
        delete_registry_value(RUN_SUBKEY, APP_NAME)
    }
}

fn read_launch_on_startup_preference() -> Result<Option<bool>, String> {
    read_registry_dword(PREF_SUBKEY, PREF_VALUE).map(|value| value.map(|v| v != 0))
}

fn write_launch_on_startup_preference(enabled: bool) -> Result<(), String> {
    write_registry_dword(PREF_SUBKEY, PREF_VALUE, u32::from(enabled))
}

fn read_registry_dword(subkey: &str, value_name: &str) -> Result<Option<u32>, String> {
    let subkey = wide_null(subkey);
    let value_name = wide_null(value_name);
    let mut value_type = 0u32;
    let mut data = 0u32;
    let mut data_len = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            &mut value_type,
            &mut data as *mut u32 as *mut _,
            &mut data_len,
        )
    };
    match status {
        ERROR_SUCCESS => Ok(Some(data)),
        ERROR_FILE_NOT_FOUND => Ok(None),
        other => Err(format!("RegGetValueW({value_name:?}) failed: {other}")),
    }
}

fn write_registry_dword(subkey: &str, value_name: &str, value: u32) -> Result<(), String> {
    with_created_key(subkey, |key| {
        let value_name = wide_null(value_name);
        let data = value.to_le_bytes();
        let status = unsafe {
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_DWORD,
                data.as_ptr(),
                data.len() as u32,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("RegSetValueExW({value_name:?}) failed: {status}"))
        }
    })
}

fn write_registry_string(subkey: &str, value_name: &str, value: &str) -> Result<(), String> {
    with_created_key(subkey, |key| {
        let value_name = wide_null(value_name);
        let data = wide_null(value);
        let status = unsafe {
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as *const u8,
                (data.len() * std::mem::size_of::<u16>()) as u32,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("RegSetValueExW({value_name:?}) failed: {status}"))
        }
    })
}

fn delete_registry_value(subkey: &str, value_name: &str) -> Result<(), String> {
    with_created_key(subkey, |key| {
        let value_name = wide_null(value_name);
        let status = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
        match status {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            other => Err(format!("RegDeleteValueW({value_name:?}) failed: {other}")),
        }
    })
}

fn with_created_key<T>(
    subkey: &str,
    action: impl FnOnce(windows_sys::Win32::System::Registry::HKEY) -> Result<T, String>,
) -> Result<T, String> {
    let subkey = wide_null(subkey);
    let mut key = 0isize;
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!("RegCreateKeyExW failed: {status}"));
    }
    let result = action(key);
    unsafe {
        RegCloseKey(key);
    }
    result
}
