//! Configuration loaded from `product.json`. Defaults are filled for any
//! missing field so a brand-new install (no config on disk) still works.
//!
//! The on-disk shape is the union of the typed `RawConfig` and `defaults`,
//! merged at parse time. Defaults match the values shipped in the repo's
//! `product.json`.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Errors surfaced by [`Config::load`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The config file could not be read from disk.
    #[error("config io error reading {path}: {source}")]
    Io {
        /// The path we tried to read.
        path: String,
        /// The underlying OS error.
        #[source]
        source: std::io::Error,
    },

    /// The config file was readable but its contents weren't valid JSON.
    #[error("config parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Read-only snapshot of `product.json`.
///
/// Construct via [`Config::load`] (file path) or [`Config::defaults`]
/// (in-memory defaults — useful in tests).
#[derive(Debug, Clone)]
pub struct Config {
    dict_data_path: PathBuf,
    dict_sqlite_path: Option<PathBuf>,
    dict_seed_on_first_open: bool,

    history_max_entries: usize,
    notes_persist: bool,
    lookup_prefer_notes_over_dict: bool,
}

impl Config {
    /// Load configuration from a `product.json` file. Missing fields fall back
    /// to the values returned by [`Config::defaults`].
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let bytes = std::fs::read(path_ref).map_err(|source| ConfigError::Io {
            path: path_ref.display().to_string(),
            source,
        })?;
        let raw: RawConfig = serde_json::from_slice(&bytes)?;
        Ok(raw.into_config())
    }

    /// Pure-Rust defaults — used in tests and as a fallback for missing JSON fields.
    pub fn defaults() -> Self {
        Self {
            dict_data_path: PathBuf::from("./Dict/data/seed_en_cn.json"),
            dict_sqlite_path: None,
            dict_seed_on_first_open: true,
            history_max_entries: 50,
            notes_persist: false,
            lookup_prefer_notes_over_dict: true,
        }
    }

    /// Path to the JSON seed file the dictionary loads from.
    pub fn dict_data_path(&self) -> &Path {
        &self.dict_data_path
    }

    /// Optional explicit sqlite path. When `None`, callers should pick a
    /// platform-conventional location (e.g. `%APPDATA%/EasyEnglish/dict.sqlite3`).
    pub fn dict_sqlite_path(&self) -> Option<&Path> {
        self.dict_sqlite_path.as_deref()
    }

    /// Should the sqlite file be auto-seeded on first open?
    pub fn dict_seed_on_first_open(&self) -> bool {
        self.dict_seed_on_first_open
    }

    /// Capacity of the runtime history ring.
    pub fn history_max_entries(&self) -> usize {
        self.history_max_entries
    }

    /// Phase 1 always returns `false`; Phase 2 may flip this on per-user request.
    pub fn notes_persist(&self) -> bool {
        self.notes_persist
    }

    /// True if `LookupService::query` should check user notes before falling
    /// back to the dictionary.
    pub fn lookup_prefer_notes_over_dict(&self) -> bool {
        self.lookup_prefer_notes_over_dict
    }
}

// ---- Internal JSON shape ---------------------------------------------------

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    dict: RawDict,
    #[serde(default)]
    core: RawCore,
}

#[derive(Debug, Default, Deserialize)]
struct RawDict {
    #[serde(default)]
    data_path: Option<PathBuf>,
    #[serde(default)]
    sqlite_path: Option<PathBuf>,
    #[serde(default)]
    seed_on_first_open: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct RawCore {
    #[serde(default)]
    history: RawHistory,
    #[serde(default)]
    notes: RawNotes,
    #[serde(default)]
    lookup: RawLookup,
}

#[derive(Debug, Default, Deserialize)]
struct RawHistory {
    #[serde(default)]
    max_entries: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct RawNotes {
    #[serde(default)]
    persist: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct RawLookup {
    #[serde(default)]
    prefer_notes_over_dict: Option<bool>,
}

impl RawConfig {
    fn into_config(self) -> Config {
        let defaults = Config::defaults();
        Config {
            dict_data_path: self.dict.data_path.unwrap_or(defaults.dict_data_path),
            dict_sqlite_path: self.dict.sqlite_path.or(defaults.dict_sqlite_path),
            dict_seed_on_first_open: self
                .dict
                .seed_on_first_open
                .unwrap_or(defaults.dict_seed_on_first_open),
            history_max_entries: self
                .core
                .history
                .max_entries
                .unwrap_or(defaults.history_max_entries),
            notes_persist: self.core.notes.persist.unwrap_or(defaults.notes_persist),
            lookup_prefer_notes_over_dict: self
                .core
                .lookup
                .prefer_notes_over_dict
                .unwrap_or(defaults.lookup_prefer_notes_over_dict),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_documentation() {
        let d = Config::defaults();
        assert_eq!(d.history_max_entries(), 50);
        assert!(!d.notes_persist());
        assert!(d.lookup_prefer_notes_over_dict());
        assert!(d.dict_seed_on_first_open());
        assert!(d.dict_sqlite_path().is_none());
    }
}
