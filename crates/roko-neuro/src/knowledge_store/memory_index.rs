//! In-memory HDC index over durable knowledge entries.

#[cfg(feature = "hdc")]
use roko_primitives::hdc::HdcVector;

#[cfg(feature = "hdc")]
use crate::KnowledgeEntry;
#[cfg(feature = "hdc")]
use crate::hdc::KnowledgeHdcEncoder;

#[cfg(feature = "hdc")]
use super::scoring::{compare_hits, fingerprint_content, fingerprint_entry};

#[cfg(feature = "hdc")]
/// A precomputed HDC index over durable knowledge entries.
///
/// The index stores both a normalized content fingerprint and the entry's
/// structured HDC vector. Searches rank by the stronger content or structured
/// match, preserving exact content lookup alongside role-aware causal lookup.
#[derive(Debug, Clone)]
pub struct MemoryIndex {
    entries: Vec<IndexedKnowledgeEntry>,
}

#[cfg(feature = "hdc")]
#[derive(Debug, Clone)]
struct IndexedKnowledgeEntry {
    entry: KnowledgeEntry,
    fingerprint: HdcVector,
    content_fingerprint: HdcVector,
}

#[cfg(feature = "hdc")]
/// One HDC search result from a [`MemoryIndex`].
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryHit {
    /// The matched knowledge entry.
    pub entry: KnowledgeEntry,
    /// Similarity against the query fingerprint in the range `0.0..=1.0`.
    pub similarity: f64,
}

#[cfg(feature = "hdc")]
impl MemoryIndex {
    /// Build an index from a collection of knowledge entries.
    ///
    /// Each entry receives content and structured fingerprints. Empty content
    /// still receives a deterministic vector, so the index remains total.
    #[must_use]
    pub fn from_entries(entries: Vec<KnowledgeEntry>) -> Self {
        let entries = entries
            .into_iter()
            .map(|entry| {
                let fingerprint = fingerprint_entry(&entry);
                let content_fingerprint = fingerprint_content(&entry.content);
                IndexedKnowledgeEntry {
                    entry,
                    fingerprint,
                    content_fingerprint,
                }
            })
            .collect();
        Self { entries }
    }

    /// Number of indexed entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the index contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Search the index for the `limit` most similar entries to `query`.
    ///
    /// The query is encoded as both content and structured probes, then
    /// compared against each precomputed entry vector. Results are sorted from
    /// highest to lowest similarity.
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<MemoryHit> {
        if limit == 0 || self.entries.is_empty() {
            return Vec::new();
        }

        let query_fingerprint = KnowledgeHdcEncoder.encode_query(query);
        let query_content_fingerprint = fingerprint_content(query);
        let mut scored: Vec<MemoryHit> = self
            .entries
            .iter()
            .map(|indexed| {
                let structured_similarity = query_fingerprint.similarity(&indexed.fingerprint);
                let content_similarity =
                    query_content_fingerprint.similarity(&indexed.content_fingerprint);
                MemoryHit {
                    entry: indexed.entry.clone(),
                    similarity: structured_similarity.max(content_similarity) as f64,
                }
            })
            .collect();

        scored.sort_by(compare_hits);
        scored.truncate(limit);
        scored
    }

    /// Return the indexed entries with their precomputed fingerprints.
    ///
    /// This is mainly useful for testing and for consumers that want to
    /// inspect or reuse the durable entries directly.
    #[must_use]
    pub fn entries(&self) -> Vec<KnowledgeEntry> {
        self.entries
            .iter()
            .map(|indexed| indexed.entry.clone())
            .collect()
    }
}
