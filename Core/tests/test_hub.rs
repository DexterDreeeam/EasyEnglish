//! Integration tests for `Hub` — see `Core/tests/.test.md`.

use ee_core::{Hub, RecordModel, RecordProvider, Storage};
use ee_utils::Signal;
use std::path::PathBuf;
use std::sync::Arc;

fn dict_db_path(filename: &str) -> PathBuf {
    // CARGO_MANIFEST_DIR is Core/ at build time; workspace root is parent
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Dict")
        .join(filename)
}

#[test]
fn hub_queries_the_real_dictionary() {
    let mut hub = Hub::new();

    // Load the single bundled dictionary via the standard RecordProvider interface.
    let storage = Storage::new(dict_db_path("word_en_v1.sqlite")).expect("load dictionary");
    hub.add_provider(Arc::new(storage));

    // Query for a highly frequent word that is guaranteed to be present.
    let result_handle = hub.query(&["apple".to_string()]);

    // Wait for async background threads to finish streaming
    let mut finished = false;
    for _ in 0..100 {
        if let Signal::Finished = result_handle.wait(Some(std::time::Duration::from_millis(15))) {
            finished = true;
            break;
        }
    }

    assert!(finished);
    let records = result_handle.get();

    // Exactly one provider holds the word, so exactly one record comes back.
    assert_eq!(records.len(), 1);

    let model = records[0].deserialize().expect("deserialize word_en");
    if let RecordModel::WordEn(word) = model {
        assert_eq!(word.word, "apple");
        // Real ECDICT data carries a phonetic and at least one definition.
        assert!(word.pronunciation.is_some());
        assert!(word.definitions.is_some_and(|d| !d.is_empty()));
    } else {
        panic!("Expected WordEn variant!");
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
