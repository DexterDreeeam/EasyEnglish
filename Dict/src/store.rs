//! `DictStore` — open / seed / query a SQLite-backed offline dictionary.
//!
//! The store is single-writer-friendly (we only ever bulk-insert during seeding)
//! and supports concurrent reads via `lookup`. Suggestion search runs entirely
//! over an in-memory cache built at open time, so it never touches sqlite.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OpenFlags};
use serde::Deserialize;
use serde_json::Value;

use crate::entry::{DictError, Entry};

/// Maximum length we accept for a lookup query, in bytes. Longer inputs are a
/// user mistake — there are no English words of this length — so we reject
/// them as `InvalidInput` instead of stressing sqlite.
const MAX_WORD_LEN: usize = 128;

/// SQL DDL executed on every open. `IF NOT EXISTS` makes this idempotent.
const SCHEMA_SQL: &str = "\
    CREATE TABLE IF NOT EXISTS entries (\
        headword     TEXT PRIMARY KEY COLLATE NOCASE,\
        phonetic     TEXT NOT NULL DEFAULT '',\
        definitions  TEXT NOT NULL\
    );\
    CREATE INDEX IF NOT EXISTS idx_entries_headword_nocase \
        ON entries(headword COLLATE NOCASE);\
";

/// One record in the seed JSON file. Permissive: optional / missing fields
/// default to empty so a sloppy seed entry can't break the whole load.
#[derive(Debug, Deserialize)]
struct SeedRecord {
    headword: String,
    #[serde(default)]
    phonetic: String,
    #[serde(default)]
    definitions: Vec<String>,
}

/// Opaque handle to an open dictionary database.
///
/// Constructed via [`DictStore::open`] (must already exist) or
/// [`DictStore::create_or_seed`] (will populate from JSON on first use).
/// `Send` so it can be moved to a worker thread; **not** `Sync` because
/// `rusqlite::Connection` isn't — wrap in `Mutex` for shared access.
pub struct DictStore {
    /// `Mutex` so we can keep a prepared statement cached across calls without
    /// requiring the user to take their own lock. Contended access is rare in
    /// the overlay use-case (one user typing into one input box).
    conn: Mutex<Connection>,

    /// Lower-cased headwords, sorted, used by `suggest` for Levenshtein search.
    /// Built once at open; never mutated after.
    headwords_lower: Vec<String>,
}

impl std::fmt::Debug for DictStore {
    /// Manual `Debug` impl: `rusqlite::Connection` isn't `Debug`, but tests
    /// (and `Result::expect_err`) need *something*. We log the cached size
    /// only — the underlying connection is opaque.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DictStore")
            .field("entries", &self.headwords_lower.len())
            .finish()
    }
}

impl DictStore {
    /// Open an existing dictionary database. Fails with `DictError::Storage`
    /// if the file is missing or unreadable.
    pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Self, DictError> {
        let conn = Connection::open_with_flags(
            db_path.as_ref(),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI,
        )?;
        Self::finalize_open(conn)
    }

    /// Open the database if it exists, otherwise create it and bulk-load the
    /// seed JSON. Idempotent on subsequent calls — if the table already has
    /// rows, the seed step is skipped.
    pub fn create_or_seed<P: AsRef<Path>, S: AsRef<Path>>(
        db_path: P,
        seed_path: S,
    ) -> Result<Self, DictError> {
        let conn = Connection::open(db_path.as_ref())?;
        conn.execute_batch(SCHEMA_SQL)?;

        let needs_seed: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
            .unwrap_or(0);

        if needs_seed == 0 {
            let bytes = std::fs::read(seed_path.as_ref())
                .map_err(|e| DictError::Seed(format!("read {:?}: {e}", seed_path.as_ref())))?;
            let records: Vec<SeedRecord> = serde_json::from_slice(&bytes)
                .map_err(|e| DictError::Seed(format!("parse json: {e}")))?;

            let tx = conn.unchecked_transaction()?;
            {
                let mut insert = tx.prepare(
                    "INSERT OR REPLACE INTO entries(headword, phonetic, definitions) \
                     VALUES(?1, ?2, ?3)",
                )?;
                for rec in &records {
                    // Skip empties so a stray blank row in the seed can't pollute the DB.
                    if rec.headword.trim().is_empty() || rec.definitions.is_empty() {
                        continue;
                    }
                    let head = rec.headword.to_lowercase();
                    let defs_json = serde_json::to_string(&rec.definitions)
                        .map_err(|e| DictError::Seed(format!("serialize defs: {e}")))?;
                    insert.execute(params![head, rec.phonetic, defs_json])?;
                }
            }
            tx.commit()?;
        }

        Self::finalize_open(conn)
    }

    /// Exact case-insensitive lookup. Returns the canonical entry on hit,
    /// `DictError::NotFound` on miss, `DictError::InvalidInput` for empty /
    /// oversized input.
    pub fn lookup(&self, word: &str) -> Result<Entry, DictError> {
        let trimmed = word.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_WORD_LEN {
            return Err(DictError::InvalidInput);
        }

        let conn = self.conn.lock().expect("dict mutex poisoned");
        let mut stmt = conn.prepare_cached(
            "SELECT headword, phonetic, definitions \
             FROM entries WHERE headword = ?1 COLLATE NOCASE LIMIT 1",
        )?;

        let row = stmt.query_row(params![trimmed], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        });

        match row {
            Ok((headword, phonetic, defs_json)) => {
                let definitions = parse_definitions(&defs_json);
                if definitions.is_empty() {
                    // A row exists but its definitions blob is empty / corrupt;
                    // surface as NotFound so callers don't show a blank result.
                    return Err(DictError::NotFound);
                }
                Ok(Entry {
                    headword,
                    phonetic,
                    definitions,
                })
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(DictError::NotFound),
            Err(e) => Err(DictError::Storage(e)),
        }
    }

    /// Suggestions ordered by ascending Levenshtein distance, alphabetical
    /// tiebreak. Empty / oversized input returns an empty vector — never errors.
    pub fn suggest(&self, prefix: &str, max: usize) -> Vec<String> {
        let trimmed = prefix.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_WORD_LEN || max == 0 {
            return Vec::new();
        }
        let query = trimmed.to_lowercase();

        // Score (distance, idx) for each headword; stable_sort by distance then
        // alphabetical (we walk `headwords_lower` in sorted order so equal
        // distances retain alphabetical order).
        let mut scored: Vec<(usize, usize)> = self
            .headwords_lower
            .iter()
            .enumerate()
            .map(|(i, hw)| (levenshtein(&query, hw), i))
            .collect();
        scored.sort_by_key(|&(d, _)| d);

        scored
            .into_iter()
            .take(max)
            .map(|(_, i)| self.headwords_lower[i].clone())
            .collect()
    }

    /// Number of distinct headwords loaded in the cache. Useful for tests.
    pub fn len(&self) -> usize {
        self.headwords_lower.len()
    }

    /// True if the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.headwords_lower.is_empty()
    }

    /// Common finalization shared by `open` and `create_or_seed`: ensure the
    /// schema exists, build the in-memory headword cache.
    fn finalize_open(conn: Connection) -> Result<Self, DictError> {
        conn.execute_batch(SCHEMA_SQL)?;
        let mut stmt =
            conn.prepare("SELECT headword FROM entries ORDER BY headword COLLATE NOCASE")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut headwords_lower = Vec::new();
        for r in rows {
            headwords_lower.push(r?.to_lowercase());
        }
        drop(stmt);
        Ok(Self {
            conn: Mutex::new(conn),
            headwords_lower,
        })
    }
}

/// Parse the `definitions` JSON blob back into a `Vec<String>`. A malformed
/// blob results in an empty vec (lookup will treat that as a miss).
fn parse_definitions(json: &str) -> Vec<String> {
    let value: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str().map(|s| s.to_owned()))
        .collect()
}

/// Standard Levenshtein distance with two rolling rows. Operates on bytes,
/// which is fine for the ASCII English headwords in our dictionary.
fn levenshtein(a: &str, b: &str) -> usize {
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let n = b_bytes.len();
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr: Vec<usize> = vec![0; n + 1];
    for (i, &ac) in a_bytes.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &bc) in b_bytes.iter().enumerate() {
            let cost = if ac == bc { 0 } else { 1 };
            let del = prev[j + 1] + 1;
            let ins = curr[j] + 1;
            let sub = prev[j] + cost;
            curr[j + 1] = del.min(ins).min(sub);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_empty_inputs() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("apple", ""), 5);
        assert_eq!(levenshtein("", "apple"), 5);
    }

    #[test]
    fn levenshtein_basic_distances() {
        assert_eq!(levenshtein("apple", "apple"), 0);
        assert_eq!(levenshtein("apple", "appl"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn parse_definitions_handles_empty_and_bad_json() {
        assert!(parse_definitions("").is_empty());
        assert!(parse_definitions("not json").is_empty());
        assert!(parse_definitions("{\"k\":\"v\"}").is_empty());
        assert_eq!(parse_definitions(r#"["a","b"]"#), vec!["a", "b"]);
    }
}
