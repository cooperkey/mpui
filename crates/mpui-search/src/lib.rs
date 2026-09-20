pub mod engine;

use nucleo::pattern::{CaseMatching, Normalization, Pattern};
use nucleo::{Config as NucleoConfig, Matcher};

/// A single search result with its score and original index.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub index: usize,
    pub score: u32,
}

/// Fuzzy search engine wrapping nucleo.
pub struct SearchEngine {
    matcher: Matcher,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(NucleoConfig::DEFAULT),
        }
    }

    /// Search `haystack` items against `query`. Returns matching indices sorted by score (best first).
    pub fn search<'a>(
        &mut self,
        query: &str,
        haystack: impl Iterator<Item = &'a str>,
    ) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
        let mut results: Vec<SearchResult> = haystack
            .enumerate()
            .filter_map(|(i, item)| {
                let haystack_str = nucleo::Utf32Str::Ascii(item.as_bytes());
                pattern
                    .score(haystack_str, &mut self.matcher)
                    .map(|score| SearchResult { index: i, score })
            })
            .collect();

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
