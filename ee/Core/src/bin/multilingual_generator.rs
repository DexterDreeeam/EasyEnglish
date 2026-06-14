//! Multilingual Wiktionary/Kaikki importer for EasyEnglish.
//!
//! Reads a local Kaikki English JSONL gzip file and emits language-specific
//! English → target and target → English datasets under `Dict/`.
//!
//! Usage:
//!
//! ```text
//! cargo run -p ee-core --bin multilingual_generator -- <kaikki-english.jsonl.gz>
//! ```
//!
//! The generated assets are derived from Wiktionary/Kaikki data. Keep the
//! corresponding attribution file with the generated dictionary files.

use flate2::read::GzDecoder;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use ee_core::{Definition, RecordModel, SerializableRecord, WordCn, WordEn};

const MAX_REVERSE_ENGLISH: usize = 10;
const MAX_MAJOR_TERMS: usize = 8;
const MAX_TARGET_TERM_CHARS: usize = 60;

#[derive(Clone, Copy)]
struct LanguageSpec {
    code: &'static str,
    english_base: &'static str,
    reverse_base: &'static str,
}

const LANGUAGES: &[LanguageSpec] = &[
    LanguageSpec {
        code: "es",
        english_base: "word_en_es_v1",
        reverse_base: "word_es_v1",
    },
    LanguageSpec {
        code: "ja",
        english_base: "word_en_ja_v1",
        reverse_base: "word_ja_v1",
    },
    LanguageSpec {
        code: "ko",
        english_base: "word_en_ko_v1",
        reverse_base: "word_ko_v1",
    },
    LanguageSpec {
        code: "pt",
        english_base: "word_en_pt_v1",
        reverse_base: "word_pt_v1",
    },
    LanguageSpec {
        code: "id",
        english_base: "word_en_id_v1",
        reverse_base: "word_id_v1",
    },
    LanguageSpec {
        code: "ar",
        english_base: "word_en_ar_v1",
        reverse_base: "word_ar_v1",
    },
    LanguageSpec {
        code: "vi",
        english_base: "word_en_vi_v1",
        reverse_base: "word_vi_v1",
    },
    LanguageSpec {
        code: "hi",
        english_base: "word_en_hi_v1",
        reverse_base: "word_hi_v1",
    },
    LanguageSpec {
        code: "fr",
        english_base: "word_en_fr_v1",
        reverse_base: "word_fr_v1",
    },
];

#[derive(Default)]
struct LanguageData {
    english: BTreeMap<String, EnglishAccumulator>,
    reverse: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Default)]
struct EnglishAccumulator {
    pos_to_terms: BTreeMap<String, BTreeSet<String>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Dict/kaikki.org-dictionary-English.jsonl.gz".to_string());
    let source_path = Path::new(&source);
    if !source_path.exists() {
        return Err(format!(
            "Kaikki English JSONL gzip not found at {:?}. Download \
             https://kaikki.org/dictionary/English/kaikki.org-dictionary-English.jsonl.gz \
             and pass its path as the first argument.",
            source_path
        )
        .into());
    }

    let mut data: HashMap<&'static str, LanguageData> = LANGUAGES
        .iter()
        .map(|lang| (lang.code, LanguageData::default()))
        .collect();

    read_kaikki(source_path, &mut data)?;

    let out_dir = Path::new("Dict");
    for lang in LANGUAGES {
        let lang_data = data
            .get(lang.code)
            .ok_or_else(|| format!("missing accumulator for {}", lang.code))?;
        write_english_dataset(
            &lang_data.english,
            &out_dir.join(lang.english_base),
            &out_dir.join(format!("{}.sqlite", lang.english_base)),
        )?;
        write_reverse_dataset(
            &lang_data.reverse,
            &out_dir.join(lang.reverse_base),
            &out_dir.join(format!("{}.sqlite", lang.reverse_base)),
        )?;
    }
    write_attribution(out_dir)?;
    Ok(())
}

fn read_kaikki(
    source_path: &Path,
    data: &mut HashMap<&'static str, LanguageData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(source_path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    for (line_no, line) in reader.lines().enumerate() {
        let line = line?;
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Skipping malformed JSON line {}: {}", line_no + 1, err);
                continue;
            }
        };

        let word = match value.get("word").and_then(Value::as_str) {
            Some(word) if is_clean_english_word(word) => word.to_lowercase(),
            _ => continue,
        };
        let pos = value
            .get("pos")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let mut seen_for_entry = HashSet::new();
        collect_translations(&value, &word, &pos, data, &mut seen_for_entry);
    }

    Ok(())
}

fn collect_translations(
    value: &Value,
    english: &str,
    pos: &str,
    data: &mut HashMap<&'static str, LanguageData>,
    seen_for_entry: &mut HashSet<(String, String)>,
) {
    if let Some(translations) = value.get("translations").and_then(Value::as_array) {
        collect_translation_array(translations, english, pos, data, seen_for_entry);
    }

    if let Some(senses) = value.get("senses").and_then(Value::as_array) {
        for sense in senses {
            if let Some(translations) = sense.get("translations").and_then(Value::as_array) {
                let sense_pos = sense
                    .get("pos")
                    .and_then(Value::as_str)
                    .unwrap_or(pos);
                collect_translation_array(translations, english, sense_pos, data, seen_for_entry);
            }
        }
    }
}

fn collect_translation_array(
    translations: &[Value],
    english: &str,
    pos: &str,
    data: &mut HashMap<&'static str, LanguageData>,
    seen_for_entry: &mut HashSet<(String, String)>,
) {
    for tr in translations {
        let code = tr
            .get("lang_code")
            .or_else(|| tr.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let Some(lang_data) = data.get_mut(code) else {
            continue;
        };

        let Some(term) = tr.get("word").and_then(Value::as_str) else {
            continue;
        };
        let term = normalize_target_term(term);
        if !is_valid_target_term(&term) {
            continue;
        }

        if !seen_for_entry.insert((code.to_string(), term.clone())) {
            continue;
        }

        lang_data
            .english
            .entry(english.to_string())
            .or_default()
            .pos_to_terms
            .entry(pos.to_string())
            .or_default()
            .insert(term.clone());
        lang_data
            .reverse
            .entry(term)
            .or_default()
            .insert(english.to_string());
    }
}

fn write_english_dataset(
    entries: &BTreeMap<String, EnglishAccumulator>,
    list_path: &Path,
    db_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    write_word_list(entries.keys(), list_path)?;

    recreate_db(db_path)?;
    let mut conn = Connection::open(db_path)?;
    let tx = conn.transaction()?;
    {
        let mut stmt =
            tx.prepare("INSERT OR REPLACE INTO storage_entries (key, value) VALUES (?, ?)")?;
        for (key, acc) in entries {
            let word = WordEn {
                word: key.clone(),
                major: Some(major_terms(acc).join(", ")),
                pronunciation: None,
                definitions: Some(definitions(acc)),
                inflections: None,
                examples: None,
            };
            let serialized = RecordModel::WordEn(word).serialize()?;
            stmt.execute(params![key, serialized])?;
        }
    }
    tx.commit()?;
    println!("Wrote {} entries to {}", entries.len(), db_path.display());
    Ok(())
}

fn write_reverse_dataset(
    entries: &BTreeMap<String, BTreeSet<String>>,
    list_path: &Path,
    db_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    write_word_list(entries.keys(), list_path)?;

    recreate_db(db_path)?;
    let mut conn = Connection::open(db_path)?;
    let tx = conn.transaction()?;
    {
        let mut stmt =
            tx.prepare("INSERT OR REPLACE INTO storage_entries (key, value) VALUES (?, ?)")?;
        for (key, english_words) in entries {
            let english = english_words
                .iter()
                .take(MAX_REVERSE_ENGLISH)
                .cloned()
                .collect::<Vec<_>>();
            let serialized = RecordModel::WordCn(WordCn {
                word: key.clone(),
                english,
            })
            .serialize()?;
            stmt.execute(params![key, serialized])?;
        }
    }
    tx.commit()?;
    println!("Wrote {} entries to {}", entries.len(), db_path.display());
    Ok(())
}

fn recreate_db(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let conn = Connection::open(path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS storage_entries (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn write_word_list<'a>(
    keys: impl Iterator<Item = &'a String>,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(path)?;
    let mut count = 0usize;
    for key in keys {
        writeln!(file, "{}", key)?;
        count += 1;
    }
    println!("Wrote {} words to {}", count, path.display());
    Ok(())
}

fn major_terms(acc: &EnglishAccumulator) -> Vec<String> {
    let mut terms = Vec::new();
    for term_set in acc.pos_to_terms.values() {
        for term in term_set {
            terms.push(term.clone());
            if terms.len() >= MAX_MAJOR_TERMS {
                return terms;
            }
        }
    }
    terms
}

fn definitions(acc: &EnglishAccumulator) -> Vec<Definition> {
    acc.pos_to_terms
        .iter()
        .map(|(pos, terms)| Definition {
            pos: pos.clone(),
            meanings: terms.iter().cloned().collect(),
        })
        .collect()
}

fn is_clean_english_word(word: &str) -> bool {
    let n = word.chars().count();
    ((2..=32).contains(&n) && word.chars().all(|c| c.is_ascii_alphabetic()))
        || word == "a"
        || word == "i"
}

fn normalize_target_term(term: &str) -> String {
    term.trim().to_lowercase()
}

fn is_valid_target_term(term: &str) -> bool {
    let n = term.chars().count();
    (1..=MAX_TARGET_TERM_CHARS).contains(&n)
        && !term.chars().any(char::is_control)
        && term.chars().any(|c| c.is_alphabetic())
}

fn write_attribution(out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = PathBuf::from(out_dir);
    path.push("WIKTIONARY-KAIKKI-LICENSE.txt");
    let mut file = File::create(path)?;
    file.write_all(
        b"EasyEnglish multilingual dictionary assets are derived from Wiktionary data extracted by Kaikki/Wiktextract.\n\nSource: https://kaikki.org/dictionary/English/\nRaw download: https://kaikki.org/dictionary/English/kaikki.org-dictionary-English.jsonl.gz\nUpstream license: Creative Commons Attribution-ShareAlike 3.0 Unported (CC BY-SA 3.0)\nLicense URL: https://creativecommons.org/licenses/by-sa/3.0/\n\nPlease cite: Tatu Ylonen, Wiktextract: Wiktionary as Machine-Readable Structured Data, Proceedings of LREC 2022.\n",
    )?;
    Ok(())
}
