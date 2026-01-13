//! Fuzzy search implementation for Svelte documentation
//!
//! Uses nucleo-matcher for fast, high-quality fuzzy matching.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::Serialize;

use super::docs_index::{extract_code_blocks, IndexEntry, SearchIndex};

/// A search result with relevance score
#[derive(Debug, Serialize)]
pub struct DocResult {
    /// Display title
    pub title: String,
    /// Section name
    pub section: String,
    /// Relevance score (0.0 - 1.0)
    pub relevance_score: f32,
    /// Brief summary
    pub summary: String,
    /// Full markdown content
    pub content: String,
    /// Extracted code examples
    pub code_examples: Vec<String>,
}

/// Output structure for the search tool
#[derive(Debug, Serialize)]
pub struct DocSearchOutput {
    /// The original query
    pub query: String,
    /// Search results ordered by relevance
    pub results: Vec<DocResult>,
    /// Total number of matches found
    pub total_matches: usize,
}

/// Search the documentation index for matching entries
pub fn search_docs(index: &SearchIndex, query: &str, limit: usize) -> Vec<DocResult> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let query_lower = query.to_lowercase();

    // Score each entry
    let mut scored_results: Vec<(u32, &IndexEntry)> = index
        .entries
        .iter()
        .filter_map(|entry| {
            let score = calculate_entry_score(&mut matcher, &pattern, entry, &query_lower);
            if score > 0 {
                Some((score, entry))
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending
    scored_results.sort_by(|a, b| b.0.cmp(&a.0));

    // Take top N and convert to output format
    scored_results
        .into_iter()
        .take(limit)
        .map(|(score, entry)| DocResult {
            title: entry.title.clone(),
            section: entry.section.clone(),
            relevance_score: normalize_score(score),
            summary: entry.summary.clone(),
            content: entry.content.clone(),
            code_examples: extract_code_blocks(&entry.content),
        })
        .collect()
}

/// Calculate a relevance score for an entry
fn calculate_entry_score(matcher: &mut Matcher, pattern: &Pattern, entry: &IndexEntry, query_lower: &str) -> u32 {
    let mut best_score: u32 = 0;

    // Score title (highest weight)
    if let Some(score) = score_string(matcher, pattern, &entry.title) {
        best_score = best_score.max(score * 3);
    }

    // Score keywords (high weight)
    for keyword in &entry.keywords {
        if let Some(score) = score_string(matcher, pattern, keyword) {
            best_score = best_score.max(score * 2);
        }
    }

    // Score ID (medium weight)
    if let Some(score) = score_string(matcher, pattern, &entry.id) {
        best_score = best_score.max(score + score / 2);
    }

    // Score summary (lower weight)
    if let Some(score) = score_string(matcher, pattern, &entry.summary) {
        best_score = best_score.max(score);
    }

    // Bonus for exact keyword match
    if entry.keywords.iter().any(|k| k == query_lower) {
        best_score = best_score.saturating_add(500);
    }

    // Bonus for title containing query
    if entry.title.to_lowercase().contains(query_lower) {
        best_score = best_score.saturating_add(300);
    }

    best_score
}

/// Score a single string against the pattern
fn score_string(matcher: &mut Matcher, pattern: &Pattern, text: &str) -> Option<u32> {
    let mut buf = Vec::new();
    let utf32 = Utf32Str::new(text, &mut buf);
    pattern.score(utf32, matcher)
}

/// Normalize score to 0.0 - 1.0 range
fn normalize_score(score: u32) -> f32 {
    // Typical scores range from 0 to ~1500 for good matches
    // Normalize to a reasonable 0-1 range
    (score as f32 / 1500.0).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_index() -> SearchIndex {
        SearchIndex {
            version: "5.0.0".to_string(),
            entries: vec![
                IndexEntry {
                    id: "runes/state".to_string(),
                    title: "$state".to_string(),
                    section: "Runes".to_string(),
                    path: "02-runes/02-state.md".to_string(),
                    summary: "The $state rune declares reactive state.".to_string(),
                    keywords: vec!["$state".to_string(), "reactive".to_string(), "runes".to_string()],
                    content: "# $state\n\nThe $state rune is used to declare reactive state.\n\n```svelte\nlet count = $state(0);\n```".to_string(),
                },
                IndexEntry {
                    id: "runes/derived".to_string(),
                    title: "$derived".to_string(),
                    section: "Runes".to_string(),
                    path: "02-runes/03-derived.md".to_string(),
                    summary: "The $derived rune creates computed values.".to_string(),
                    keywords: vec!["$derived".to_string(), "computed".to_string(), "runes".to_string()],
                    content: "# $derived\n\nUse $derived for computed values.".to_string(),
                },
                IndexEntry {
                    id: "runes/props".to_string(),
                    title: "$props".to_string(),
                    section: "Runes".to_string(),
                    path: "02-runes/05-props.md".to_string(),
                    summary: "The $props rune declares component props.".to_string(),
                    keywords: vec!["$props".to_string(), "props".to_string(), "runes".to_string()],
                    content: "# $props\n\nDeclare props with $props().".to_string(),
                },
            ],
        }
    }

    #[test]
    fn test_search_exact_match() {
        let index = create_test_index();
        let results = search_docs(&index, "$state", 3);

        assert!(!results.is_empty());
        assert_eq!(results[0].title, "$state");
    }

    #[test]
    fn test_search_fuzzy_match() {
        let index = create_test_index();
        let results = search_docs(&index, "reactive", 3);

        assert!(!results.is_empty());
        // Should find $state because "reactive" is in its keywords and summary
        assert!(results.iter().any(|r| r.title == "$state"));
    }

    #[test]
    fn test_search_no_match() {
        let index = create_test_index();
        let results = search_docs(&index, "xyznonexistent", 3);

        assert!(results.is_empty());
    }
}
