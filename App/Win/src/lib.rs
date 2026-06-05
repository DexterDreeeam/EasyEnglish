//! `ee-win` — placeholder for the Windows platform shell.
//!
//! Phase 1 only locks down the module's existence. The tray icon, global
//! hotkey, frameless overlay window, and the WiX MSI packaging path
//! will land in a future iteration. See `App/Win/.design.md`.
//!
//! Anything that won't compile on non-Windows hosts MUST be gated with
//! `#[cfg(target_os = "windows")]` so that `cargo build --workspace`
//! stays green on macOS / Linux developer machines.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Intentionally empty — Phase 2.
// When real code lands, structure it as:
//
//     #[cfg(target_os = "windows")]
//     mod imp;
//
//     #[cfg(target_os = "windows")]
//     pub use imp::*;
