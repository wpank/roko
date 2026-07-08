# HDC Fingerprint — Encoding Pipeline

> How an Engram's Body content is converted into an HdcVector by the HDC encoder.

**Status**: Shipping  
**Crate**: `bardo-primitives`  
**Depends on**: [HdcVector Format](01-hdc-vector.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

The HDC encoder converts a `Body` value into an `HdcVector` in three stages: tokenization
(body → tokens), token projection (token → random vector via seeded hash), and bundle
(combine token vectors via majority vote). The result is a 10,240-bit vector that encodes
the semantic content of the body. The encoder version is stored alongside the vector so
that future encoder upgrades can be detected.

---

## Pipeline Stages

```
Body
  │
  ▼ (1) Tokenize
[token_1, token_2, ..., token_N]
  │
  ▼ (2) Project each token → HdcVector via seeded RNG
[vec_1, vec_2, ..., vec_N]
  │
  ▼ (3) Bundle via majority vote
HdcVector (final fingerprint)
```

---

## Stage 1: Tokenization

Tokenization is body-variant-specific:

| Body variant | Tokenization |
|---|---|
| `Body::Text(s)` | Unicode word splitting; each word (lowercased, stripped of punctuation) is a token |
| `Body::Embedding(vec)` | Each `f32` is quantized to a 256-bin integer token |
| `Body::Json(v)` | DFS traversal of JSON; each leaf string/number value is a token |
| `Body::Binary(_)` | Not encodable — fingerprint is `None` for binary bodies |
| `Body::Structured(fields)` | Each `(key, value)` pair produces a bound vector (see below) |

For `Body::Structured`, each field is encoded as `key_vec XOR value_vec` (HDC binding),
then all bound vectors are bundled via majority vote. This preserves the key-value
structure in the vector space.

---

## Stage 2: Token Projection

Each token is projected to an `HdcVector` by seeding a deterministic RNG with the token
string and sampling 10,240 random bits:

```rust
<!-- source: crates/bardo-primitives/src/encoder.rs -->

fn project_token(token: &str, encoder_version: u32) -> HdcVector {
    // Seed = BLAKE3(encoder_version || token_bytes)
    // This ensures the same token always maps to the same vector for a given version.
    let seed_bytes = blake3::hash(
        &[&encoder_version.to_le_bytes(), token.as_bytes()].concat()
    );
    let mut rng = rand_chacha::ChaCha8Rng::from_seed(*seed_bytes.as_bytes());
    HdcVector::random(&mut rng)
}
```

The seed includes `encoder_version` so that the same token maps to different vectors
under different encoder versions (making cross-version comparison semantically undefined).

---

## Stage 3: Bundle

```rust
<!-- source: crates/bardo-primitives/src/encoder.rs -->

fn bundle_tokens(token_vecs: &[HdcVector]) -> HdcVector {
    if token_vecs.is_empty() {
        return HdcVector::zero();
    }
    majority_bundle(&token_vecs.iter().collect::<Vec<_>>())
}
```

---

## Full Encoding Function

```rust
<!-- source: crates/bardo-primitives/src/encoder.rs -->

pub struct HdcEncoder {
    pub version: u32,
}

impl HdcEncoder {
    pub fn encode(&self, body: &Body) -> Option<HdcFingerprint> {
        let tokens = self.tokenize(body)?;
        if tokens.is_empty() {
            return None;
        }
        let vecs: Vec<HdcVector> = tokens.iter()
            .map(|t| project_token(t, self.version))
            .collect();
        let vector = bundle_tokens(&vecs);
        Some(HdcFingerprint { vector, encoder_version: self.version })
    }

    fn tokenize(&self, body: &Body) -> Option<Vec<String>> {
        match body {
            Body::Text(s) => Some(tokenize_text(s)),
            Body::Embedding(v) => Some(tokenize_embedding(v)),
            Body::Json(j) => Some(tokenize_json(j)),
            Body::Binary(_) => None,  // not encodable
            Body::Structured(fields) => Some(tokenize_structured(fields)),
        }
    }
}
```

---

## Current Encoder Version

```rust
<!-- source: crates/bardo-primitives/src/encoder.rs -->

/// The current production encoder version.
pub const CURRENT_ENCODER_VERSION: u32 = 1;
```

---

## Invariants

1. `encode()` is deterministic for the same `(body, version)` pair.
2. `encode()` returns `None` for `Body::Binary` — binary bodies have no fingerprint.
3. The empty body produces `None` (not a zero vector).
4. `encoder_version` in the output equals `self.version`.
5. Different encoder versions produce incomparable vectors for the same body.

---

## Open Questions

- Should sub-word tokenization (BPE) be used instead of word tokenization? Word-level
  is simpler and sufficient for current use; BPE would improve multilingual support.
- Should `Body::Embedding` fingerprinting use the embedding dimensions directly rather
  than quantizing to tokens?

## See Also

- [`01-hdc-vector.md`](01-hdc-vector.md) — the vector type
- [`03-similarity-distance.md`](03-similarity-distance.md) — using the produced vector
- [`04-encoder-versioning.md`](04-encoder-versioning.md) — encoder version management
