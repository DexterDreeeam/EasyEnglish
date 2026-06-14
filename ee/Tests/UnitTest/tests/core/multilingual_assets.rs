use ee_core::Storage;
use rusqlite::Connection;
use std::fs::File;
use std::io::{BufRead, BufReader};

const LANG_DATASETS: &[(&str, &str)] = &[
    ("word_en_cn_v1", "word_cn_v1"),
    ("word_en_es_v1", "word_es_v1"),
    ("word_en_ja_v1", "word_ja_v1"),
    ("word_en_ko_v1", "word_ko_v1"),
    ("word_en_pt_v1", "word_pt_v1"),
    ("word_en_id_v1", "word_id_v1"),
    ("word_en_ar_v1", "word_ar_v1"),
    ("word_en_vi_v1", "word_vi_v1"),
    ("word_en_hi_v1", "word_hi_v1"),
    ("word_en_fr_v1", "word_fr_v1"),
];

#[test]
fn multilingual_dictionary_assets_are_readable() {
    for (english_base, reverse_base) in LANG_DATASETS {
        assert_dataset_readable(english_base);
        assert_dataset_readable(reverse_base);
    }
}

fn assert_dataset_readable(base: &str) {
    let list_path = super::paths::dict_file(base);
    let db_path = super::paths::dict_file(&format!("{base}.sqlite"));
    assert!(list_path.is_file(), "missing word list: {list_path:?}");
    assert!(db_path.is_file(), "missing sqlite DB: {db_path:?}");

    let keys = load_keys(base);
    assert!(!keys.is_empty(), "empty word list: {list_path:?}");
    let row_count = sqlite_row_count(&db_path);
    assert_eq!(
        keys.len(),
        row_count,
        "word-list count and sqlite row count differ for {base}"
    );

    let first = &keys[0];
    let storage = Storage::new(&db_path).unwrap_or_else(|_| panic!("open sqlite DB: {db_path:?}"));
    let value = storage
        .get(first)
        .unwrap_or_else(|| panic!("missing first key {first:?} in {db_path:?}"));
    assert!(
        value.contains("\"record_type\""),
        "serialized record for {first:?} in {db_path:?} has no record_type"
    );
}

fn load_keys(base: &str) -> Vec<String> {
    let list_path = super::paths::dict_file(base);
    let file = File::open(&list_path).unwrap_or_else(|_| panic!("open word list: {list_path:?}"));
    let reader = BufReader::new(file);
    reader
        .lines()
        .map(|line| line.unwrap_or_else(|_| panic!("read word list: {list_path:?}")))
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn sqlite_row_count(db_path: &std::path::Path) -> usize {
    let conn = Connection::open(db_path).unwrap_or_else(|_| panic!("open sqlite DB: {db_path:?}"));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM storage_entries", [], |row| row.get(0))
        .unwrap_or_else(|_| panic!("count sqlite rows: {db_path:?}"));
    assert!(count > 0, "empty sqlite DB: {db_path:?}");
    count as usize
}
