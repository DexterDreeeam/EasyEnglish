//! Dictionary database + word-list discovery (highest version selection).

use crate::logging::log_message;
use std::path::PathBuf;

pub(crate) fn scan_for_highest_db_version() -> Option<PathBuf> {
    let dict_dir = get_db_path(""); // Get Dict/ folder
    log_message(&format!("[Scan] Scanning directory: {:?}", dict_dir));
    let mut highest_version = 0;
    let mut highest_path = None;

    if let Ok(entries) = std::fs::read_dir(&dict_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if filename.starts_with("word_en_v") && filename.ends_with(".sqlite") {
                    let version_part =
                        &filename["word_en_v".len()..(filename.len() - ".sqlite".len())];
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
    log_message(&format!(
        "[Scan] Selected highest database: {:?}",
        highest_path
    ));
    highest_path
}

pub(crate) fn load_highest_version_word_list() -> Vec<String> {
    let dict_dir = get_db_path(""); // Get Dict/ folder
    log_message(&format!(
        "[List] Scanning directory for word list: {:?}",
        dict_dir
    ));
    let mut highest_version = 0;
    let mut highest_file = None;

    if let Ok(entries) = std::fs::read_dir(&dict_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                // The word list shares the `word_en_v{N}` base name with the
                // database but carries no extension; the `.sqlite` exclusion keeps
                // the two apart.
                if filename.ends_with(".sqlite") {
                    continue;
                }
                if let Some(version_part) = filename.strip_prefix("word_en_v") {
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
