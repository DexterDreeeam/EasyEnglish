//! `ee-win` — Windows platform shell search overlay.

#![allow(unsafe_code)]
#![warn(missing_docs)]

#[cfg(target_os = "windows")]
mod imp;

#[cfg(target_os = "windows")]
pub use imp::run;

/// Dummy run function for non-Windows platforms to keep cross-compilation green.
#[cfg(not(target_os = "windows"))]
pub fn run() -> Result<(), String> {
    Ok(())
}
