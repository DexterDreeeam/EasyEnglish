//! `search` — Online dictionary and translation API search submodule.

use crate::RecordProvider;

/// Online search service provider.
pub struct Search {
    _opaque: (),
}

impl Search {
    /// Create a new Search service instance.
    ///
    /// # Panics
    ///
    /// This function is not yet implemented and always panics.
    pub fn new() -> Self {
        unimplemented!("Search is not yet implemented")
    }
}

impl RecordProvider for Search {
    fn get(&self, _key: &str) -> Option<String> {
        unimplemented!("Search get is not yet implemented")
    }
}
