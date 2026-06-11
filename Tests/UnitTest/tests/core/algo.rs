use ee_core::{levenshtein_distance, prefix_candidates, rank_candidates};

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
    assert_eq!(levenshtein_distance("苹果", "苹果"), 0);
    assert_eq!(levenshtein_distance("苹果", "香蕉"), 2);
    assert_eq!(levenshtein_distance("苹果树", "苹果"), 1);
}

#[test]
fn test_rank_candidates() {
    let candidates = vec!["apple", "apricot", "application"];
    let suggestions = rank_candidates("appl", &candidates, 3);

    assert_eq!(suggestions.len(), 2);
    assert_eq!(suggestions[0], "apple");
    assert_eq!(suggestions[1], "application");
}

fn naive_rank(query: &str, candidates: &[&str], max: usize) -> Vec<String> {
    if max == 0 || candidates.is_empty() {
        return Vec::new();
    }
    let q = query.trim().to_lowercase();
    let mut scored: Vec<(usize, String)> = candidates
        .iter()
        .filter_map(|&c| {
            let cl = c.trim().to_lowercase();
            let score = levenshtein_distance(&q, &cl);
            let is_prefix = cl.starts_with(&q) || q.starts_with(&cl);
            if score <= 3 || is_prefix {
                Some((score, c.to_string()))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().take(max).map(|(_, i)| i).collect()
}

#[test]
fn optimized_matches_naive_reference() {
    let candidates = [
        "apple",
        "apply",
        "applet",
        "application",
        "apricot",
        "ample",
        "app",
        "a",
        "zebra",
        "Apple",
        "APPLES",
        "snapple",
        "grapple",
        "maple",
        "pineapple",
        "indicator",
        "indicate",
        "gist",
        "ghost",
    ];
    for query in [
        "appl", "apple", "app", "", "xyz", "indicato", "gist", "grapple", "a",
    ] {
        for max in [1usize, 3, 5, 100] {
            assert_eq!(
                rank_candidates(query, &candidates, max),
                naive_rank(query, &candidates, max),
                "mismatch for query={query:?} max={max}"
            );
        }
    }
}

#[test]
fn length_band_keeps_exactly_distance_three() {
    let candidates = ["xbcde", "xbcdef"];
    assert!(rank_candidates("abc", &candidates, 5).contains(&"xbcde".to_string()));
    assert!(!rank_candidates("abc", &candidates, 5).contains(&"xbcdef".to_string()));
}

#[test]
fn prefix_beyond_threshold_is_surfaced() {
    let candidates = ["application", "apple"];
    let out = rank_candidates("appl", &candidates, 5);
    assert_eq!(out, vec!["apple".to_string(), "application".to_string()]);
}

#[test]
fn prefix_candidates_exact_then_prefix_no_fuzzy() {
    let candidates = ["苹果", "苹果酱", "苹果树", "香蕉", "苹"];
    let out = prefix_candidates("苹果", &candidates, 5);
    assert_eq!(out, vec!["苹果", "苹果树", "苹果酱"]);

    let out2 = prefix_candidates("苹", &candidates, 5);
    assert_eq!(out2, vec!["苹", "苹果", "苹果树", "苹果酱"]);

    assert!(prefix_candidates("苹菓", &candidates, 5).is_empty());
    assert!(prefix_candidates("", &candidates, 5).is_empty());
}
