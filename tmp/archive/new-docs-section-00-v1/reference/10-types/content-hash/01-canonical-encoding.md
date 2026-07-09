# ContentHash — Canonical Encoding

> The exact byte sequence fed to BLAKE3 to produce an Engram's ContentHash.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`canonical_encode(engram)` produces a deterministic byte vector that is then passed to
BLAKE3. The encoding is field-ordered and length-delimited. The exact byte layout is
specified here so that any implementation (in any language) can reproduce the same hash
given the same Engram fields.

---

## Encoding Rules

1. **Field order**: fields are encoded in the order listed in the table below.
2. **Length prefixing**: variable-length fields are prefixed with their length as a
   little-endian `u64`.
3. **No separators**: there are no field separators — the length prefix is the only
   delimiter.
4. **BTreeMap iteration**: `tags` is encoded in BTreeMap iteration order (lexicographic
   by key). This guarantees determinism.
5. **Lineage order**: lineage hashes are encoded in the order they appear in the `Vec`.
   Callers are responsible for consistent ordering.

---

## Field Encoding Table

| Order | Field | Encoding |
|---|---|---|
| 1 | `kind` | `u64_le(len) ++ kind.as_bytes()` |
| 2 | `body` | `u64_le(len) ++ body.canonical_bytes()` |
| 3 | `created_at_ms` | `i64_le` (8 bytes, fixed) |
| 4 | `provenance.author` | `u64_le(len) ++ author.as_bytes()` |
| 5 | `lineage.len()` | `u64_le` (8 bytes, fixed) |
| 6 | For each parent hash | `32` bytes (fixed — ContentHash is always 32 bytes) |
| 7 | `tags.len()` | `u64_le` (8 bytes, fixed) |
| 8 | For each `(k, v)` in tags (sorted) | `u64_le(key_len) ++ key.as_bytes() ++ u64_le(val_len) ++ val.as_bytes()` |

---

## Implementation

```rust
<!-- source: crates/roko-core/src/content_hash.rs -->

pub fn canonical_encode(engram: &Engram) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);

    // 1. kind
    let kind_bytes = engram.kind.as_bytes();
    buf.extend_from_slice(&(kind_bytes.len() as u64).to_le_bytes());
    buf.extend_from_slice(kind_bytes);

    // 2. body
    let body_bytes = engram.body.canonical_bytes();
    buf.extend_from_slice(&(body_bytes.len() as u64).to_le_bytes());
    buf.extend_from_slice(&body_bytes);

    // 3. created_at_ms
    buf.extend_from_slice(&engram.created_at_ms.to_le_bytes());

    // 4. provenance.author (trust and taint are NOT included)
    let author_bytes = engram.provenance.author.as_bytes();
    buf.extend_from_slice(&(author_bytes.len() as u64).to_le_bytes());
    buf.extend_from_slice(author_bytes);

    // 5 + 6. lineage
    buf.extend_from_slice(&(engram.lineage.len() as u64).to_le_bytes());
    for parent_hash in &engram.lineage {
        buf.extend_from_slice(parent_hash.as_bytes());
    }

    // 7 + 8. tags (BTreeMap — already in sorted order)
    buf.extend_from_slice(&(engram.tags.len() as u64).to_le_bytes());
    for (key, value) in &engram.tags {
        let key_bytes = key.as_bytes();
        buf.extend_from_slice(&(key_bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(key_bytes);
        let val_bytes = value.as_bytes();
        buf.extend_from_slice(&(val_bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(val_bytes);
    }

    buf
}

pub fn compute_hash(engram: &Engram) -> ContentHash {
    let encoded = canonical_encode(engram);
    let digest = blake3::hash(&encoded);
    ContentHash(*digest.as_bytes())
}
```

---

## Body Canonical Encoding

`Body::canonical_bytes()` is variant-specific:

```rust
<!-- source: crates/roko-core/src/body.rs -->

impl Body {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        match self {
            Body::Text(s) => s.as_bytes().to_vec(),
            Body::Embedding(vec) => {
                // f32 little-endian concatenation
                vec.iter().flat_map(|f| f.to_le_bytes()).collect()
            }
            Body::Json(v) => serde_json::to_vec(v).expect("valid JSON"),
            Body::Binary(bytes) => bytes.clone(),
            Body::Structured(fields) => {
                // Sorted by key for determinism
                let mut sorted: Vec<_> = fields.iter().collect();
                sorted.sort_by_key(|(k, _)| k.as_str());
                let mut out = Vec::new();
                for (k, v) in sorted {
                    out.extend_from_slice(k.as_bytes());
                    out.extend_from_slice(b"\x00");
                    out.extend_from_slice(v.canonical_bytes().as_slice());
                    out.extend_from_slice(b"\x01");
                }
                out
            }
        }
    }
}
```

---

## Kind Canonical Encoding

```rust
<!-- source: crates/roko-core/src/kind.rs -->

impl Kind {
    /// Return the stable string bytes for this Kind variant.
    /// The string is the snake_case name of the variant.
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            Kind::AgentOutput    => b"agent_output",
            Kind::GateVerdict    => b"gate_verdict",
            Kind::ToolTrace      => b"tool_trace",
            Kind::KnowledgeEntry => b"knowledge_entry",
            Kind::Prediction     => b"prediction",
            Kind::Observation    => b"observation",
            Kind::Plan           => b"plan",
            Kind::Episode        => b"episode",
            Kind::Reflection     => b"reflection",
            Kind::Pheromone      => b"pheromone",
            Kind::Metric         => b"metric",
            Kind::ContextAssembly => b"context_assembly",
            Kind::ModelSelection => b"model_selection",
            Kind::ErrorRecord    => b"error_record",
            // Custom variants use the custom string's bytes:
            Kind::Custom(s)      => s.as_bytes(),  // NOT 'static; handled separately
        }
    }
}
```

Note: for `Kind::Custom(s)`, the actual string bytes are used directly. There is no
wrapper prefix — two custom Kind values with different strings always produce different
encodings.

---

## Invariants

1. `canonical_encode()` is pure and deterministic — same inputs always produce the same
   bytes.
2. `canonical_encode()` never reads `score`, `decay`, `provenance.trust`, `provenance.taint`,
   or `fingerprint`.
3. The output byte length is unbounded (large Body values produce large encodings) but
   BLAKE3 handles arbitrary-length inputs.
4. Tag encoding uses `BTreeMap` order, not insertion order.
5. `ContentHash` is always exactly 32 bytes.

---

## Open Questions

- Should the encoding include a version prefix (e.g., `0x01`) to allow future migration?
  Not currently included; breaking change if added.
- Should `Body::Embedding` use network byte order rather than little-endian?

## See Also

- [`00-overview.md`](00-overview.md) — what enters the hash and why
- [`02-api-reference.md`](02-api-reference.md) — method signatures
- [`03-invariants.md`](03-invariants.md) — full invariant list
