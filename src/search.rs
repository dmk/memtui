//! Fuzzy search functionality using nucleo-matcher

use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};

/// Performs fuzzy search on a list of key names, returning indices of matches
/// sorted by match quality (best matches first)
pub fn fuzzy_search_keys(keys: &[Option<crate::types::KeyMetadata>], pattern: &str) -> Vec<usize> {
    if pattern.is_empty() {
        // Empty pattern matches everything - return all indices with loaded keys
        return keys
            .iter()
            .enumerate()
            .filter_map(|(i, k)| k.as_ref().map(|_| i))
            .collect();
    }

    let mut matcher = Matcher::new(Config::DEFAULT);
    let pat = Pattern::parse(pattern, CaseMatching::Ignore, Normalization::Smart);

    let mut scored_matches: Vec<(usize, u32)> = keys
        .iter()
        .enumerate()
        .filter_map(|(idx, key_opt)| {
            key_opt.as_ref().and_then(|key| {
                let mut buf = Vec::new();
                pat.score(nucleo_matcher::Utf32Str::new(&key.name, &mut buf), &mut matcher)
                    .map(|score| (idx, score))
            })
        })
        .collect();

    // Sort by score descending (higher score = better match)
    scored_matches.sort_by(|a, b| b.1.cmp(&a.1));

    scored_matches.into_iter().map(|(idx, _)| idx).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{KeyMetadata, ValueType};
    use std::time::Duration;

    fn make_key(name: &str) -> Option<KeyMetadata> {
        Some(KeyMetadata {
            name: name.to_string(),
            value_type: ValueType::String,
            size_bytes: 0,
            ttl: Some(Duration::from_secs(3600)),
            last_accessed: None,
            encoding: None,
        })
    }

    #[test]
    fn test_fuzzy_search_empty_pattern() {
        let keys = vec![make_key("user:123"), make_key("user:456"), None, make_key("session:abc")];
        let results = fuzzy_search_keys(&keys, "");
        assert_eq!(results, vec![0, 1, 3]); // All loaded keys
    }

    #[test]
    fn test_fuzzy_search_exact_match() {
        let keys = vec![make_key("user:123"), make_key("user:456"), make_key("session:abc")];
        let results = fuzzy_search_keys(&keys, "user:123");
        assert!(!results.is_empty());
        assert_eq!(results[0], 0); // Exact match should be first
    }

    #[test]
    fn test_fuzzy_search_partial() {
        let keys = vec![make_key("user:123"), make_key("user:456"), make_key("session:abc")];
        let results = fuzzy_search_keys(&keys, "user");
        assert_eq!(results.len(), 2); // Should match both user keys
    }

    #[test]
    fn test_fuzzy_search_no_match() {
        let keys = vec![make_key("user:123"), make_key("user:456"), make_key("session:abc")];
        let results = fuzzy_search_keys(&keys, "zzzzz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_fuzzy_search_case_insensitive() {
        let keys = vec![make_key("User:123"), make_key("USER:456"), make_key("session:abc")];
        let results = fuzzy_search_keys(&keys, "user");
        assert_eq!(results.len(), 2);
    }
}

