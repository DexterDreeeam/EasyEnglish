//! Integration tests for `LookupService` — see `Core/tests/.test.md`.

use std::fs;

use ee_core::{LookupError, LookupHit, LookupService, NoteStore};
use ee_dict::DictStore;
use tempfile::TempDir;

const TINY_SEED: &str = r#"[
    {"headword":"apple","phonetic":"/ˈæp.əl/","definitions":["苹果"]},
    {"headword":"apply","phonetic":"/əˈplaɪ/","definitions":["申请"]},
    {"headword":"book","phonetic":"/bʊk/","definitions":["书"]}
]"#;

fn mini_dict() -> (TempDir, DictStore) {
    let dir = TempDir::new().unwrap();
    let seed = dir.path().join("seed.json");
    let db = dir.path().join("dict.sqlite3");
    fs::write(&seed, TINY_SEED).unwrap();
    let dict = DictStore::create_or_seed(&db, &seed).unwrap();
    (dir, dict)
}

#[test]
fn note_first_hit_returns_note_without_touching_dict() {
    let (_dir, dict) = mini_dict();
    let mut notes = NoteStore::new();
    notes.set("apple", "custom translation".to_string());
    let svc = LookupService::new(/*prefer_notes_over_dict=*/ true);

    let hit = svc.query("apple", &notes, &dict).expect("hit");
    match hit {
        LookupHit::Note(n) => {
            assert_eq!(n.word, "apple");
            assert_eq!(n.content, "custom translation");
        }
        LookupHit::Dict(_) => panic!("expected Note hit, got Dict (note must win)"),
    }
}

#[test]
fn note_first_falls_back_to_dict_when_no_note() {
    let (_dir, dict) = mini_dict();
    let notes = NoteStore::new();
    let svc = LookupService::new(true);

    let hit = svc.query("apply", &notes, &dict).expect("hit");
    assert!(matches!(hit, LookupHit::Dict(_)));
}

#[test]
fn dict_first_hit_returns_dict_even_when_note_exists() {
    let (_dir, dict) = mini_dict();
    let mut notes = NoteStore::new();
    notes.set("apple", "should be ignored".to_string());
    let svc = LookupService::new(/*prefer_notes_over_dict=*/ false);

    let hit = svc.query("apple", &notes, &dict).expect("hit");
    match hit {
        LookupHit::Dict(e) => assert_eq!(e.headword, "apple"),
        LookupHit::Note(_) => panic!("expected Dict hit with dict-first policy"),
    }
}

#[test]
fn dict_first_falls_back_to_note_when_dict_misses() {
    let (_dir, dict) = mini_dict();
    let mut notes = NoteStore::new();
    notes.set("mango", "芒果".to_string());
    let svc = LookupService::new(false);

    let hit = svc.query("mango", &notes, &dict).expect("hit");
    match hit {
        LookupHit::Note(n) => assert_eq!(n.content, "芒果"),
        LookupHit::Dict(_) => panic!("expected Note fallback"),
    }
}

#[test]
fn both_miss_returns_not_found() {
    let (_dir, dict) = mini_dict();
    let notes = NoteStore::new();
    let svc = LookupService::new(true);
    let err = svc.query("nosuch", &notes, &dict).expect_err("miss");
    assert!(matches!(err, LookupError::NotFound), "got: {err:?}");
}

#[test]
fn invalid_input_short_circuits_before_either_store() {
    let (_dir, dict) = mini_dict();
    let notes = NoteStore::new();
    let svc = LookupService::new(true);
    for empty in ["", "   ", "\t\n"] {
        let err = svc.query(empty, &notes, &dict).expect_err("invalid");
        assert!(
            matches!(err, LookupError::InvalidInput),
            "expected InvalidInput for {empty:?}, got: {err:?}"
        );
    }
    let too_long = "x".repeat(200);
    let err = svc.query(&too_long, &notes, &dict).expect_err("invalid");
    assert!(matches!(err, LookupError::InvalidInput));
}

#[test]
fn case_insensitive_in_both_orderings() {
    let (_dir, dict) = mini_dict();
    let mut notes = NoteStore::new();
    notes.set("apple", "n".to_string());

    let note_first = LookupService::new(true);
    let dict_first = LookupService::new(false);

    assert!(note_first.query("APPLE", &notes, &dict).is_ok());
    assert!(dict_first.query("APPLE", &notes, &dict).is_ok());
}
