//! `ee-win` — Windows platform shell search overlay.

#![allow(unsafe_code)]
#![warn(missing_docs)]

#[cfg(target_os = "windows")]
mod dict;
#[cfg(target_os = "windows")]
mod focus;
#[cfg(target_os = "windows")]
mod logging;
#[cfg(target_os = "windows")]
mod overlay;
#[cfg(target_os = "windows")]
mod runner;
#[cfg(target_os = "windows")]
mod signals;
#[cfg(target_os = "windows")]
mod tray;
#[cfg(target_os = "windows")]
mod win32;

#[cfg(target_os = "windows")]
pub use runner::run;

/// Dummy run function for non-Windows platforms to keep cross-compilation green.
#[cfg(not(target_os = "windows"))]
pub fn run() -> Result<(), String> {
    Ok(())
}
