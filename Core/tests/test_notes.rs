//! Integration tests for `NoteStore` — see `Core/tests/.test.md`.

use ee_core::NoteStore;

#[test]
fn new_is_empty() {
    let s = NoteStore::new();
    assert_eq!(s.len(), 0);
    assert!(s.is_empty());
    assert!(s.list().is_empty());
}

#[test]
fn set_then_get_returns_note() {
    let mut s = NoteStore::new();
    s.set("apple", "fruit".to_string());
    let n = s.get("apple").expect("hit");
    assert_eq!(n.word, "apple");
    assert_eq!(n.content, "fruit");
    assert_eq!(s.len(), 1);
}

#[test]
fn set_lowercases_word() {
    let mut s = NoteStore::new();
    s.set("Apple", "fruit".to_string());
    assert!(s.get("APPLE").is_some());
    assert!(s.get("apple").is_some());
    let n = s.get("aPpLe").expect("hit");
    assert_eq!(n.word, "apple");
}

#[test]
fn set_overwrites_existing() {
    let mut s = NoteStore::new();
    s.set("apple", "first".to_string());
    s.set("APPLE", "second".to_string());
    assert_eq!(s.len(), 1);
    assert_eq!(s.get("apple").unwrap().content, "second");
}

#[test]
fn set_empty_is_noop() {
    let mut s = NoteStore::new();
    s.set("", "content".to_string());
    s.set("   ", "content".to_string());
    s.set("\t\n", "content".to_string());
    assert!(s.is_empty());
}

#[test]
fn remove_existing_returns_true() {
    let mut s = NoteStore::new();
    s.set("apple", "fruit".to_string());
    assert!(s.remove("APPLE"));
    assert!(s.is_empty());
}

#[test]
fn remove_missing_returns_false() {
    let mut s = NoteStore::new();
    assert!(!s.remove("ghost"));
}

#[test]
fn remove_is_case_insensitive() {
    let mut s = NoteStore::new();
    s.set("apple", "fruit".to_string());
    assert!(s.remove("ApPlE"));
}

#[test]
fn list_is_sorted_by_word_ascending() {
    let mut s = NoteStore::new();
    s.set("banana", "黄".to_string());
    s.set("apple", "红".to_string());
    s.set("cherry", "深红".to_string());
    let words: Vec<&str> = s.list().into_iter().map(|n| n.word.as_str()).collect();
    assert_eq!(words, vec!["apple", "banana", "cherry"]);
}
