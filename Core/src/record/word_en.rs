use serde::{Serialize, Deserialize};
use super::SerializableRecord;

/// Strongly-typed English word structure with optional attributes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WordEn {
    /// Canonical lowercase English word.
    pub word: String,
    /// US pronunciation node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation: Option<Pronunciation>,
    /// Meanings grouped by Part of Speech.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definitions: Option<Vec<Definition>>,
    /// Plurals or verb-tense variants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inflections: Option<Inflections>,
    /// Example sentences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<Example>>,
}

/// Backwards compatibility alias for easy migration.
pub type WordData = WordEn;

impl SerializableRecord for WordEn {
    fn serialize(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// US Pronunciation information (UK omitted based on design review).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pronunciation {
    /// US IPA phonetic spelling, e.g., "əˈplaɪ".
    pub ipa: String,
    /// Relative local path to cached MP3 audio file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    /// Download URL of CDN hosting pronunciation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<String>,
}

/// A specific Part of Speech meaning list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Definition {
    /// POS, e.g., "n.", "v.", "adj.".
    pub pos: String,
    /// List of meanings.
    pub meanings: Vec<String>,
}

/// Common English morphological variants (all elements optional).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Inflections {
    /// Plural form (nouns).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plural: Option<String>,
    /// Past tense (verbs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub past_tense: Option<String>,
    /// Past participle (verbs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub past_participle: Option<String>,
    /// Present participle (verbs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub present_participle: Option<String>,
    /// Third-person singular (verbs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub third_singular: Option<String>,
}

/// Parallel English-Chinese sentence examples.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Example {
    /// English sentence.
    pub en: String,
    /// Chinese translation.
    pub zh: String,
}
