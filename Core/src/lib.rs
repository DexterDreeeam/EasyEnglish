//! `ee-core` — configuration, lookup orchestration, history, notes, AppState.
//!
//! See `Core/.design.md` for the design and `Core/.interface.md` for the API
//! contract.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod history;
mod lookup;
mod notes;
mod state;

pub use config::{Config, ConfigError};
pub use history::{HistoryEntry, HistoryStore};
pub use lookup::{LookupError, LookupHit, LookupService};
pub use notes::{Note, NoteStore};
pub use state::AppState;
