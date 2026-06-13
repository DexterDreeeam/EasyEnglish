use serde::{Deserialize, Serialize};

/// A trait defining a strongly-typed model that can be serialized into a database string.
pub trait SerializableRecord {
    /// Serialize this record type into a JSON database string.
    fn serialize(&self) -> Result<String, serde_json::Error>;
}

/// The logical database entity categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordType {
    /// English Word.
    #[serde(rename = "word_en")]
    WordEn,
    /// Chinese Word.
    #[serde(rename = "word_cn")]
    WordCn,
    /// Custom User Note.
    #[serde(rename = "note")]
    Note,
    /// Timestamped History log.
    #[serde(rename = "history")]
    History,
}

/// A polymorphic wrapper containing the strongly-typed deserialized models.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "record_type", rename_all = "snake_case")]
pub enum RecordModel {
    /// Strongly-typed english word with rich metadata.
    WordEn(WordEn),
    /// Chinese headword mapping to its most frequent English equivalents.
    WordCn(WordCn),
    /// User note with plain text content.
    Note(Note),
    /// History record.
    History(History),
}

impl RecordModel {
    /// Retrieve the typed category of this record model.
    pub fn r#type(&self) -> RecordType {
        match self {
            RecordModel::WordEn(_) => RecordType::WordEn,
            RecordModel::WordCn(_) => RecordType::WordCn,
            RecordModel::Note(_) => RecordType::Note,
            RecordModel::History(_) => RecordType::History,
        }
    }
}

// Implement serializable on the polymorphic container itself
impl SerializableRecord for RecordModel {
    fn serialize(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

// Module declarations
mod history;
mod note;
mod word_cn;
mod word_en;

pub use history::History;
pub use note::Note;
pub use word_cn::WordCn;
pub use word_en::{Definition, Example, Inflections, Pronunciation, WordData, WordEn};

// Definition of Record physical model
/// A raw, low-level physical row containing a key and serialized value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Storage primary lookup key (e.g., "apple")
    pub key: String,
    /// Storage serialized raw value (JSON or plain-text)
    pub value: String,
}

impl Record {
    /// Create a new low-level record row.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Try to polymorphicly deserialize the raw value into standard strongly-typed models.
    pub fn deserialize(&self) -> Result<RecordModel, serde_json::Error> {
        serde_json::from_str(&self.value)
    }
}
