//! `RecordProvider` — A unified interface/trait for querying records.

/// A trait defining a read-only provider for retrieving values by key.
pub trait RecordProvider {
    /// Retrieve the value associated with `key`.
    /// Returns `None` if the key does not exist.
    fn get(&self, key: &str) -> Option<String>;
}
