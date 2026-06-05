//! Integration tests for `AppState` — see `Core/tests/.test.md`.

use std::fs;

use ee_core::{AppState, Config, LookupHit};
use ee_dict::DictStore;
use tempfile::TempDir;

const TINY_SEED: &str = r#"[
    {"headword":"apple","phonetic":"/ˈæp.əl/","definitions":["苹果"]},
    {"headword":"book","phonetic":"/bʊk/","definitions":["书"]}
]"#;

/// Build (`TempDir`, fresh AppState) seeded with two entries. The TempDir
/// must outlive the AppState — the dict file lives inside it.
fn fresh_state() -> (TempDir, AppState) {
    let dir = TempDir::new().unwrap();
    let seed = dir.path().join("seed.json");
    let db = dir.path().join("dict.sqlite3");
    fs::write(&seed, TINY_SEED).unwrap();
    let dict = DictStore::create_or_seed(&db, &seed).unwrap();
    (dir, AppState::new(Config::defaults(), dict))
}

#[test]
fn default_state_is_empty() {
    let (_d, state) = fresh_state();
    assert!(state.input().is_empty());
    assert!(state.last_hit().is_none());
    assert!(state.status().is_empty());
    assert!(state.recent().is_empty());
    assert!(state.notes().is_empty());
}

#[test]
fn submit_empty_input_is_noop() {
    let (_d, mut state) = fresh_state();
    state.submit();
    assert!(state.status().is_empty());
    assert!(state.last_hit().is_none());
}

#[test]
fn submit_hit_records_to_history_and_sets_status_found() {
    let (_d, mut state) = fresh_state();
    state.input_mut().push_str("apple");
    state.submit();
    assert!(matches!(state.last_hit(), Some(LookupHit::Dict(_))));
    assert_eq!(state.status(), "Found");
    let recent = state.recent();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].word, "apple");
}

#[test]
fn submit_miss_clears_last_hit_with_not_found_status() {
    let (_d, mut state) = fresh_state();
    state.input_mut().push_str("nosuch");
    state.submit();
    assert!(state.last_hit().is_none());
    assert!(state.status().contains("Not found"));
    assert!(state.recent().is_empty());
}

#[test]
fn reset_clears_buffer_and_hit() {
    let (_d, mut state) = fresh_state();
    state.input_mut().push_str("apple");
    state.submit();
    state.reset();
    assert!(state.input().is_empty());
    assert!(state.last_hit().is_none());
    assert!(state.status().is_empty());
}

#[test]
fn add_note_then_submit_returns_note_hit() {
    let (_d, mut state) = fresh_state();
    state.add_note("apple", "user-defined".to_string());
    state.input_mut().push_str("apple");
    state.submit();
    match state.last_hit() {
        Some(LookupHit::Note(n)) => assert_eq!(n.content, "user-defined"),
        other => panic!("expected Note hit, got {other:?}"),
    }
}

#[test]
fn remove_note_then_submit_returns_dict_hit_again() {
    let (_d, mut state) = fresh_state();
    state.add_note("apple", "user".to_string());
    assert!(state.remove_note("APPLE"));
    state.input_mut().push_str("apple");
    state.submit();
    assert!(matches!(state.last_hit(), Some(LookupHit::Dict(_))));
}

#[test]
fn get_note_round_trip() {
    let (_d, mut state) = fresh_state();
    state.add_note("apple", "fruit".to_string());
    assert_eq!(
        state.get_note("apple").map(|n| n.content.as_str()),
        Some("fruit")
    );
    assert!(state.get_note("missing").is_none());
}
