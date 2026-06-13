//! `algo` — Core string distance, similarity metrics, and fuzzy ranking algorithms.

#![allow(dead_code)]

/// Standard Levenshtein distance using two rolling rows of dynamic programming.
///
/// This implementation is optimized for space efficiency and operates on UTF-8
/// characters (`char` unicode scalar values) to natively support multi-language strings.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let len_a = a_chars.len();
    let len_b = b_chars.len();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    // Allocate only what is needed for the smaller string to save heap space
    let (smaller, larger) = if len_a < len_b {
        (&a_chars, &b_chars)
    } else {
        (&b_chars, &a_chars)
    };

    let s_len = smaller.len();
    let l_len = larger.len();

    let mut prev_row: Vec<usize> = (0..=s_len).collect();
    let mut curr_row: Vec<usize> = vec![0; s_len + 1];

    for i in 1..=l_len {
        curr_row[0] = i;
        for j in 1..=s_len {
            let cost = if larger[i - 1] == smaller[j - 1] {
                0
            } else {
                1
            };
            curr_row[j] = std::cmp::min(
                curr_row[j - 1] + 1, // Insertion
                std::cmp::min(
                    prev_row[j] + 1,        // Deletion
                    prev_row[j - 1] + cost, // Substitution
                ),
            );
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[s_len]
}

/// Maximum edit distance at which a non-prefix candidate is still surfaced.
const MAX_SUGGEST_DISTANCE: usize = 3;

/// ASCII-case-insensitive `haystack.starts_with(prefix)`, allocation-free.
fn ascii_ci_starts_with(haystack: &str, prefix: &str) -> bool {
    let mut hay = haystack.chars();
    for pc in prefix.chars() {
        match hay.next() {
            Some(hc) if hc.eq_ignore_ascii_case(&pc) => {}
            _ => return false,
        }
    }
    true
}

/// Rank candidates by **exact then prefix** match only — no fuzzy edit distance.
///
/// Returns up to `max` candidates whose start matches `query` (ASCII
/// case-insensitively; non-ASCII such as CJK compares exactly), ordered with the
/// exact match first, then shortest (closest) prefixes, ties broken
/// alphabetically. Used for Chinese → English lookup where fuzzy matching is
/// undesirable.
pub fn prefix_candidates(query: &str, candidates: &[&str], max: usize) -> Vec<String> {
    if max == 0 || candidates.is_empty() {
        return Vec::new();
    }
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let q_len = query.chars().count();

    let mut scored: Vec<(usize, &str)> = Vec::new();
    for &c in candidates {
        let cand = c.trim();
        if ascii_ci_starts_with(cand, query) {
            // Extra length beyond the query: 0 for an exact match, larger for
            // longer completions, so exact and closest prefixes sort first.
            let extra = cand.chars().count().saturating_sub(q_len);
            scored.push((extra, c));
        }
    }
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    scored
        .into_iter()
        .take(max)
        .map(|(_, c)| c.to_string())
        .collect()
}

/// Bounded Levenshtein distance between the pre-lowercased query `q` and a
/// candidate string (lowercased on the fly, ASCII-aware), returning `None` as
/// soon as the distance is proven to exceed `max`.
///
/// `prev`/`curr` are reusable scratch rows (length `q.len() + 1`) so the hot
/// loop performs no per-candidate heap allocation. Early exit is sound because
/// the per-row minimum of the DP table is non-decreasing, so once it passes
/// `max` the final cell can only be larger.
fn bounded_distance_ci(
    q: &[char],
    candidate: &str,
    max: usize,
    prev: &mut Vec<usize>,
    curr: &mut Vec<usize>,
) -> Option<usize> {
    let qn = q.len();
    prev.clear();
    prev.extend(0..=qn);

    let mut rows = 0usize;
    for cc in candidate.chars() {
        rows += 1;
        let cc = cc.to_ascii_lowercase();
        curr.clear();
        curr.push(rows);
        let mut row_min = rows;
        for j in 1..=qn {
            let cost = if q[j - 1] == cc { 0 } else { 1 };
            let v = (curr[j - 1] + 1).min(prev[j] + 1).min(prev[j - 1] + cost);
            curr.push(v);
            if v < row_min {
                row_min = v;
            }
        }
        std::mem::swap(prev, curr);
        if row_min > max {
            return None;
        }
    }

    let dist = prev[qn];
    if dist <= max {
        Some(dist)
    } else {
        None
    }
}

/// Sort and rank a list of candidate strings against a query word.
///
/// Returns up to `max` candidate strings, ordered by ascending Levenshtein
/// distance, breaking ties alphabetically. A candidate is surfaced when it is
/// within [`MAX_SUGGEST_DISTANCE`] edits of the query, or when either string is
/// a prefix of the other.
///
/// The scan is tuned to stay responsive over large word lists (100k+): an exact
/// length-band pre-filter and an early-exiting bounded distance skip the
/// expensive full DP for the vast majority of candidates, and comparisons avoid
/// per-candidate allocation. Results are identical to a naive full scan.
pub fn rank_candidates(query: &str, candidates: &[&str], max: usize) -> Vec<String> {
    if max == 0 || candidates.is_empty() {
        return Vec::new();
    }

    let query_lower = query.trim().to_lowercase();
    let q_chars: Vec<char> = query_lower.chars().collect();
    let q_len = q_chars.len();

    let mut prev: Vec<usize> = Vec::with_capacity(q_len + 1);
    let mut curr: Vec<usize> = Vec::with_capacity(q_len + 1);

    let mut scored: Vec<(usize, &str)> = Vec::new();
    for &c in candidates {
        let candidate = c.trim();
        let is_prefix = ascii_ci_starts_with(candidate, &query_lower)
            || ascii_ci_starts_with(&query_lower, candidate);

        let score = if is_prefix {
            // Prefix matches are kept regardless of distance, but need the true
            // distance for correct ordering (e.g. "appl" → "application" = 7).
            levenshtein_distance(&query_lower, &candidate.to_ascii_lowercase())
        } else {
            // Exact length lower bound: distance >= |len(q) - len(c)|, so a
            // length gap beyond the threshold can never be within range.
            let c_len = candidate.chars().count();
            if q_len.abs_diff(c_len) > MAX_SUGGEST_DISTANCE {
                continue;
            }
            match bounded_distance_ci(
                &q_chars,
                candidate,
                MAX_SUGGEST_DISTANCE,
                &mut prev,
                &mut curr,
            ) {
                Some(d) => d,
                None => continue,
            }
        };

        scored.push((score, c));
    }

    // Sort by score ascending, and tie-break alphabetically
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));

    scored
        .into_iter()
        .take(max)
        .map(|(_, item)| item.to_string())
        .collect()
}
