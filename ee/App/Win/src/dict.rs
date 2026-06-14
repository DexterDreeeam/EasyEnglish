//! Dictionary database + word-list discovery (highest version selection).

use crate::logging::log_message;
use std::path::PathBuf;

const DEFAULT_ENGLISH_PREFIX: &str = "word_en_cn";
const DEFAULT_TARGET_PREFIX: &str = "word_cn";
const DICTIONARY_PACKAGE_CONFIG: &str = "dictionary-package.ini";

/// Dictionary package prefixes selected by the installed language package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DictionaryPackageConfig {
    /// English-to-target dictionary prefix, without `_vN`.
    pub(crate) english_prefix: String,
    /// Target-to-English dictionary prefix, without `_vN`.
    pub(crate) target_prefix: String,
}

impl Default for DictionaryPackageConfig {
    fn default() -> Self {
        Self {
            english_prefix: DEFAULT_ENGLISH_PREFIX.to_string(),
            target_prefix: DEFAULT_TARGET_PREFIX.to_string(),
        }
    }
}

/// Load the installed package's dictionary prefix configuration.
pub(crate) fn load_dictionary_package_config() -> DictionaryPackageConfig {
    let path = get_db_path(DICTIONARY_PACKAGE_CONFIG);
    if !path.is_file() {
        return DictionaryPackageConfig::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(raw) => match parse_dictionary_package_config(&raw) {
            Some(config) => config,
            None => {
                log_message(&format!(
                    "[DictConfig] Invalid dictionary config at {:?}; using defaults.",
                    path
                ));
                DictionaryPackageConfig::default()
            }
        },
        Err(err) => {
            log_message(&format!(
                "[DictConfig] Failed to read dictionary config at {:?}: {}; using defaults.",
                path, err
            ));
            DictionaryPackageConfig::default()
        }
    }
}

/// Parse the installed package dictionary configuration.
pub(crate) fn parse_dictionary_package_config(raw: &str) -> Option<DictionaryPackageConfig> {
    let mut english_prefix = None;
    let mut target_prefix = None;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') || line.starts_with('[')
        {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() || !valid_dictionary_prefix(value) {
            return None;
        }
        match key {
            "EnglishPrefix" => english_prefix = Some(value.to_string()),
            "TargetPrefix" => target_prefix = Some(value.to_string()),
            _ => {}
        }
    }

    Some(DictionaryPackageConfig {
        english_prefix: english_prefix?,
        target_prefix: target_prefix?,
    })
}

fn valid_dictionary_prefix(value: &str) -> bool {
    value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub(crate) fn scan_for_highest_db_version(prefix: &str) -> Option<PathBuf> {
    let dict_dir = get_db_path(""); // Get Dict/ folder
    log_message(&format!(
        "[Scan] Scanning directory for {} db: {:?}",
        prefix, dict_dir
    ));
    let mut highest_version = 0;
    let mut highest_path = None;

    let db_prefix = format!("{}_v", prefix);
    if let Ok(entries) = std::fs::read_dir(&dict_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if let Some(rest) = filename.strip_prefix(&db_prefix) {
                    if let Some(version_part) = rest.strip_suffix(".sqlite") {
                        if let Ok(v) = version_part.parse::<usize>() {
                            log_message(&format!("[Scan] Found database: {} (v{})", filename, v));
                            if v > highest_version {
                                highest_version = v;
                                highest_path = Some(path);
                            }
                        }
                    }
                }
            }
        }
    }
    log_message(&format!(
        "[Scan] Selected highest database: {:?}",
        highest_path
    ));
    highest_path
}

pub(crate) fn load_highest_version_word_list(prefix: &str) -> Vec<String> {
    let dict_dir = get_db_path(""); // Get Dict/ folder
    log_message(&format!(
        "[List] Scanning directory for {} word list: {:?}",
        prefix, dict_dir
    ));
    let mut highest_version = 0;
    let mut highest_file = None;

    let list_prefix = format!("{}_v", prefix);
    if let Ok(entries) = std::fs::read_dir(&dict_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                // The word list shares the `{prefix}_v{N}` base name with the
                // database but carries no extension; the `.sqlite` exclusion keeps
                // the two apart.
                if filename.ends_with(".sqlite") {
                    continue;
                }
                if let Some(version_part) = filename.strip_prefix(&list_prefix) {
                    if let Ok(v) = version_part.parse::<usize>() {
                        log_message(&format!("[List] Found word list: {} (v{})", filename, v));
                        if v > highest_version {
                            highest_version = v;
                            highest_file = Some(path);
                        }
                    }
                }
            }
        }
    }

    if let Some(path) = highest_file {
        log_message(&format!("[List] Loading selected word list: {:?}", path));
        if let Ok(file) = std::fs::File::open(&path) {
            let reader = std::io::BufReader::new(file);
            use std::io::BufRead;
            let list: Vec<String> = reader.lines().map_while(Result::ok).collect();
            log_message(&format!("[List] Loaded {} words successfully.", list.len()));
            return list;
        }
    }
    log_message("[List] No word list loaded!");
    Vec::new()
}

fn get_db_path(filename: &str) -> PathBuf {
    let path = std::env::current_dir()
        .unwrap_or_default()
        .join("Dict")
        .join(filename);
    if path.exists() {
        return path;
    }
    if let Ok(exe_path) = std::env::current_exe() {
        let mut p = exe_path;
        for _ in 0..5 {
            if let Some(parent) = p.parent() {
                p = parent.to_path_buf();
                let possible = p.join("Dict").join(filename);
                if possible.exists() {
                    return possible;
                }
            }
        }
    }
    PathBuf::from("Dict").join(filename)
}
