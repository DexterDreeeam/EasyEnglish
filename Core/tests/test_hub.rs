//! Integration tests for `Hub` — see `Core/tests/.test.md`.

use std::sync::Arc;
use std::collections::HashSet;
use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader};
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

fn load_word_list(filename: &str) -> HashSet<String> {
    let path = dict_db_path(filename);
    let file = File::open(&path).unwrap_or_else(|_| panic!("Failed to open word list: {:?}", path));
    let reader = BufReader::new(file);
    reader.lines().map(|l| l.unwrap().trim().to_lowercase()).collect()
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
    let result_handle = hub.query("apply");

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

#[test]
fn hub_full_word_list_overlap_test() {
    let mut hub = Hub::new();

    // Load all three real DBs
    let s1 = Storage::new(dict_db_path("word_en_v1.sqlite")).expect("load v1");
    let s2 = Storage::new(dict_db_path("word_en_v2.sqlite")).expect("load v2");
    let s3 = Storage::new(dict_db_path("word_en_v3.sqlite")).expect("load v3");

    hub.add_provider(Arc::new(s1));
    hub.add_provider(Arc::new(s2));
    hub.add_provider(Arc::new(s3));

    // 1. Load word list files to check overlaps
    let v1_words = load_word_list("word_list_v1");
    let v2_words = load_word_list("word_list_v2");
    let v3_words = load_word_list("word_list_v3");

    // 2. Sample 1000 test cases
    // - 300 words that exist in v1, v2, and v3
    // - 300 words that exist only in v2 and v3 (not in v1)
    // - 300 words that exist only in v3 (not in v1 and v2)
    // - 100 words that do not exist in any list
    let mut test_cases = Vec::new(); // (word, expected_count)

    // Select v1 overlapping (must be in all three because v1 is subset of v2, which is subset of v3)
    for w in v1_words.iter().take(300) {
        test_cases.push((w.clone(), 3));
    }

    // Select v2-only (in v2 and v3, but not in v1)
    let v2_only: Vec<String> = v2_words.iter().filter(|w| !v1_words.contains(*w)).cloned().collect();
    for w in v2_only.iter().take(300) {
        test_cases.push((w.clone(), 2));
    }

    // Select v3-only (in v3, but not in v1 or v2)
    let v3_only: Vec<String> = v3_words.iter().filter(|w| !v2_words.contains(*w)).cloned().collect();
    for w in v3_only.iter().take(300) {
        test_cases.push((w.clone(), 1));
    }

    // Select 100 non-existent words
    for i in 0..100 {
        test_cases.push((format!("nonexistentword{}", i), 0));
    }

    assert_eq!(test_cases.len(), 1000);

    // 3. Batch execute all 1,000 queries sequentially
    // Since sqlite is extremely fast, 1,000 queries will finish in milliseconds.
    for (word, expected_count) in test_cases {
        let result_handle = hub.query(&word);

        // Block wait for completion
        let mut finished = false;
        for _ in 0..100 {
            match result_handle.wait(Some(std::time::Duration::from_millis(5))) {
                Signal::Finished => {
                    finished = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(finished, "Query for '{}' did not finish!", word);
        let records = result_handle.get();
        assert_eq!(
            records.len(),
            expected_count,
            "Word '{}' failed! Expected hit count: {}, got: {}",
            word,
            expected_count,
            records.len()
        );

        // Verify deserialization correctness
        for rec in records {
            assert_eq!(rec.key, word);
            let model = rec.deserialize().expect("successful deserialization");
            if let RecordModel::WordEn(word_en) = model {
                assert_eq!(word_en.word, word);
                assert!(word_en.definitions.is_some());
            } else {
                panic!("Expected WordEn variant!");
            }
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

    let result_handle = hub.query("test");
    
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
