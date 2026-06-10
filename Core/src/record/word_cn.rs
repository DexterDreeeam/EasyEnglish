use super::SerializableRecord;
use serde::{Deserialize, Serialize};

/// Chinese headword record mapping a Chinese term to its most frequent English
/// equivalents. Used by the Chinese → English (`word_cn`) dictionary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WordCn {
    /// Canonical Chinese term (the lookup key).
    pub word: String,
    /// English equivalents, ordered most-frequent first (capped upstream at 10).
    pub english: Vec<String>,
}

impl WordCn {
    /// Create a new Chinese word record.
    pub fn new(word: impl Into<String>, english: Vec<String>) -> Self {
        Self {
            word: word.into(),
            english,
        }
    }
}

impl SerializableRecord for WordCn {
    fn serialize(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
