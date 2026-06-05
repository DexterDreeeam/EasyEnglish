//! `ee-linux` — placeholder for the Linux platform shell.
//!
//! Phase 1 only locks down the module's existence. The tray icon, global
//! hotkey, frameless overlay window, and the AppImage + deb via cargo-packager packaging path
//! will land in a future iteration. See `App/Linux/.design.md`.
//!
//! Anything that won't compile on non-Linux hosts MUST be gated with
//! `#[cfg(target_os = "linux")]` so that `cargo build --workspace`
//! stays green on Windows / macOS developer machines.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Intentionally empty — Phase 2.
// When real code lands, structure it as:
//
//     #[cfg(target_os = "linux")]
//     mod imp;
//
//     #[cfg(target_os = "linux")]
//     pub use imp::*;
