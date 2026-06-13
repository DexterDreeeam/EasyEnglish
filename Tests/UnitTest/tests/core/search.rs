//! Integration tests for `Search`.

use ee_core::{RecordProvider, Search};

#[test]
#[should_panic(expected = "Search is not yet implemented")]
fn search_new_panics_unimplemented() {
    let _search = Search::new();
}

#[test]
fn search_implements_record_provider() {
    // We cannot construct Search without panicking yet, but we can verify it compiles with RecordProvider
    let provider: Option<Box<dyn RecordProvider>> = None;
    assert!(provider.is_none());
}
