//! Integration tests for `Storage` — see `Core/tests/.test.md`.

use ee_core::{Storage, RecordProvider};
use tempfile::TempDir;

#[test]
fn new_storage_initializes_empty_tables() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    
    let storage = Storage::new(&db_path).expect("initialize storage");
    // Verify querying non-existent key returns None
    assert!(storage.get("apple").is_none());
}

#[test]
fn insert_or_update_persists_string() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    
    let storage = Storage::new(&db_path).expect("initialize storage");
    storage.insert_or_update("apple", r#"{"definition": "苹果"}"#);
    
    let val = storage.get("apple").expect("retrieve value");
    assert_eq!(val, r#"{"definition": "苹果"}"#);
}

#[test]
fn insert_or_update_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    
    let storage = Storage::new(&db_path).expect("initialize storage");
    storage.insert_or_update("apple", "first");
    storage.insert_or_update("apple", "second");
    
    let val = storage.get("apple").expect("retrieve value");
    assert_eq!(val, "second");
}

#[test]
fn delete_removes_key() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    
    let storage = Storage::new(&db_path).expect("initialize storage");
    storage.insert_or_update("apple", "first");
    assert!(storage.get("apple").is_some());
    
    storage.delete("apple");
    assert!(storage.get("apple").is_none());
}

#[test]
fn storage_implements_record_provider() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    
    let storage = Storage::new(&db_path).expect("initialize storage");
    storage.insert_or_update("apple", "trait fruit");
    
    // Test through dynamic or static dispatch of the RecordProvider trait
    let provider: &dyn RecordProvider = &storage;
    assert_eq!(provider.get("apple").unwrap(), "trait fruit");
}
