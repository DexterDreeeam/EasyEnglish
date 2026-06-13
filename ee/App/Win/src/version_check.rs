//! One-shot update check against the packaged and remote `version` files.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// Raw GitHub URL for the latest packaged version marker.
pub(crate) const REMOTE_VERSION_URL: &str =
    "https://raw.githubusercontent.com/DexterDreeeam/EasyEnglish/main/ee/version";

/// Result of the one-shot version check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VersionCheckResult {
    /// Local and remote versions match.
    Current,
    /// Remote version differs from the packaged local version.
    UpdateAvailable { local: String, remote: String },
    /// The check could not complete. This should be logged, not shown as a toast.
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

/// Start the check on a background thread and return a receiver for polling.
pub(crate) fn start_version_check() -> Receiver<VersionCheckResult> {
    let (tx, rx) = mpsc::channel();
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
    std::fs::read_to_string(&path).map_err(|e| format!("failed to read {:?}: {e}", path))
}

fn fetch_remote_version() -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();
    agent
        .get(REMOTE_VERSION_URL)
        .call()
        .map_err(|e| format!("remote version request failed: {e}"))?
        .into_string()
        .map_err(|e| format!("remote version body failed: {e}"))
}

fn find_version_file() -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        let direct = cwd.join("version");
        if direct.exists() {
            return Some(direct);
        }
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(found) = walk_up_for_version(&exe_path) {
            return Some(found);
        }
    }
    None
}

fn walk_up_for_version(start: &Path) -> Option<PathBuf> {
    let mut path = start.to_path_buf();
    if path.is_file() {
        path.pop();
    }
    for _ in 0..6 {
        let candidate = path.join("version");
        if candidate.exists() {
            return Some(candidate);
        }
        if !path.pop() {
            break;
        }
    }
    None
}
