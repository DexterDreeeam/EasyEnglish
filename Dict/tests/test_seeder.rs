//! Verifies the canonical shipped seed (`Dict/data/seed_en_cn.json`) loads
//! cleanly and every entry is well-formed. See `Dict/tests/.test.md`.

use std::path::PathBuf;

use ee_dict::DictStore;
use tempfile::TempDir;

/// Path to the seed JSON relative to the workspace root. `CARGO_MANIFEST_DIR`
/// is the crate's own directory at build time; the workspace root is its parent.
fn seed_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("data").join("seed_en_cn.json")
}

#[test]
fn real_seed_loads_cleanly() {
    assert!(seed_path().exists(), "seed file is missing");

    let dir = TempDir::new().unwrap();
    let db = dir.path().join("dict.sqlite3");
    let dict = DictStore::create_or_seed(&db, seed_path()).expect("load real seed");
    assert!(
        dict.len() >= 200,
        "seed has shrunk to {}; AGENTS.md requires >= 200 entries",
        dict.len()
    );
}

#[test]
fn every_entry_has_nonempty_definitions() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("dict.sqlite3");
    let dict = DictStore::create_or_seed(&db, seed_path()).expect("load real seed");

    // Read the raw JSON once to enumerate every headword, then ask DictStore
    // for each. Using the public lookup API ensures the seed loaded faithfully.
    let bytes = std::fs::read(seed_path()).unwrap();
    let raw: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let entries = raw.as_array().expect("array");
    assert!(!entries.is_empty());

    for rec in entries {
        let word = rec
            .get("headword")
            .and_then(|v| v.as_str())
            .expect("headword string");
        let entry = dict
            .lookup(word)
            .unwrap_or_else(|e| panic!("seed entry `{word}` missing or unreadable: {e:?}"));
        assert!(
            !entry.definitions.is_empty(),
            "seed entry `{word}` has no Chinese definition"
        );
    }
}

#[test]
fn headwords_are_ascii_lowercase() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("dict.sqlite3");
    let dict = DictStore::create_or_seed(&db, seed_path()).expect("load real seed");

    // Probe a few words known to be in the seed; each returned headword must
    // be all-ASCII and lower-case (the seed is restricted to such on purpose).
    for sample in ["apple", "answer", "morning", "zero"] {
        let entry = dict.lookup(sample).expect("hit");
        assert!(
            entry
                .headword
                .chars()
                .all(|c| c.is_ascii() && (c.is_ascii_lowercase() || c == '-')),
            "headword `{}` should be ASCII lowercase",
            entry.headword
        );
    }
}
