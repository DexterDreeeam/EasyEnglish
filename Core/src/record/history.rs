use serde::{Serialize, Deserialize};
use super::SerializableRecord;

/// Bounded query history log entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct History {
    /// History content, e.g. a Unix timestamp string.
    pub content: String,
}

impl History {
    /// Create a new History instance.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

impl SerializableRecord for History {
    fn serialize(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.content)
    }
}
