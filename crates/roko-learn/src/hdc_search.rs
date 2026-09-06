//! P4-02: HDC fingerprint similarity search.
//!
//! Provides search functionality over episode HDC fingerprints.
//! Given a query fingerprint, finds the most similar episodes
//! using Hamming distance.

use roko_primitives::hdc::HdcVector;
use serde::{Deserialize, Serialize};

/// A search result from HDC similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdcSearchResult {
    /// Episode or entry identifier.
    pub id: String,
    /// Similarity score (0.0 to 1.0).
    pub similarity: f32,
    /// Optional metadata about the matched entry.
    #[serde(default)]
    pub metadata: Option<String>,
}

/// Search a collection of episodes for those with fingerprints most similar
/// to the query.
///
/// Returns the top-k results sorted by similarity descending.
pub fn search_episodes(
    query: &HdcVector,
    episodes: &[(String, HdcVector)],
    top_k: usize,
) -> Vec<HdcSearchResult> {
    let mut results: Vec<HdcSearchResult> = episodes
        .iter()
        .map(|(id, fp)| HdcSearchResult {
            id: id.clone(),
            similarity: query.similarity(fp),
            metadata: None,
        })
        .collect();

    results.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));
    results.truncate(top_k);
    results
}

/// Search with a minimum similarity threshold.
pub fn search_with_threshold(
    query: &HdcVector,
    episodes: &[(String, HdcVector)],
    threshold: f32,
    top_k: usize,
) -> Vec<HdcSearchResult> {
    let mut results: Vec<HdcSearchResult> = episodes
        .iter()
        .filter_map(|(id, fp)| {
            let sim = query.similarity(fp);
            if sim >= threshold {
                Some(HdcSearchResult {
                    id: id.clone(),
                    similarity: sim,
                    metadata: None,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));
    results.truncate(top_k);
    results
}

/// Format search results for CLI display.
#[must_use]
pub fn format_results(results: &[HdcSearchResult]) -> String {
    if results.is_empty() {
        return "No similar episodes found.".to_string();
    }

    let mut output = String::from("HDC Similarity Search Results:\n");
    output.push_str(&format!("{:<40} {:>10}\n", "Episode ID", "Similarity"));
    output.push_str(&"-".repeat(52));
    output.push('\n');

    for result in results {
        output.push_str(&format!(
            "{:<40} {:>9.4}\n",
            if result.id.len() > 38 {
                format!("{}...", &result.id[..35])
            } else {
                result.id.clone()
            },
            result.similarity,
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_exact_match() {
        let query = HdcVector::random();
        let episodes = vec![
            ("episode-1".to_string(), HdcVector::random()),
            ("episode-exact".to_string(), query),
            ("episode-3".to_string(), HdcVector::random()),
        ];

        let results = search_episodes(&query, &episodes, 3);
        assert_eq!(results[0].id, "episode-exact");
        assert!(results[0].similarity > 0.99);
    }

    #[test]
    fn threshold_search_filters() {
        let query = HdcVector::random();
        let episodes = vec![
            ("episode-1".to_string(), HdcVector::random()),
            ("episode-exact".to_string(), query),
        ];

        let results = search_with_threshold(&query, &episodes, 0.99, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "episode-exact");
    }

    #[test]
    fn format_results_handles_empty() {
        let formatted = format_results(&[]);
        assert!(formatted.contains("No similar"));
    }
}
