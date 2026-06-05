//! Integration tests for `DictStore` — see `Dict/tests/.test.md` for the
//! test specification.

use std::fs;

use ee_dict::{DictError, DictStore};
use tempfile::TempDir;

/// Build a tiny seed JSON inside a fresh tempdir and return
/// `(tempdir, seed_path, db_path)`. Keeping the tempdir alive in the caller
/// is required: it is dropped when the function-local binding goes out of
/// scope and the files on disk are removed with it.
fn make_seeded(words: &str) -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let seed = dir.path().join("seed.json");
    let db = dir.path().join("dict.sqlite3");
    fs::write(&seed, words).expect("write seed");
    (dir, seed, db)
}

const TINY_SEED: &str = r#"[
    {"headword":"apple","phonetic":"/ˈæp.əl/","definitions":["苹果","苹果树"]},
    {"headword":"apply","phonetic":"/əˈplaɪ/","definitions":["申请"]},
    {"headword":"book","phonetic":"/bʊk/","definitions":["书","预订"]}
]"#;

#[test]
fn create_or_seed_loads_all_entries() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    assert_eq!(dict.len(), 3);
    assert!(!dict.is_empty());
}

#[test]
fn create_or_seed_is_idempotent() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let first = DictStore::create_or_seed(&db, &seed).expect("seed first");
    assert_eq!(first.len(), 3);
    drop(first);
    let second = DictStore::create_or_seed(&db, &seed).expect("seed second");
    assert_eq!(
        second.len(),
        3,
        "second open should observe the same row count, not re-seed"
    );
}

#[test]
fn open_missing_file_returns_storage_error() {
    let dir = TempDir::new().expect("tempdir");
    let missing = dir.path().join("nope.sqlite3");
    let err = DictStore::open(&missing).expect_err("expected error for missing file");
    assert!(matches!(err, DictError::Storage(_)), "got: {err:?}");
}

#[test]
fn lookup_hit_returns_canonical_entry() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    let entry = dict.lookup("apple").expect("hit");
    assert_eq!(entry.headword, "apple");
    assert_eq!(entry.phonetic, "/ˈæp.əl/");
    assert_eq!(
        entry.definitions,
        vec!["苹果".to_string(), "苹果树".to_string()]
    );
}

#[test]
fn lookup_is_case_insensitive() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    for variant in ["APPLE", "Apple", "ApPlE"] {
        let entry = dict
            .lookup(variant)
            .unwrap_or_else(|_| panic!("hit {variant}"));
        assert_eq!(
            entry.headword, "apple",
            "expected canonical lowercase headword regardless of input casing"
        );
    }
}

#[test]
fn lookup_unknown_word_returns_not_found() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    let err = dict.lookup("nosuch").expect_err("miss");
    assert!(matches!(err, DictError::NotFound), "got: {err:?}");
}

#[test]
fn lookup_empty_returns_invalid_input() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    for empty in ["", "   ", "\t\n"] {
        let err = dict.lookup(empty).expect_err("invalid");
        assert!(
            matches!(err, DictError::InvalidInput),
            "got: {err:?} for {empty:?}"
        );
    }
}

#[test]
fn lookup_oversized_returns_invalid_input() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    let too_long = "x".repeat(200);
    let err = dict.lookup(&too_long).expect_err("invalid");
    assert!(matches!(err, DictError::InvalidInput));
}

#[test]
fn suggest_empty_returns_empty() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    assert!(dict.suggest("", 5).is_empty());
    assert!(dict.suggest("   ", 5).is_empty());
}

#[test]
fn suggest_max_zero_returns_empty() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    assert!(dict.suggest("apple", 0).is_empty());
}

#[test]
fn suggest_exact_match_ranks_first() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    let results = dict.suggest("apple", 3);
    assert_eq!(results.first().map(String::as_str), Some("apple"));
}

#[test]
fn suggest_typo_ranks_correct_word_first() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    let results = dict.suggest("appel", 1);
    assert_eq!(
        results.first().map(String::as_str),
        Some("apple"),
        "distance-1 hit `apple` must beat distance-2 `apply`; got {results:?}"
    );
}

#[test]
fn suggest_respects_max_limit() {
    let (_dir, seed, db) = make_seeded(TINY_SEED);
    let dict = DictStore::create_or_seed(&db, &seed).expect("seed");
    assert!(dict.suggest("z", 2).len() <= 2);
}
