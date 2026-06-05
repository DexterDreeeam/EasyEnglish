//! Public types shared across the dictionary module.

use serde::{Deserialize, Serialize};

/// One dictionary entry returned by `DictStore::lookup`.
///
/// The `headword` is always the seed's canonical lower-case form, never the
/// caller's input casing. The `definitions` vector is the natural display
/// order (insertion order from the seed) and is guaranteed non-empty when an
/// entry is returned from a successful lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Canonical lower-case English headword (e.g. `"apple"`).
    pub headword: String,
    /// IPA phonetic spelling. May be empty if the seed has none.
    pub phonetic: String,
    /// Chinese senses in display order. Non-empty for a returned entry.
    pub definitions: Vec<String>,
}

/// Errors surfaced by the dictionary module.
///
/// Marked `#[non_exhaustive]` so we can grow the error taxonomy without a
/// major-version bump. Callers should always wildcard-match on this type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DictError {
    /// The requested word does not exist in the dictionary.
    #[error("word not found")]
    NotFound,

    /// The caller-supplied word was empty, all whitespace, or longer than
    /// 128 bytes. These are validation failures rather than miss results.
    #[error("invalid input")]
    InvalidInput,

    /// Anything that came from `rusqlite` (file missing, schema error, etc.).
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    /// The seed JSON could not be read or parsed.
    #[error("seed error: {0}")]
    Seed(String),
}
