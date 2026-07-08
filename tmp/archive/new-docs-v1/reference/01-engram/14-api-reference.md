# Engram — API Reference

> Complete public Rust API for `Engram`, `EngramBuilder`, and directly related types in `roko-core`.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Struct reference](01-struct-reference.md), [Builder](07-builder-pattern.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

The primary public surface consists of `Engram` (the struct, with instance methods),
`EngramBuilder` (construction), and `ContentHash` (identity). Substrate interactions are
via the `Substrate` trait, which all storage backends implement.

---

## `Engram` — Instance Methods

```rust
<!-- source: crates/roko-core/src/engram.rs -->

impl Engram {
    /// Recompute the expected id from stable fields and compare to self.id.
    /// Returns true if the id is valid.
    /// The Substrate calls this on every ingest.
    pub fn verify_id(&self) -> bool;

    /// Returns the effective weight of this Engram right now.
    /// Combines score.effective() × decay.weight_at(now, created_at_ms).
    pub fn effective_weight(&self) -> f64;

    /// Returns true if this Engram has a valid HDC fingerprint.
    pub fn has_fingerprint(&self) -> bool;

    /// Returns the decay weight at a specific timestamp (milliseconds).
    pub fn decay_weight_at(&self, ts_ms: i64) -> f64;

    /// Returns the age of this Engram in seconds.
    pub fn age_secs(&self) -> f64;

    /// Returns true if this Engram's lineage is empty (root Engram).
    pub fn is_root(&self) -> bool;

    /// Returns true if this Engram is tainted.
    pub fn is_tainted(&self) -> bool;
}
```

---

## `EngramBuilder` — Construction

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

impl EngramBuilder {
    /// Create a new builder with all defaults.
    pub fn new() -> Self;

    // Required fields
    pub fn kind(self, kind: Kind) -> Self;
    pub fn body(self, body: Body) -> Self;

    // Optional fields
    pub fn decay(self, decay: Decay) -> Self;
    pub fn provenance(self, provenance: Provenance) -> Self;
    pub fn score(self, score: Score) -> Self;
    pub fn lineage(self, lineage: Vec<ContentHash>) -> Self;
    /// Add a single parent to lineage.
    pub fn parent(self, parent: ContentHash) -> Self;
    /// Add a single tag.
    pub fn tag(self, key: impl Into<String>, value: impl Into<String>) -> Self;
    /// Set all tags at once (replaces any previously set tags).
    pub fn tags(self, tags: BTreeMap<String, String>) -> Self;
    /// Override the creation timestamp. Default: SystemTime::now().
    pub fn created_at_ms(self, ms: i64) -> Self;
    /// Skip fingerprint computation. FOR TESTS ONLY.
    pub fn skip_fingerprint(self) -> Self;

    /// Compute id and fingerprint; validate invariants; return Engram or error.
    pub fn build(self) -> Result<Engram, EngramBuildError>;
}
```

---

## `ContentHash` — Identity Type

```rust
<!-- source: crates/roko-core/src/content_hash.rs -->

impl ContentHash {
    /// Compute from pre-serialized canonical bytes.
    pub fn from_bytes(canonical: &[u8]) -> Self;

    /// Parse from 64-char lowercase hex string.
    pub fn from_hex(s: &str) -> Result<Self, ContentHashError>;

    /// Display as 64-char lowercase hex string.
    pub fn to_hex(&self) -> String;

    /// Raw 32-byte array reference.
    pub fn as_bytes(&self) -> &[u8; 32];

    /// Truncated 8-char hex for display purposes (not unique).
    pub fn short_hex(&self) -> String;
}

impl Display for ContentHash { /* 64-char hex */ }
impl Debug for ContentHash { /* "ContentHash(7f83b165...)" */ }
```

---

## `HdcFingerprint` — Semantic Fingerprint

```rust
<!-- source: crates/roko-core/src/engram.rs -->

impl HdcFingerprint {
    /// Normalized similarity to another fingerprint.
    /// Returns None if encoder versions differ.
    pub fn similarity(&self, other: &Self) -> Option<f32>;

    /// Raw Hamming distance. Returns None if encoder versions differ.
    pub fn hamming_distance(&self, other: &Self) -> Option<u32>;

    /// Returns true if the two fingerprints can be compared.
    pub fn compatible(&self, other: &Self) -> bool;
}
```

---

## `Substrate` Trait — Storage Interface

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

/// Trait implemented by all Engram storage backends.
pub trait Substrate: Send + Sync {
    // --- Write ---
    /// Insert an Engram. If the id already exists, this is a no-op (idempotent).
    fn insert(&self, engram: Engram) -> Result<InsertResult, SubstrateError>;

    /// Replace the score for an existing Engram. Does not change id.
    fn update_score(&self, id: &ContentHash, score: Score) -> Result<(), SubstrateError>;

    /// Replace the decay model for an existing Engram. Does not change id.
    fn update_decay(&self, id: &ContentHash, decay: Decay) -> Result<(), SubstrateError>;

    /// Set the tainted flag and propagate to all descendants.
    fn taint(&self, id: &ContentHash, reason: &str) -> Result<usize, SubstrateError>;

    /// Upgrade the trust level of an Engram. Records in custody.
    fn attest(
        &self,
        id: &ContentHash,
        new_trust: TrustLevel,
        attester: &str,
    ) -> Result<(), SubstrateError>;

    // --- Read ---
    /// Retrieve by exact ContentHash. O(1).
    fn get(&self, id: &ContentHash) -> Option<Engram>;

    /// Return direct parents of an Engram.
    fn parents(&self, id: &ContentHash) -> Vec<Engram>;

    /// Return direct children of an Engram.
    fn children(&self, id: &ContentHash) -> Vec<Engram>;

    /// Return all ancestors up to max_depth.
    fn ancestors(&self, id: &ContentHash, max_depth: usize) -> Vec<Engram>;

    /// Return all descendants up to max_depth.
    fn descendants(&self, id: &ContentHash, max_depth: usize) -> Vec<Engram>;

    /// Similarity search by HDC fingerprint.
    fn find_similar(
        &self,
        query: &HdcFingerprint,
        threshold: f32,
        limit: usize,
    ) -> Vec<(Engram, f32)>;  // (engram, similarity_score)

    /// Full-text / kind filter scan.
    fn scan(
        &self,
        filter: SubstrateFilter,
        limit: usize,
    ) -> Vec<Engram>;

    // --- Lifecycle ---
    /// Run garbage collection: remove Engrams below effective_weight threshold.
    fn gc(&self, threshold: f64) -> Result<GcResult, SubstrateError>;

    /// Flush pending writes to durable storage.
    fn flush(&self) -> Result<(), SubstrateError>;

    /// Total count of Engrams in the substrate.
    fn len(&self) -> usize;
}
```

---

## `SubstrateFilter` — Query Builder

```rust
<!-- source: crates/roko-core/src/substrate.rs -->

pub struct SubstrateFilter {
    pub kinds: Option<Vec<Kind>>,
    pub min_score: Option<f64>,
    pub max_age_ms: Option<i64>,
    pub min_trust: Option<TrustLevel>,
    pub exclude_tainted: bool,
    pub author: Option<String>,
    pub tags: BTreeMap<String, String>,
}

impl SubstrateFilter {
    pub fn new() -> Self;
    pub fn kind(self, kind: Kind) -> Self;
    pub fn kinds(self, kinds: Vec<Kind>) -> Self;
    pub fn min_score(self, score: f64) -> Self;
    pub fn max_age_secs(self, secs: f64) -> Self;
    pub fn min_trust(self, trust: TrustLevel) -> Self;
    pub fn exclude_tainted(self) -> Self;
    pub fn author(self, author: impl Into<String>) -> Self;
    pub fn tag(self, key: impl Into<String>, value: impl Into<String>) -> Self;
}
```

---

## Error Types

```rust
<!-- source: crates/roko-core/src/error.rs -->

#[derive(Debug)]
pub enum EngramBuildError {
    MissingKind,
    MissingBody,
    BodyKindMismatch { kind: Kind, body_variant: &'static str },
    InvalidTimestamp(i64),
    DuplicateLineage(ContentHash),
    InvalidTagKey(String),
    JsonFieldInvalid { field: &'static str, error: String },
    CustomBodyEmptyTypeTag,
}

#[derive(Debug)]
pub enum SubstrateError {
    HashMismatch { id: ContentHash },
    LineageCycle,
    UnknownEncoderVersion(u32),
    ClockSkewTooLarge(i64),
    StorageError(String),
    NotFound(ContentHash),
    TrustDowngrade { current: TrustLevel, proposed: TrustLevel },
}

#[derive(Debug)]
pub enum ContentHashError {
    InvalidHexLength { expected: usize, got: usize },
    InvalidHexChar { char: char },
}
```

---

## See Also

- [`07-builder-pattern.md`](07-builder-pattern.md) — builder usage guide
- [`12-invariants.md`](12-invariants.md) — what the API enforces
- [`13-examples.md`](13-examples.md) — worked usage examples
