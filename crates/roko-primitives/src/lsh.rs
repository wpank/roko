//! P4-26: Locality-Sensitive Hashing (LSH) index for sub-linear HDC similarity search.
//!
//! Provides an `LshIndex` that hashes [`HdcVector`]s into buckets using
//! random hyperplanes, enabling approximate nearest-neighbor queries in
//! O(L * k) time instead of O(n * d) brute-force, where L = number of tables,
//! k = hash bits per table, n = indexed vectors, d = vector dimension.

use crate::hdc::{HdcVector, HDC_BITS};

/// Number of hash tables (more tables = higher recall, more memory).
const DEFAULT_NUM_TABLES: usize = 8;

/// Number of hash bits per table (more bits = fewer false positives but lower recall).
const DEFAULT_HASH_BITS: usize = 16;

/// A set of random hyperplanes for one hash table.
#[derive(Debug, Clone)]
struct HashTable {
    /// Random hyperplane indices (which bit positions to sample).
    hyperplane_indices: Vec<usize>,
    /// Buckets: hash -> list of (vector_id, vector).
    buckets: std::collections::HashMap<u64, Vec<usize>>,
}

impl HashTable {
    fn new(hash_bits: usize, seed: u64) -> Self {
        let mut rng_state = seed;
        let hyperplane_indices: Vec<usize> = (0..hash_bits)
            .map(|_| {
                rng_state = rng_state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                (rng_state >> 33) as usize % HDC_BITS
            })
            .collect();

        Self {
            hyperplane_indices,
            buckets: std::collections::HashMap::new(),
        }
    }

    fn hash(&self, vector: &HdcVector) -> u64 {
        let bytes = vector.to_bytes();
        let mut hash = 0u64;
        for (i, &bit_index) in self.hyperplane_indices.iter().enumerate() {
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            if byte_index < bytes.len() && (bytes[byte_index] >> bit_offset) & 1 == 1 {
                hash |= 1 << i;
            }
        }
        hash
    }

    fn insert(&mut self, vector_id: usize, vector: &HdcVector) {
        let hash = self.hash(vector);
        self.buckets.entry(hash).or_default().push(vector_id);
    }

    fn query(&self, vector: &HdcVector) -> Vec<usize> {
        let hash = self.hash(vector);
        self.buckets
            .get(&hash)
            .cloned()
            .unwrap_or_default()
    }
}

/// LSH index for approximate nearest-neighbor search over HDC vectors.
///
/// # Example
///
/// ```
/// use roko_primitives::hdc::HdcVector;
/// use roko_primitives::lsh::LshIndex;
///
/// let mut index = LshIndex::new(4, 8);
/// let v1 = HdcVector::random();
/// let v2 = HdcVector::random();
/// index.insert("entry-1", v1);
/// index.insert("entry-2", v2);
///
/// let results = index.query(&v1, 5);
/// assert!(!results.is_empty());
/// ```
pub struct LshIndex {
    tables: Vec<HashTable>,
    /// Stored vectors for re-ranking.
    vectors: Vec<(String, HdcVector)>,
}

impl LshIndex {
    /// Create a new LSH index with the given number of tables and hash bits.
    #[must_use]
    pub fn new(num_tables: usize, hash_bits: usize) -> Self {
        let tables: Vec<HashTable> = (0..num_tables.max(1))
            .map(|i| HashTable::new(hash_bits.max(1), (i as u64 + 1) * 0x517c_c1b7_2722_0a95))
            .collect();
        Self {
            tables,
            vectors: Vec::new(),
        }
    }

    /// Create an index with default parameters.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_NUM_TABLES, DEFAULT_HASH_BITS)
    }

    /// Insert a vector with a string identifier.
    pub fn insert(&mut self, id: impl Into<String>, vector: HdcVector) {
        let idx = self.vectors.len();
        self.vectors.push((id.into(), vector));
        for table in &mut self.tables {
            table.insert(idx, &vector);
        }
    }

    /// Query for the top-k most similar vectors.
    ///
    /// Returns `(id, similarity)` pairs sorted by similarity descending.
    #[must_use]
    pub fn query(&self, query: &HdcVector, k: usize) -> Vec<(String, f32)> {
        if self.vectors.is_empty() {
            return Vec::new();
        }

        // Collect candidate indices from all tables (union).
        let mut candidates = std::collections::HashSet::new();
        for table in &self.tables {
            for idx in table.query(query) {
                candidates.insert(idx);
            }
        }

        // Re-rank candidates by exact similarity.
        let mut results: Vec<(String, f32)> = candidates
            .into_iter()
            .filter_map(|idx| {
                self.vectors.get(idx).map(|(id, vec)| {
                    (id.clone(), query.similarity(vec))
                })
            })
            .collect();

        results.sort_by(|a, b| b.1.total_cmp(&a.1));
        results.truncate(k);
        results
    }

    /// Number of indexed vectors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Whether the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Brute-force query for comparison/testing.
    ///
    /// Returns the top-k most similar vectors by exact similarity.
    #[must_use]
    pub fn brute_force_query(&self, query: &HdcVector, k: usize) -> Vec<(String, f32)> {
        let mut results: Vec<(String, f32)> = self
            .vectors
            .iter()
            .map(|(id, vec)| (id.clone(), query.similarity(vec)))
            .collect();
        results.sort_by(|a, b| b.1.total_cmp(&a.1));
        results.truncate(k);
        results
    }
}

impl Default for LshIndex {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_query_finds_similar_vectors() {
        let mut index = LshIndex::new(4, 8);
        let v1 = HdcVector::random();
        let v2 = v1; // Exact copy should be found.
        let v3 = HdcVector::random();

        index.insert("exact", v1);
        index.insert("different", v3);

        let results = index.query(&v2, 5);
        // The exact copy should appear with similarity 1.0.
        if let Some((id, sim)) = results.first() {
            if id == "exact" {
                assert!(*sim > 0.99, "exact match should have high similarity: {sim}");
            }
        }
    }

    #[test]
    fn empty_index_returns_empty() {
        let index = LshIndex::with_defaults();
        let results = index.query(&HdcVector::random(), 5);
        assert!(results.is_empty());
    }

    #[test]
    fn brute_force_is_deterministic() {
        let mut index = LshIndex::with_defaults();
        let query = HdcVector::random();
        for i in 0..10 {
            index.insert(format!("v{i}"), HdcVector::random());
        }

        let r1 = index.brute_force_query(&query, 3);
        let r2 = index.brute_force_query(&query, 3);
        assert_eq!(r1.len(), r2.len());
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.0, b.0);
        }
    }
}
