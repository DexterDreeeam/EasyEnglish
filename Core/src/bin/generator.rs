//! ECDICT importer / dataset generator for EasyEnglish.
//!
//! Reads the upstream ECDICT SQLite database (`stardict` table) and emits the
//! highest-tier dataset consumed by the app:
//!   * `Dict/word_en_v4.sqlite` — `storage_entries(key, value)` with serialized
//!     [`RecordModel::WordEn`] JSON values.
//!   * `Dict/word_list_v4` — newline-separated, lowercase, sorted headwords used
//!     for the in-memory fuzzy/prefix suggestion list.
//!
//! Selection: keep clean single words that have a Chinese translation, dropping
//! proper nouns (capitalised originals) and pure inflected forms (ECDICT
//! `exchange` `0:lemma`). Words are ranked by commonness — contemporary corpus
//! frequency (`frq`), then BNC (`bnc`), then exam/Collins/Oxford tags, then the
//! remaining real (phonetic-bearing) words — and the top `TARGET_WORDS` are kept.
//!
//! The ECDICT source DB is a large (~800 MB) build input and is NOT committed;
//! download it from the ECDICT release and place it at `Dict/ecdict.db` (see the
//! repo `.gitignore`). Run with: `cargo run -p ee-core --bin generator`.

use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use ee_core::{Definition, Inflections, Pronunciation, RecordModel, SerializableRecord, WordEn};

/// Number of headwords to emit into the v4 dataset.
const TARGET_WORDS: usize = 100_000;
/// Maximum Chinese definition lines kept per word (keeps rows compact).
const MAX_DEFINITIONS: usize = 8;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Dict/ecdict.db".to_string());
    let src_path = Path::new(&src);
    if !src_path.exists() {
        return Err(format!(
            "ECDICT source database not found at {:?}. Download ecdict-sqlite from the \
             ECDICT release and place it there (see .gitignore).",
            src_path
        )
        .into());
    }

    println!("Reading ECDICT source: {:?}", src_path);
    let selected = select_words(src_path)?;
    println!("Selected {} headwords.", selected.len());

    write_dataset(&selected, "Dict/word_list_v4", "Dict/word_en_v4.sqlite")?;
    println!("Done.");
    Ok(())
}

/// A raw ECDICT row reduced to the fields we need, with a computed ranking key.
struct Candidate {
    key: String,
    word_en: WordEn,
    tier: u8,
    rank: i64,
    tie: i64,
}

fn select_words(src_path: &Path) -> Result<Vec<Candidate>, Box<dyn std::error::Error>> {
    let conn = Connection::open(src_path)?;
    let mut stmt = conn.prepare(
        "SELECT word, phonetic, translation, exchange, frq, bnc, collins, oxford, tag \
         FROM stardict WHERE translation IS NOT NULL AND translation <> ''",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?.unwrap_or_default(),
            r.get::<_, Option<String>>(2)?.unwrap_or_default(),
            r.get::<_, Option<String>>(3)?.unwrap_or_default(),
            r.get::<_, Option<i64>>(4)?.unwrap_or(0),
            r.get::<_, Option<i64>>(5)?.unwrap_or(0),
            r.get::<_, Option<i64>>(6)?.unwrap_or(0),
            r.get::<_, Option<i64>>(7)?.unwrap_or(0),
            r.get::<_, Option<String>>(8)?.unwrap_or_default(),
        ))
    })?;

    let mut candidates: Vec<Candidate> = Vec::new();
    for row in rows.flatten() {
        let (word, phonetic, translation, exchange, frq, bnc, collins, oxford, tag) = row;

        if !is_clean_word(&word) {
            continue;
        }
        // Drop proper nouns / acronyms (capitalised original).
        if word.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            continue;
        }
        let key = word.to_lowercase();
        // Drop pure inflected forms; the lemma carries the inflections instead.
        if is_inflection_of_other(&exchange, &key) {
            continue;
        }

        let len = key.chars().count() as i64;
        let has_phonetic = !phonetic.trim().is_empty();

        let (tier, rank, tie) = if frq > 0 {
            (0u8, frq, 0)
        } else if bnc > 0 {
            (1, bnc, len)
        } else if collins > 0 || oxford > 0 || !tag.trim().is_empty() {
            (2, -(collins * 10 + oxford), len)
        } else if has_phonetic && has_vowel(&key) && len >= 3 {
            (3, len, 0)
        } else {
            continue; // unranked, no phonetic → very likely junk/abbreviation
        };

        let word_en = build_word_en(&key, &phonetic, &translation, &exchange);
        candidates.push(Candidate {
            key,
            word_en,
            tier,
            rank,
            tie,
        });
    }

    // Most-common first: tier, then rank, then tie-break, then alphabetical.
    candidates.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then(a.rank.cmp(&b.rank))
            .then(a.tie.cmp(&b.tie))
            .then(a.key.cmp(&b.key))
    });

    // Keep the first (best) occurrence of each key, capped at TARGET_WORDS.
    let mut seen = HashSet::new();
    let mut selected = Vec::with_capacity(TARGET_WORDS);
    for cand in candidates {
        if seen.insert(cand.key.clone()) {
            selected.push(cand);
            if selected.len() >= TARGET_WORDS {
                break;
            }
        }
    }
    Ok(selected)
}

fn write_dataset(
    selected: &[Candidate],
    list_path: &str,
    db_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Word list: lowercase, unique, alphabetically sorted.
    let mut keys: Vec<&str> = selected.iter().map(|c| c.key.as_str()).collect();
    keys.sort_unstable();
    let mut list_file = File::create(list_path)?;
    for k in &keys {
        writeln!(list_file, "{}", k)?;
    }
    println!("Wrote {} words to {}", keys.len(), list_path);

    let path = Path::new(db_path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let mut conn = Connection::open(path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS storage_entries (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;

    let tx = conn.transaction()?;
    {
        let mut stmt =
            tx.prepare("INSERT OR REPLACE INTO storage_entries (key, value) VALUES (?, ?)")?;
        for cand in selected {
            let serialized = RecordModel::WordEn(cand.word_en.clone()).serialize()?;
            stmt.execute(params![cand.key, serialized])?;
        }
    }
    tx.commit()?;
    println!("Wrote {} entries to {}", selected.len(), db_path);
    Ok(())
}

/// Map an ECDICT row into the app's strongly-typed [`WordEn`].
fn build_word_en(key: &str, phonetic: &str, translation: &str, exchange: &str) -> WordEn {
    let (major, definitions) = parse_translation(translation);
    let pronunciation = {
        let ipa = phonetic.trim();
        if ipa.is_empty() {
            None
        } else {
            Some(Pronunciation {
                ipa: ipa.to_string(),
                audio: None,
                audio_url: None,
            })
        }
    };
    WordEn {
        word: key.to_string(),
        major,
        pronunciation,
        definitions,
        inflections: parse_inflections(exchange),
        examples: None,
    }
}

/// A clean single word: ASCII letters only, length 2..=22, plus `a`/`i`.
fn is_clean_word(word: &str) -> bool {
    let n = word.chars().count();
    ((2..=22).contains(&n) && word.chars().all(|c| c.is_ascii_alphabetic()))
        || word == "a"
        || word == "i"
}

fn has_vowel(word: &str) -> bool {
    word.chars()
        .any(|c| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))
}

/// True if `exchange` marks this word as a derived form of a different lemma
/// (ECDICT `0:lemma`). Such inflected forms are dropped in favour of the lemma.
fn is_inflection_of_other(exchange: &str, key: &str) -> bool {
    for item in exchange.split('/') {
        if let Some(lemma) = item.strip_prefix("0:") {
            let lemma = lemma.trim().to_lowercase();
            if !lemma.is_empty() && lemma != key {
                return true;
            }
        }
    }
    false
}

/// Parse an ECDICT multi-line Chinese `translation` into a concise `major`
/// gloss (first line) and per-line [`Definition`]s.
fn parse_translation(translation: &str) -> (Option<String>, Option<Vec<Definition>>) {
    let mut defs: Vec<Definition> = Vec::new();
    let mut major: Option<String> = None;

    for raw_line in translation.split('\n') {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let (pos, meaning) = split_pos(line);
        if major.is_none() {
            let gloss = if meaning.is_empty() { line } else { &meaning };
            major = Some(gloss.to_string());
        }
        defs.push(Definition {
            pos,
            meanings: vec![if meaning.is_empty() {
                line.to_string()
            } else {
                meaning
            }],
        });
        if defs.len() >= MAX_DEFINITIONS {
            break;
        }
    }

    let definitions = if defs.is_empty() { None } else { Some(defs) };
    (major, definitions)
}

/// Split a leading part-of-speech token (e.g. `n.`, `vt.`, `adj.`) from a
/// translation line. Returns `(pos, remainder)`; `pos` is empty when the line
/// has no recognisable POS prefix (e.g. lines that begin with a bracketed
/// domain marker instead of a part-of-speech token).
fn split_pos(line: &str) -> (String, String) {
    if let Some(sp) = line.find(' ') {
        let head = &line[..sp];
        if let Some(stem) = head.strip_suffix('.') {
            if !stem.is_empty() && stem.chars().all(|c| c.is_ascii_alphabetic()) {
                return (head.to_string(), line[sp + 1..].trim().to_string());
            }
        }
    }
    (String::new(), line.to_string())
}

/// Parse ECDICT `exchange` (`p:.../d:.../i:.../3:.../s:...`) into [`Inflections`].
fn parse_inflections(exchange: &str) -> Option<Inflections> {
    let mut inf = Inflections {
        plural: None,
        past_tense: None,
        past_participle: None,
        present_participle: None,
        third_singular: None,
    };
    let mut any = false;
    for item in exchange.split('/') {
        let mut parts = item.splitn(2, ':');
        let kind = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if value.is_empty() {
            continue;
        }
        let slot = match kind {
            "p" => &mut inf.past_tense,
            "d" => &mut inf.past_participle,
            "i" => &mut inf.present_participle,
            "3" => &mut inf.third_singular,
            "s" => &mut inf.plural,
            _ => continue,
        };
        *slot = Some(value.to_string());
        any = true;
    }
    if any {
        Some(inf)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_word_filters_shape() {
        assert!(is_clean_word("indicator"));
        assert!(is_clean_word("a"));
        assert!(is_clean_word("i"));
        assert!(!is_clean_word("e"));
        assert!(!is_clean_word("two words"));
        assert!(!is_clean_word("co-op"));
        assert!(!is_clean_word("3d"));
    }

    #[test]
    fn inflection_of_other_detected() {
        // running's lemma is run → dropped.
        assert!(is_inflection_of_other("0:run/1:i", "running"));
        // run is its own lemma → kept.
        assert!(!is_inflection_of_other("i:running/3:runs", "run"));
        assert!(!is_inflection_of_other("", "run"));
    }

    #[test]
    fn split_pos_extracts_known_pos() {
        assert_eq!(
            split_pos("n. 苹果, 家伙"),
            ("n.".to_string(), "苹果, 家伙".to_string())
        );
        assert_eq!(
            split_pos("vt. 申请"),
            ("vt.".to_string(), "申请".to_string())
        );
        // Domain-tag line has no POS prefix.
        assert_eq!(
            split_pos("[计] 指示器"),
            (String::new(), "[计] 指示器".to_string())
        );
    }

    #[test]
    fn parse_translation_builds_major_and_defs() {
        let (major, defs) = parse_translation("n. 指示器, 指示剂\n[计] 指示器");
        assert_eq!(major.as_deref(), Some("指示器, 指示剂"));
        let defs = defs.unwrap();
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].pos, "n.");
        assert_eq!(defs[0].meanings, vec!["指示器, 指示剂".to_string()]);
        assert_eq!(defs[1].pos, "");
    }

    #[test]
    fn parse_inflections_maps_exchange() {
        let inf = parse_inflections("d:perceived/p:perceived/3:perceives/i:perceiving").unwrap();
        assert_eq!(inf.past_tense.as_deref(), Some("perceived"));
        assert_eq!(inf.past_participle.as_deref(), Some("perceived"));
        assert_eq!(inf.third_singular.as_deref(), Some("perceives"));
        assert_eq!(inf.present_participle.as_deref(), Some("perceiving"));
        assert!(inf.plural.is_none());
        assert!(parse_inflections("").is_none());
    }
}
