//! RAG-07: HNSW-style ANN index for f32 vectors.
//!
//! This module provides [`HnswIndex`], an approximate nearest-neighbour (ANN)
//! index for dense `f32` vectors.  The current implementation uses brute-force
//! linear scan sorted by cosine similarity, which is correct for small-to-medium
//! collections (< ~10 000 entries) and can be upgraded to a real HNSW graph later
//! without changing the public API.
//!
//! # Design
//!
//! - **`insert(id, vector)`** — registers a vector under a `u64` id.  Calling
//!   `insert` a second time with the same id overwrites the previous entry.
//! - **`query(vector, k)`** — returns the `k` nearest neighbours as
//!   `(id, cosine_similarity)` pairs, sorted by descending similarity.
//!
//! # Cosine similarity
//!
//! The similarity score is the standard cosine dot-product, normalised to
//! `[−1, 1]`.  Zero-magnitude vectors score `0.0` against every other vector.
//!
//! # Example
//!
//! ```
//! use roko_primitives::hnsw::HnswIndex;
//!
//! let mut index = HnswIndex::new(4); // dimensionality = 4
//! index.insert(1, &[1.0, 0.0, 0.0, 0.0]);
//! index.insert(2, &[0.0, 1.0, 0.0, 0.0]);
//! index.insert(3, &[0.8, 0.6, 0.0, 0.0]);
//!
//! let results = index.query(&[1.0, 0.0, 0.0, 0.0], 2);
//! assert_eq!(results.len(), 2);
//! assert_eq!(results[0].0, 1); // exact match should be first
//! ```

/// ANN index for dense `f32` vectors.
///
/// Uses brute-force cosine search for correctness and simplicity.
/// The API is intentionally forward-compatible with a real HNSW implementation:
/// only `insert` / `query` / `len` / `is_empty` are exposed.
#[derive(Debug, Clone)]
pub struct HnswIndex {
    /// Expected dimensionality (informational; not enforced).
    dims: usize,
    /// Stored entries: `(id, normalised_vector)`.
    entries: Vec<(u64, Vec<f32>)>,
}

impl HnswIndex {
    /// Create a new empty index.
    ///
    /// `dims` is the expected vector dimensionality (used for documentation and
    /// future HNSW layer sizing). The current brute-force backend does not
    /// enforce it — queries and inserts with different lengths are handled
    /// gracefully (shorter vectors are treated as zero-padded).
    #[must_use]
    pub fn new(dims: usize) -> Self {
        Self {
            dims,
            entries: Vec::new(),
        }
    }

    /// The declared dimensionality of this index.
    #[must_use]
    pub fn dims(&self) -> usize {
        self.dims
    }

    /// Insert or overwrite a vector.
    ///
    /// The vector is L2-normalised before storage so that dot product equals
    /// cosine similarity at query time.  Zero-magnitude vectors are stored as-is
    /// (all zeros) and will score `0.0` against every query.
    pub fn insert(&mut self, id: u64, vector: &[f32]) {
        let normalised = normalise(vector);
        // Overwrite if the id already exists.
        if let Some(entry) = self.entries.iter_mut().find(|(eid, _)| *eid == id) {
            entry.1 = normalised;
            return;
        }
        self.entries.push((id, normalised));
    }

    /// Query for the `k` approximate nearest neighbours.
    ///
    /// Returns at most `k` `(id, cosine_similarity)` pairs sorted by
    /// descending similarity.  Returns an empty vec when `k == 0` or the
    /// index is empty.
    #[must_use]
    pub fn query(&self, vector: &[f32], k: usize) -> Vec<(u64, f32)> {
        if k == 0 || self.entries.is_empty() {
            return Vec::new();
        }

        let query_norm = normalise(vector);

        let mut scored: Vec<(u64, f32)> = self
            .entries
            .iter()
            .map(|(id, stored)| {
                let sim = dot_product(&query_norm, stored);
                (*id, sim)
            })
            .collect();

        // Sort descending by similarity.
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        scored.truncate(k);
        scored
    }

    /// Number of indexed vectors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the index contains no vectors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove all entries from the index.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// L2-normalise a vector slice.  Returns zeros for zero-magnitude inputs.
fn normalise(v: &[f32]) -> Vec<f32> {
    let mag = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag < f32::EPSILON {
        return v.to_vec();
    }
    v.iter().map(|x| x / mag).collect()
}

/// Dot product of two slices (interpreted as normalised vectors → cosine sim).
///
/// Short-circuits at the shorter length; excess dimensions are treated as zero.
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    a[..len].iter().zip(b[..len].iter()).map(|(x, y)| x * y).sum()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn empty_index_returns_empty_results() {
        let index = HnswIndex::new(3);
        assert!(index.query(&[1.0, 0.0, 0.0], 5).is_empty());
    }

    #[test]
    fn exact_match_scores_one() {
        let mut index = HnswIndex::new(3);
        index.insert(1, &[1.0, 0.0, 0.0]);
        let results = index.query(&[1.0, 0.0, 0.0], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
        assert!(approx_eq(results[0].1, 1.0));
    }

    #[test]
    fn orthogonal_vectors_score_zero() {
        let mut index = HnswIndex::new(2);
        index.insert(10, &[1.0, 0.0]);
        index.insert(11, &[0.0, 1.0]);
        let results = index.query(&[1.0, 0.0], 2);
        // First should be id=10 (score~1.0), second id=11 (score~0.0).
        assert_eq!(results[0].0, 10);
        assert!(approx_eq(results[1].1, 0.0));
    }

    #[test]
    fn k_limits_results() {
        let mut index = HnswIndex::new(2);
        for i in 0u64..10 {
            index.insert(i, &[i as f32, 0.0]);
        }
        let results = index.query(&[1.0, 0.0], 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn overwrite_replaces_entry() {
        let mut index = HnswIndex::new(2);
        index.insert(1, &[1.0, 0.0]);
        index.insert(1, &[0.0, 1.0]);
        assert_eq!(index.len(), 1);
        // Now querying [0,1] should give score ~1.0 for id=1.
        let results = index.query(&[0.0, 1.0], 1);
        assert_eq!(results[0].0, 1);
        assert!(approx_eq(results[0].1, 1.0));
    }

    #[test]
    fn results_sorted_descending() {
        let mut index = HnswIndex::new(3);
        index.insert(1, &[1.0, 0.0, 0.0]);
        index.insert(2, &[0.5, 0.5, 0.0]);
        index.insert(3, &[0.0, 0.0, 1.0]);
        let results = index.query(&[1.0, 0.0, 0.0], 3);
        assert!(results[0].1 >= results[1].1);
        assert!(results[1].1 >= results[2].1);
    }

    #[test]
    fn zero_k_returns_empty() {
        let mut index = HnswIndex::new(2);
        index.insert(1, &[1.0, 0.0]);
        assert!(index.query(&[1.0, 0.0], 0).is_empty());
    }

    #[test]
    fn len_and_is_empty() {
        let mut index = HnswIndex::new(2);
        assert!(index.is_empty());
        index.insert(1, &[1.0, 0.0]);
        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
        index.clear();
        assert!(index.is_empty());
    }
}
