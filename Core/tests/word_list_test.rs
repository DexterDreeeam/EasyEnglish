//! Integration test suite `word_list_test` — see `Core/tests/.test.md`.
//!
//! Performs full-scale lookup correctness testing of 1,000 real vocabulary items
//! against the single bundled dictionary (`word_en_v1.sqlite` + `word_en_v1`).

use ee_core::{Hub, RecordModel, Storage};
use ee_utils::Signal;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;

fn dict_file_path(filename: &str) -> PathBuf {
    // CARGO_MANIFEST_DIR is Core/ at build time; workspace root is parent
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Dict")
        .join(filename)
}

fn load_word_list(filename: &str) -> Vec<String> {
    let path = dict_file_path(filename);
    let file = File::open(&path).unwrap_or_else(|_| panic!("Failed to open word list: {:?}", path));
    let reader = BufReader::new(file);
    reader
        .lines()
        .map(|l| l.unwrap().trim().to_lowercase())
        .filter(|l| !l.is_empty())
        .collect()
}

#[test]
fn word_list_test() {
    println!("Initializing 1,000-word integration test suite (word_list_test)...");

    let mut hub = Hub::new();

    // Load the single bundled dictionary via the RecordProvider interface.
    let storage = Storage::new(dict_file_path("word_en_v1.sqlite")).expect("load dictionary");
    hub.add_provider(Arc::new(storage));

    // 1. Load the headword list paired with the database.
    let words = load_word_list("word_en_v1");
    assert!(
        words.len() >= 1_000,
        "word list unexpectedly small: {}",
        words.len()
    );

    // 2. Sample 1,000 test cases:
    //    - 900 words spread evenly across the list (each must hit exactly once)
    //    - 100 words that do not exist (each must hit zero times)
    let mut test_cases = Vec::new(); // (word, expected_count)
    let stride = words.len() / 900;
    for i in 0..900 {
        test_cases.push((words[i * stride].clone(), 1));
    }
    for i in 0..100 {
        test_cases.push((format!("nonexistentword{}", i), 0));
    }
    assert_eq!(test_cases.len(), 1000);

    // 3. Batch execute all 1,000 queries sequentially.
    //    Since sqlite is extremely fast, 1,000 queries finish in milliseconds.
    for (word, expected_count) in test_cases {
        let result_handle = hub.query(std::slice::from_ref(&word));

        // Block wait for completion
        let mut finished = false;
        for _ in 0..100 {
            if let Signal::Finished = result_handle.wait(Some(std::time::Duration::from_millis(5)))
            {
                finished = true;
                break;
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

    println!("All 1,000 word cases successfully processed, deserialized and asserted!");
}
