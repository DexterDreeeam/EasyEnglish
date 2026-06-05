use serde::{Serialize, Deserialize};
use super::SerializableRecord;

/// User-defined note record containing arbitrary custom annotation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Note {
    /// Note plain text content.
    pub content: String,
}

impl Note {
    /// Create a new Note instance.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl SerializableRecord for Note {
    fn serialize(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.content)
    }
}
