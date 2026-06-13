//! `ee-mac` — placeholder for the macOS platform shell.
//!
//! Phase 1 only locks down the module's existence. The tray icon, global
//! hotkey, frameless overlay window, and the dmg via cargo-packager packaging path
//! will land in a future iteration. See `App/Mac/.design.md`.
//!
//! Anything that won't compile on non-macOS hosts MUST be gated with
//! `#[cfg(target_os = "macos")]` so that `cargo build --workspace`
//! stays green on Windows / Linux developer machines.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Intentionally empty — Phase 2.
// When real code lands, structure it as:
//
//     #[cfg(target_os = "macos")]
//     mod imp;
//
//     #[cfg(target_os = "macos")]
//     pub use imp::*;
