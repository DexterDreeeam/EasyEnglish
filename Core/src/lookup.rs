//! `LookupService` — compose a `NoteStore` query with a `DictStore` query.
//!
//! The lookup order is governed by `prefer_notes_over_dict` from `Config`
//! (default true): Note first, Dict on miss. A miss in both surfaces as
//! `LookupError::NotFound`.

use ee_dict::{DictError, DictStore, Entry};

use crate::notes::{Note, NoteStore};

/// Successful lookup, tagged by the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupHit {
    /// The user has attached a custom note to this word.
    Note(Note),
    /// The dictionary has a definition for this word.
    Dict(Entry),
}

impl LookupHit {
    /// The canonical lower-cased word for this hit.
    pub fn word(&self) -> &str {
        match self {
            LookupHit::Note(n) => &n.word,
            LookupHit::Dict(e) => &e.headword,
        }
    }
}

/// Errors surfaced by [`LookupService::query`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LookupError {
    /// Neither the user's notes nor the dictionary had an entry.
    #[error("word not found")]
    NotFound,
    /// The input was empty / whitespace-only / oversized (>128 bytes).
    #[error("invalid input")]
    InvalidInput,
    /// The dictionary backend errored; wrapped from [`ee_dict::DictError`].
    #[error("dictionary storage failure: {0}")]
    Storage(DictError),
}

/// Stateless query orchestrator. Holds the configuration toggles only —
/// the actual data lives in the `NoteStore` and `DictStore` passed at call time.
#[derive(Debug, Clone)]
pub struct LookupService {
    prefer_notes_over_dict: bool,
}

impl LookupService {
    /// Construct with the policy from `Config::lookup_prefer_notes_over_dict`.
    pub fn new(prefer_notes_over_dict: bool) -> Self {
        Self {
            prefer_notes_over_dict,
        }
    }

    /// Run the lookup pipeline on `word`. Trim / length validation happens
    /// before either store is consulted.
    pub fn query(
        &self,
        word: &str,
        notes: &NoteStore,
        dict: &DictStore,
    ) -> Result<LookupHit, LookupError> {
        let trimmed = word.trim();
        if trimmed.is_empty() || trimmed.len() > 128 {
            return Err(LookupError::InvalidInput);
        }

        if self.prefer_notes_over_dict {
            if let Some(note) = notes.get(trimmed) {
                return Ok(LookupHit::Note(note.clone()));
            }
            match dict.lookup(trimmed) {
                Ok(entry) => Ok(LookupHit::Dict(entry)),
                Err(DictError::NotFound) => Err(LookupError::NotFound),
                Err(DictError::InvalidInput) => Err(LookupError::InvalidInput),
                Err(other) => Err(LookupError::Storage(other)),
            }
        } else {
            match dict.lookup(trimmed) {
                Ok(entry) => Ok(LookupHit::Dict(entry)),
                Err(DictError::NotFound) => {
                    if let Some(note) = notes.get(trimmed) {
                        Ok(LookupHit::Note(note.clone()))
                    } else {
                        Err(LookupError::NotFound)
                    }
                }
                Err(DictError::InvalidInput) => Err(LookupError::InvalidInput),
                Err(other) => Err(LookupError::Storage(other)),
            }
        }
    }
}
