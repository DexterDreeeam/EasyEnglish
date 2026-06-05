//! `algo` — Core string distance, similarity metrics, and fuzzy ranking algorithms.

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
            let cost = if larger[i - 1] == smaller[j - 1] { 0 } else { 1 };
            curr_row[j] = std::cmp::min(
                curr_row[j - 1] + 1, // Insertion
                std::cmp::min(
                    prev_row[j] + 1,      // Deletion
                    prev_row[j - 1] + cost, // Substitution
                ),
            );
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[s_len]
}

/// Sort and rank a list of candidate strings against a query word.
///
/// Returns up to `max` candidate strings, ordered by ascending Levenshtein distance,
/// breaking ties alphabetically.
pub fn rank_candidates(query: &str, candidates: &[&str], max: usize) -> Vec<String> {
    if max == 0 || candidates.is_empty() {
        return Vec::new();
    }

    let query_lower = query.trim().to_lowercase();
    let mut scored: Vec<(usize, String)> = candidates
        .iter()
        .map(|&c| {
            let c_lower = c.trim().to_lowercase();
            let score = levenshtein_distance(&query_lower, &c_lower);
            (score, c.to_string())
        })
        .collect();

    // Sort by score ascending, and tie-break alphabetically
    scored.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1))
    });

    scored
        .into_iter()
        .take(max)
        .map(|(_, item)| item)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_basic() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("apple", ""), 5);
        assert_eq!(levenshtein_distance("", "apple"), 5);
        assert_eq!(levenshtein_distance("apple", "apple"), 0);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_levenshtein_unicode() {
        // Multi-byte Chinese character edits
        assert_eq!(levenshtein_distance("苹果", "苹果"), 0);
        assert_eq!(levenshtein_distance("苹果", "香蕉"), 2);
        assert_eq!(levenshtein_distance("苹果树", "苹果"), 1);
    }

    #[test]
    fn test_rank_candidates() {
        let candidates = vec!["apple", "apricot", "application"];
        let suggestions = rank_candidates("appl", &candidates, 3);
        
        // "apple" is distance 1 from "appl", "apricot" is distance 4, "application" is distance 7
        assert_eq!(suggestions.len(), 3);
        assert_eq!(suggestions[0], "apple");
        assert_eq!(suggestions[1], "apricot");
        assert_eq!(suggestions[2], "application");
    }
}
