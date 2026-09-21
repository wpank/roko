//! RAG-22: Cross-encoder reranking interface.
//!
//! Defines the [`Reranker`] trait and two built-in implementations:
//!
//! - [`NoopReranker`] — identity reranker; preserves original rank order.
//! - [`HeuristicReranker`] — scores by query-term overlap and document length,
//!   yielding a lightweight local reranker that never makes network calls.
//!
//! Both are gated behind `[retrieval] rerank_enabled` in `roko.toml`.
//! The factory helper [`reranker_for_config`] returns the appropriate
//! implementation based on the config flag.

use roko_core::config::RetrievalConfig;

/// Provider-neutral interface for reranking retrieved documents.
///
/// Implementations return a list of `(original_index, score)` pairs sorted
/// in descending score order.  The caller uses the index to recover the
/// corresponding document from the original candidate list.
///
/// # Enabling
///
/// Set `[retrieval] rerank_enabled = true` in `roko.toml`.  The default
/// [`NoopReranker`] is a zero-cost stand-in; the [`HeuristicReranker`] is the
/// built-in local implementation.
pub trait Reranker: Send + Sync {
    /// Rerank `documents` with respect to `query`.
    ///
    /// Returns a `Vec<(original_index, score)>` sorted descending by score.
    /// The `original_index` maps back into the `documents` slice, so callers
    /// can reconstruct the ordered list without holding a copy.
    fn rerank(&self, query: &str, documents: &[&str]) -> Vec<(usize, f64)>;

    /// Human-readable name for the reranker implementation.
    fn name(&self) -> &str;
}

// ── NoopReranker ──────────────────────────────────────────────────────────────

/// An identity reranker that preserves the original document order.
///
/// Used when `[retrieval] rerank_enabled = false` (the default).  Scores are
/// assigned as `1.0 - (index / len)` so the output list is still in the original
/// order but with monotonically decreasing scores to satisfy callers that
/// expect a score field.
#[derive(Clone, Debug, Default)]
pub struct NoopReranker;

impl Reranker for NoopReranker {
    fn rerank(&self, _query: &str, documents: &[&str]) -> Vec<(usize, f64)> {
        let n = documents.len();
        if n == 0 {
            return Vec::new();
        }
        (0..n)
            .map(|i| {
                // Score decreases from 1.0 down to avoid a tie at 0 for
                // very short lists.
                let score = 1.0 - (i as f64 / n as f64);
                (i, score)
            })
            .collect()
    }

    fn name(&self) -> &str {
        "noop"
    }
}

// ── HeuristicReranker ─────────────────────────────────────────────────────────

/// A local heuristic reranker based on query-term overlap and document length.
///
/// Scoring formula (per document `d` for query `q`):
///
/// ```text
/// overlap_score = |query_terms ∩ doc_terms| / |query_terms|   (0.0 when q is empty)
/// length_penalty = 1.0 / (1.0 + log2(1 + word_count(d)))
/// score = overlap_score * (1.0 - length_weight) + (1.0 - length_penalty) * length_weight
/// ```
///
/// The `length_weight` parameter (default 0.15) controls how much the length
/// penalty contributes relative to term overlap.  A value of `0.0` makes this
/// a pure term-overlap reranker; `1.0` ranks shorter documents first regardless
/// of query relevance.
///
/// # Why this formula?
///
/// - **Overlap** rewards documents that share vocabulary with the query.
/// - **Length penalty** lightly discounts very long documents that could dilute
///   precision when the context window is bounded.
/// - Both signals are cheap to compute without any network calls.
#[derive(Clone, Debug)]
pub struct HeuristicReranker {
    /// Weight of the length penalty signal.  Must be in `[0.0, 1.0]`.
    pub length_weight: f64,
}

impl Default for HeuristicReranker {
    fn default() -> Self {
        Self { length_weight: 0.15 }
    }
}

impl HeuristicReranker {
    /// Construct with a custom length-penalty weight.
    #[must_use]
    pub fn new(length_weight: f64) -> Self {
        // Clamp to a valid range.
        Self {
            length_weight: length_weight.clamp(0.0, 1.0),
        }
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| !w.is_empty())
            .collect()
    }

    fn score_one(&self, query_terms: &[String], doc: &str) -> f64 {
        let doc_tokens = Self::tokenize(doc);
        let doc_word_count = doc_tokens.len();

        let overlap_score = if query_terms.is_empty() {
            0.0
        } else {
            let matches = query_terms
                .iter()
                .filter(|qt| doc_tokens.iter().any(|dt| dt == *qt))
                .count();
            matches as f64 / query_terms.len() as f64
        };

        // Length penalty: shorter docs score higher on this component.
        let length_penalty = 1.0 / (1.0 + (1.0 + doc_word_count as f64).log2());

        // Weighted combination.
        let w = self.length_weight;
        overlap_score * (1.0 - w) + (1.0 - length_penalty) * w
    }
}

impl Reranker for HeuristicReranker {
    fn rerank(&self, query: &str, documents: &[&str]) -> Vec<(usize, f64)> {
        let query_terms = Self::tokenize(query);
        let mut scored: Vec<(usize, f64)> = documents
            .iter()
            .enumerate()
            .map(|(i, doc)| (i, self.score_one(&query_terms, doc)))
            .collect();
        // Sort descending by score; stable sort for determinism on equal scores.
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    fn name(&self) -> &str {
        "heuristic"
    }
}

// ── Factory ───────────────────────────────────────────────────────────────────

/// Construct a boxed [`Reranker`] appropriate for the given retrieval config.
///
/// Returns a [`NoopReranker`] when `rerank_enabled` is `false` (the default),
/// or a [`HeuristicReranker`] with default parameters when enabled.
#[must_use]
pub fn reranker_for_config(config: &RetrievalConfig) -> Box<dyn Reranker> {
    if config.rerank_enabled {
        Box::new(HeuristicReranker::default())
    } else {
        Box::new(NoopReranker)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── NoopReranker ──────────────────────────────────────────────────────────

    #[test]
    fn noop_reranker_preserves_order() {
        let docs = ["alpha", "beta", "gamma"];
        let result = NoopReranker.rerank("query", &docs);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, 0);
        assert_eq!(result[1].0, 1);
        assert_eq!(result[2].0, 2);
    }

    #[test]
    fn noop_reranker_scores_descending() {
        let docs = ["a", "b", "c"];
        let result = NoopReranker.rerank("q", &docs);
        assert!(result[0].1 >= result[1].1);
        assert!(result[1].1 >= result[2].1);
    }

    #[test]
    fn noop_reranker_empty_documents() {
        let result = NoopReranker.rerank("anything", &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn noop_reranker_name() {
        assert_eq!(NoopReranker.name(), "noop");
    }

    // ── HeuristicReranker ─────────────────────────────────────────────────────

    #[test]
    fn heuristic_reranker_exact_match_scores_higher_than_no_match() {
        let reranker = HeuristicReranker::default();
        let docs = ["this document is irrelevant", "rust programming language"];
        let result = reranker.rerank("rust language", &docs);
        // The second document (index 1) should be ranked first.
        assert_eq!(
            result[0].0, 1,
            "document containing query terms should rank first"
        );
    }

    #[test]
    fn heuristic_reranker_returns_all_documents() {
        let reranker = HeuristicReranker::default();
        let docs = ["doc one", "doc two", "doc three", "doc four"];
        let result = reranker.rerank("doc", &docs);
        assert_eq!(result.len(), 4);
        // Every index appears exactly once.
        let mut indices: Vec<usize> = result.iter().map(|(i, _)| *i).collect();
        indices.sort_unstable();
        assert_eq!(indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn heuristic_reranker_empty_query_all_zero_overlap() {
        let reranker = HeuristicReranker::default();
        let docs = ["hello world", "foo bar"];
        let result = reranker.rerank("", &docs);
        // Overlap is 0 for all; scores should still be returned in some order.
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn heuristic_reranker_empty_documents() {
        let reranker = HeuristicReranker::default();
        let result = reranker.rerank("query", &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn heuristic_reranker_name() {
        assert_eq!(HeuristicReranker::default().name(), "heuristic");
    }

    #[test]
    fn heuristic_reranker_length_weight_clamped() {
        let r = HeuristicReranker::new(2.0);
        assert!((r.length_weight - 1.0).abs() < f64::EPSILON);
        let r2 = HeuristicReranker::new(-0.5);
        assert!(r2.length_weight.abs() < f64::EPSILON);
    }

    #[test]
    fn heuristic_reranker_sorted_descending() {
        let reranker = HeuristicReranker::default();
        let docs = ["completely unrelated", "rust async programming", "rust language features"];
        let result = reranker.rerank("rust programming", &docs);
        // Scores should be in descending order.
        for i in 1..result.len() {
            assert!(
                result[i - 1].1 >= result[i].1,
                "scores not in descending order at index {i}"
            );
        }
    }

    // ── Factory ───────────────────────────────────────────────────────────────

    #[test]
    fn factory_returns_noop_when_disabled() {
        let config = RetrievalConfig::default(); // rerank_enabled = false
        let reranker = reranker_for_config(&config);
        assert_eq!(reranker.name(), "noop");
    }

    #[test]
    fn factory_returns_heuristic_when_enabled() {
        let mut config = RetrievalConfig::default();
        config.rerank_enabled = true;
        let reranker = reranker_for_config(&config);
        assert_eq!(reranker.name(), "heuristic");
    }
}
