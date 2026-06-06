//! Integration tests for `Hub` — see `Core/tests/.test.md`.

use std::sync::Arc;
use std::path::PathBuf;
use ee_core::{Hub, RecordModel, Storage, RecordProvider};
use ee_utils::Signal;

fn dict_db_path(filename: &str) -> PathBuf {
// CARGO_MANIFEST_DIR is Core/ at build time; workspace root is parent
PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("Dict")
    .join(filename)
}

#[test]
fn hub_concurrently_queries_three_real_dbs() {
let mut hub = Hub::new();

// Load all three real DBs via standard relative path loading (using RecordProvider interface)
let s1 = Storage::new(dict_db_path("word_en_v1.sqlite")).expect("load v1");
let s2 = Storage::new(dict_db_path("word_en_v2.sqlite")).expect("load v2");
let s3 = Storage::new(dict_db_path("word_en_v3.sqlite")).expect("load v3");

hub.add_provider(Arc::new(s1));
hub.add_provider(Arc::new(s2));
hub.add_provider(Arc::new(s3));

// Query for a highly frequent word present in all three databases ("apply")
let result_handle = hub.query(&["apply".to_string()]);

// Wait for async background threads to finish streaming
let mut finished = false;
for _ in 0..100 {
    match result_handle.wait(Some(std::time::Duration::from_millis(15))) {
        Signal::Finished => {
            finished = true;
            break;
        }
        _ => {}
    }
}

assert!(finished);
let records = result_handle.get();

// Word should have been hit in all 3 databases (v1, v2, v3)
assert_eq!(records.len(), 3);

// Verify all 3 records deserialize into identical, fully populated WordEn structures
for rec in records {
    let model = rec.deserialize().expect("deserialize word_en");
    if let RecordModel::WordEn(word) = model {
        assert_eq!(word.word, "apply");
        assert_eq!(word.pronunciation.as_ref().unwrap().ipa, "əˈplaɪ");
        assert_eq!(word.inflections.as_ref().unwrap().past_tense.as_ref().unwrap(), "applied");
    } else {
        panic!("Expected WordEn variant!");
    }
}
}

struct SlowProvider {
delay: std::time::Duration,
}

impl RecordProvider for SlowProvider {
fn get(&self, key: &str) -> Option<String> {
    std::thread::sleep(self.delay);
    if key == "test" {
        Some("slow_val".to_string())
    } else {
        None
    }
}
}

#[test]
fn hub_can_be_cancelled_mid_get() {
let mut hub = Hub::new();
hub.add_provider(Arc::new(SlowProvider {
    delay: std::time::Duration::from_millis(500),
}));

let result_handle = hub.query(&["test".to_string()]);
    
// Let the worker loop run and hit the wait, then immediately cancel
std::thread::sleep(std::time::Duration::from_millis(50));
result_handle.cancel();

// Verify it finishes immediately without waiting the full 500ms
let start = std::time::Instant::now();
let sig = result_handle.wait(Some(std::time::Duration::from_millis(200)));
let elapsed = start.elapsed();

assert!(matches!(sig, Signal::Finished));
assert!(elapsed < std::time::Duration::from_millis(200));
}
