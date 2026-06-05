//! `NoteStore` — runtime-only "English → arbitrary content" mapping.
//!
//! Notes are the user's personal annotations on a word; the content need not
//! be a translation. Storage is in-memory only in Phase 1 — the store starts
//! empty on every process launch. Persistence is intentionally deferred so
//! we can validate the access pattern before committing to a disk format.

use std::collections::HashMap;

/// A single note record: the word the user attached the content to (always
/// stored lower-cased) plus the free-form content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Lower-cased English word.
    pub word: String,
    /// Arbitrary user content (typically a custom translation or mnemonic).
    pub content: String,
}

/// Runtime map of `lowercase(word) -> Note`. Case-insensitive on every operation.
///
/// Notes are *not* persisted across runs in Phase 1.
#[derive(Debug, Default)]
pub struct NoteStore {
    notes: HashMap<String, Note>,
}

impl NoteStore {
    /// Construct an empty store. The store starts empty on every launch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or overwrite a note. The `word` is lower-cased before storage.
    /// Empty / whitespace-only words are rejected silently (no-op) to keep
    /// `set` ergonomic in tight UI loops.
    pub fn set(&mut self, word: &str, content: String) {
        let key = word.trim().to_lowercase();
        if key.is_empty() {
            return;
        }
        self.notes.insert(key.clone(), Note { word: key, content });
    }

    /// Remove a note. Returns `true` if it existed, `false` if it didn't
    /// (idempotent).
    pub fn remove(&mut self, word: &str) -> bool {
        let key = word.trim().to_lowercase();
        if key.is_empty() {
            return false;
        }
        self.notes.remove(&key).is_some()
    }

    /// Case-insensitive lookup.
    pub fn get(&self, word: &str) -> Option<&Note> {
        let key = word.trim().to_lowercase();
        if key.is_empty() {
            return None;
        }
        self.notes.get(&key)
    }

    /// All notes, sorted by `word` ascending. Allocates — call sparingly.
    pub fn list(&self) -> Vec<&Note> {
        let mut items: Vec<&Note> = self.notes.values().collect();
        items.sort_by(|a, b| a.word.cmp(&b.word));
        items
    }

    /// Number of notes currently stored.
    pub fn len(&self) -> usize {
        self.notes.len()
    }

    /// True if the store has no notes.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }
}
