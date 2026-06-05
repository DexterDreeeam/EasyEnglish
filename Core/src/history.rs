//! `HistoryStore` — bounded runtime ring of recent successful lookups.
//!
//! Phase 1 stores entries in memory only; the ring is reset on every launch.
//! Persistence is a Phase 2 concern (planned via `core.history.persist` in
//! `product.json`).

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// One recorded lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// The word the user looked up (lower-cased canonical form).
    pub word: String,
    /// Wall-clock unix-seconds when the entry was recorded.
    pub at_unix_secs: i64,
}

/// Bounded VecDeque of recent entries, most-recent at the front.
///
/// `record` deduplicates: a freshly-recorded word that already exists is moved
/// to the front and its timestamp is updated.
#[derive(Debug)]
pub struct HistoryStore {
    cap: usize,
    items: VecDeque<HistoryEntry>,
    clock: fn() -> i64,
}

impl HistoryStore {
    /// Construct with a maximum capacity (from `Config::history_max_entries`).
    /// A capacity of 0 is allowed; `record` becomes a no-op.
    pub fn with_capacity(max: usize) -> Self {
        Self {
            cap: max,
            items: VecDeque::with_capacity(max),
            clock: default_clock,
        }
    }

    /// Test-only constructor: inject a deterministic clock instead of wall time.
    #[doc(hidden)]
    pub fn with_clock(max: usize, clock: fn() -> i64) -> Self {
        Self {
            cap: max,
            items: VecDeque::with_capacity(max),
            clock,
        }
    }

    /// Record a successful lookup. Empty input is silently dropped.
    pub fn record(&mut self, word: &str) {
        let trimmed = word.trim().to_lowercase();
        if trimmed.is_empty() || self.cap == 0 {
            return;
        }
        let ts = (self.clock)();
        // Drop any existing entry for the same word — we'll re-push to the front.
        if let Some(pos) = self.items.iter().position(|e| e.word == trimmed) {
            self.items.remove(pos);
        }
        self.items.push_front(HistoryEntry {
            word: trimmed,
            at_unix_secs: ts,
        });
        while self.items.len() > self.cap {
            self.items.pop_back();
        }
    }

    /// Most recent first.
    pub fn recent(&self) -> Vec<HistoryEntry> {
        self.items.iter().cloned().collect()
    }

    /// Drop every entry.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Number of recorded entries.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True if no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

fn default_clock() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
