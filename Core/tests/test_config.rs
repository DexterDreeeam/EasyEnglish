//! Integration tests for `Config` — see `Core/tests/.test.md`.

use std::fs;
use std::path::PathBuf;

use ee_core::{Config, ConfigError};
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `Core/` at build time; the workspace root is its parent.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn load_real_product_json_uses_documented_defaults() {
    let path = workspace_root().join("product.json");
    let config = Config::load(&path).expect("load real product.json");
    assert_eq!(config.history_max_entries(), 50);
    assert!(!config.notes_persist());
    assert!(config.lookup_prefer_notes_over_dict());
    assert!(config.dict_seed_on_first_open());
    // Path is given relative to the workspace root in product.json.
    assert!(config.dict_data_path().ends_with("seed_en_cn.json"));
    assert!(config.dict_sqlite_path().is_none());
}

#[test]
fn load_missing_file_returns_io_error() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("nope.json");
    let err = Config::load(&missing).expect_err("missing file should error");
    assert!(matches!(err, ConfigError::Io { .. }), "got: {err:?}");
}

#[test]
fn load_malformed_json_returns_parse_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.json");
    fs::write(&path, "this is not json").unwrap();
    let err = Config::load(&path).expect_err("garbled json should error");
    assert!(matches!(err, ConfigError::Parse(_)), "got: {err:?}");
}

#[test]
fn partial_override_keeps_other_defaults() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("partial.json");
    fs::write(&path, r#"{ "core": { "history": { "max_entries": 7 } } }"#).unwrap();
    let config = Config::load(&path).expect("partial load");
    assert_eq!(config.history_max_entries(), 7);
    // Other knobs stay at default.
    assert!(!config.notes_persist());
    assert!(config.lookup_prefer_notes_over_dict());
}
