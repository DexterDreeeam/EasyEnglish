//! One-shot update check against the packaged and remote `version` files.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

/// Raw GitHub URL for the latest packaged version marker.
const REMOTE_VERSION_URL: &str =
    "https://raw.githubusercontent.com/DexterDreeeam/EasyEnglish/main/ee/version";

/// Result of the one-shot version check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VersionCheckResult {
    /// Local and remote versions match.
    Current,
    /// Remote version differs from the packaged local version.
    UpdateAvailable { local: String, remote: String },
    /// The check could not complete. This should be logged only.
    Failed(String),
}

/// Compare two version strings after trimming whitespace.
pub(crate) fn compare_versions(local: &str, remote: &str) -> VersionCheckResult {
    let local = local.trim();
    let remote = remote.trim();
    if local.is_empty() {
        VersionCheckResult::Failed("local version is empty".to_string())
    } else if remote.is_empty() {
        VersionCheckResult::Failed("remote version is empty".to_string())
    } else if local == remote {
        VersionCheckResult::Current
    } else {
        VersionCheckResult::UpdateAvailable {
            local: local.to_string(),
            remote: remote.to_string(),
        }
    }
}

/// Start the version check on a background thread.
pub(crate) fn start_version_check() -> Receiver<VersionCheckResult> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let result = run_version_check();
        let _ = tx.send(result);
    });
    rx
}

fn run_version_check() -> VersionCheckResult {
    match (read_local_version(), fetch_remote_version()) {
        (Ok(local), Ok(remote)) => compare_versions(&local, &remote),
        (Err(err), _) => VersionCheckResult::Failed(err),
        (_, Err(err)) => VersionCheckResult::Failed(err),
    }
}

fn read_local_version() -> Result<String, String> {
    let path = find_version_file().ok_or_else(|| "version file not found".to_string())?;
    std::fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {e}", path.display()))
}

fn fetch_remote_version() -> Result<String, String> {
    ureq::get(REMOTE_VERSION_URL)
        .call()
        .map_err(|e| format!("remote version request failed: {e}"))?
        .into_string()
        .map_err(|e| format!("remote version body failed: {e}"))
}

fn find_version_file() -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        let direct = cwd.join("version");
        if direct.is_file() {
            return Some(direct);
        }
    }

    std::env::current_exe()
        .ok()
        .and_then(|exe_path| walk_up_for_version(&exe_path))
}

fn walk_up_for_version(start: &Path) -> Option<PathBuf> {
    let mut path = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let candidate = path.join("version");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !path.pop() {
            return None;
        }
    }
}
