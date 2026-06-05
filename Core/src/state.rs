//! `AppState` — UI-facing application model.
//!
//! The platform crates (`Win`/`Mac`/`Linux`) read `AppState`'s public getters
//! every frame and call its mutators in response to user input. AppState
//! itself performs no rendering and never touches OS APIs — it just composes
//! the `Config` / `NoteStore` / `HistoryStore` / `LookupService` / `DictStore`
//! fields it was constructed with.

use ee_dict::DictStore;

use crate::config::Config;
use crate::history::{HistoryEntry, HistoryStore};
use crate::lookup::{LookupError, LookupHit, LookupService};
use crate::notes::{Note, NoteStore};

/// Stateful binding of all the Core machinery to a single user session.
#[derive(Debug)]
pub struct AppState {
    config: Config,
    dict: DictStore,
    notes: NoteStore,
    history: HistoryStore,
    lookup: LookupService,

    input: String,
    last_hit: Option<LookupHit>,
    status: String,
}

impl AppState {
    /// Build a fresh AppState. The `dict` argument is taken by value because
    /// `AppState` owns the dictionary handle for the rest of its lifetime;
    /// the platform crate constructs the dict via `DictStore::create_or_seed`
    /// and hands it in.
    pub fn new(config: Config, dict: DictStore) -> Self {
        let history = HistoryStore::with_capacity(config.history_max_entries());
        let lookup = LookupService::new(config.lookup_prefer_notes_over_dict());
        Self {
            config,
            dict,
            notes: NoteStore::new(),
            history,
            lookup,
            input: String::new(),
            last_hit: None,
            status: String::new(),
        }
    }

    // ---- Buffer access (the platform UI binds an InputText to this) -------

    /// The user-entered input as last set by the UI.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Mutable handle to the input buffer. The UI typically calls
    /// `state.input_mut().clear(); state.input_mut().push_str(typed)`.
    pub fn input_mut(&mut self) -> &mut String {
        &mut self.input
    }

    // ---- Read-only state the UI binds to each frame -----------------------

    /// `Some(hit)` after a successful submit; `None` on miss / empty input.
    pub fn last_hit(&self) -> Option<&LookupHit> {
        self.last_hit.as_ref()
    }

    /// Human-readable status (e.g. `"Found"`, `"Not found: foo"`).
    pub fn status(&self) -> &str {
        &self.status
    }

    /// Recent successful lookups, most-recent first.
    pub fn recent(&self) -> Vec<HistoryEntry> {
        self.history.recent()
    }

    /// Borrowed view of the note store (for the UI's notes panel, future use).
    pub fn notes(&self) -> &NoteStore {
        &self.notes
    }

    /// Active `Config`. Public so platform crates can read shell preferences
    /// (hotkey, window size, ...) without a second parse.
    pub fn config(&self) -> &Config {
        &self.config
    }

    // ---- Mutators triggered by user input ---------------------------------

    /// Trim the input and run the lookup pipeline. No-op when input is empty.
    /// Successful hits are recorded to history.
    pub fn submit(&mut self) {
        let trimmed = self.input.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        match self.lookup.query(&trimmed, &self.notes, &self.dict) {
            Ok(hit) => {
                self.history.record(hit.word());
                self.status = "Found".to_string();
                self.last_hit = Some(hit);
            }
            Err(LookupError::NotFound) => {
                self.last_hit = None;
                self.status = format!("Not found: {trimmed}");
            }
            Err(LookupError::InvalidInput) => {
                self.last_hit = None;
                self.status = "Invalid input".to_string();
            }
            Err(LookupError::Storage(err)) => {
                self.last_hit = None;
                self.status = format!("Storage error: {err}");
            }
        }
    }

    /// Clear input + last hit + status. Called by the platform shell when the
    /// overlay is dismissed.
    pub fn reset(&mut self) {
        self.input.clear();
        self.last_hit = None;
        self.status.clear();
    }

    /// Add or overwrite a user-defined note. Word is lower-cased before storage.
    pub fn add_note(&mut self, word: &str, content: String) {
        self.notes.set(word, content);
    }

    /// Remove a user-defined note. Returns `true` if a note existed.
    pub fn remove_note(&mut self, word: &str) -> bool {
        self.notes.remove(word)
    }

    /// Direct access to the user's note for `word`, if any.
    pub fn get_note(&self, word: &str) -> Option<&Note> {
        self.notes.get(word)
    }
}
