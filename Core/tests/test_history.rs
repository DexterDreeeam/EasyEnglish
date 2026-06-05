//! Integration tests for `HistoryStore` — see `Core/tests/.test.md`.

use ee_core::HistoryStore;

// Use a deterministic monotonically-increasing clock so order assertions are stable.
fn fake_clock() -> i64 {
    use std::sync::atomic::{AtomicI64, Ordering};
    static T: AtomicI64 = AtomicI64::new(1_000_000);
    T.fetch_add(1, Ordering::SeqCst)
}

#[test]
fn cap_zero_record_is_noop() {
    let mut h = HistoryStore::with_capacity(0);
    h.record("apple");
    assert!(h.is_empty());
}

#[test]
fn record_single() {
    let mut h = HistoryStore::with_clock(10, fake_clock);
    h.record("apple");
    assert_eq!(h.len(), 1);
    assert_eq!(h.recent().first().map(|e| e.word.as_str()), Some("apple"));
}

#[test]
fn record_dedups_existing_word_and_moves_to_front() {
    let mut h = HistoryStore::with_clock(10, fake_clock);
    h.record("apple");
    h.record("banana");
    h.record("apple");
    let words: Vec<String> = h.recent().into_iter().map(|e| e.word).collect();
    assert_eq!(words, vec!["apple".to_string(), "banana".to_string()]);
    assert_eq!(h.len(), 2);
}

#[test]
fn record_evicts_oldest_when_over_cap() {
    let mut h = HistoryStore::with_clock(2, fake_clock);
    h.record("a");
    h.record("b");
    h.record("c");
    let words: Vec<String> = h.recent().into_iter().map(|e| e.word).collect();
    assert_eq!(words, vec!["c".to_string(), "b".to_string()]);
}

#[test]
fn record_empty_input_is_noop() {
    let mut h = HistoryStore::with_clock(10, fake_clock);
    h.record("");
    h.record("   ");
    assert!(h.is_empty());
}

#[test]
fn clear_empties_the_store() {
    let mut h = HistoryStore::with_clock(10, fake_clock);
    h.record("apple");
    h.record("banana");
    h.clear();
    assert!(h.is_empty());
}
