# HDC Fingerprint — Examples

> Worked examples for encoding, comparison, near-duplicate detection, and encoder migration.

**Status**: Shipping  
**Crate**: `bardo-primitives`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## Example 1 — Encode a Text Body

```rust
<!-- source: crates/bardo-primitives/tests/hdc_examples.rs -->

let encoder = HdcEncoder { version: CURRENT_ENCODER_VERSION };
let body = Body::Text("The quick brown fox jumps over the lazy dog".into());
let fp = encoder.encode(&body).expect("text is encodable");

assert_eq!(fp.encoder_version, CURRENT_ENCODER_VERSION);
assert_eq!(fp.vector.0.len(), 160);
// Popcount should be near 5120 (±10%)
let pc = fp.vector.popcount();
assert!(pc > 4_500 && pc < 5_700, "popcount = {}", pc);
```

---

## Example 2 — Similar Texts Have Low Hamming Distance

```rust
<!-- source: crates/bardo-primitives/tests/hdc_examples.rs -->

let encoder = HdcEncoder { version: 1 };
let fp1 = encoder.encode(&Body::Text("Roko is a cognitive agent runtime".into())).unwrap();
let fp2 = encoder.encode(&Body::Text("Roko is a cognitive runtime for agents".into())).unwrap();

let dist = fp1.vector.hamming_distance(&fp2.vector);
// Semantically similar → distance should be well below 10_240/2 = 5120
assert!(dist < 3_000, "distance = {} (expected < 3000 for similar text)", dist);
```

---

## Example 3 — Dissimilar Texts Have High Hamming Distance

```rust
<!-- source: crates/bardo-primitives/tests/hdc_examples.rs -->

let encoder = HdcEncoder { version: 1 };
let fp1 = encoder.encode(&Body::Text("quantum mechanics and wave functions".into())).unwrap();
let fp2 = encoder.encode(&Body::Text("chocolate cake recipe with frosting".into())).unwrap();

let dist = fp1.vector.hamming_distance(&fp2.vector);
// Semantically unrelated → distance should be near 5120
assert!(dist > 4_000, "distance = {} (expected > 4000 for unrelated text)", dist);
```

---

## Example 4 — Near-Duplicate Detection

```rust
<!-- source: crates/bardo-primitives/tests/hdc_examples.rs -->

let encoder = HdcEncoder { version: 1 };
let fp_original = encoder.encode(&Body::Text("Roko stores knowledge as Engrams".into())).unwrap();
let fp_paraphrase = encoder.encode(&Body::Text("Engrams are how Roko stores knowledge".into())).unwrap();
let fp_unrelated = encoder.encode(&Body::Text("Python is a programming language".into())).unwrap();

// Paraphrase should be a near-duplicate
assert!(fp_original.vector.is_near_duplicate(&fp_paraphrase.vector),
    "paraphrase should be near-duplicate");

// Unrelated content should not be near-duplicate
assert!(!fp_original.vector.is_near_duplicate(&fp_unrelated.vector),
    "unrelated content should not be near-duplicate");
```

---

## Example 5 — Cross-Version Comparison Returns None

```rust
<!-- source: crates/bardo-primitives/tests/hdc_examples.rs -->

let encoder_v1 = HdcEncoder { version: 1 };
let encoder_v2 = HdcEncoder { version: 2 };
let body = Body::Text("test content".into());

let fp1 = encoder_v1.encode(&body).unwrap();
let fp2 = encoder_v2.encode(&body).unwrap();

// Different versions → similarity_checked returns None
assert!(fp1.similarity_checked(&fp2).is_none());

// Same version → returns Some
let fp1b = encoder_v1.encode(&body).unwrap();
assert!(fp1.similarity_checked(&fp1b).is_some());
```

---

## Example 6 — Re-encoding Does Not Change ContentHash

```rust
<!-- source: crates/bardo-primitives/tests/hdc_examples.rs -->

let original = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(Body::Text("HDC fingerprinting".into()))
    .provenance(Provenance::local_agent("001"))
    .build()?;

let original_id = original.id;
let original_fp_version = original.fingerprint.as_ref().map(|f| f.encoder_version);

// Re-encode with v2
let encoder_v2 = HdcEncoder { version: 2 };
let new_fp = reencode(&original, &encoder_v2);

// ContentHash is unchanged
assert_eq!(original_id, original.id);

// New fingerprint is v2
assert_eq!(new_fp.unwrap().encoder_version, 2);
assert_ne!(original_fp_version, Some(2));
```

---

## Example 7 — Binary Body Has No Fingerprint

```rust
<!-- source: crates/bardo-primitives/tests/hdc_examples.rs -->

let encoder = HdcEncoder { version: 1 };
let binary_body = Body::Binary(vec![0x89, 0x50, 0x4E, 0x47]);  // PNG header bytes
assert!(encoder.encode(&binary_body).is_none());
```

---

## Example 8 — Majority Bundle of Token Vectors

```rust
<!-- source: crates/bardo-primitives/tests/hdc_examples.rs -->

let mut rng = rand::thread_rng();
let v1 = HdcVector::random(&mut rng);
let v2 = HdcVector::random(&mut rng);
let v3 = HdcVector::random(&mut rng);

let bundle = majority_bundle(&[&v1, &v2, &v3]);
// Bundle should be approximately 50% ones (balanced)
let pc = bundle.popcount();
assert!(pc > 4_500 && pc < 5_700);

// Bundle of identical vectors should equal the vector
let bundle_same = majority_bundle(&[&v1, &v1, &v1]);
assert_eq!(bundle_same, v1);
```

---

## Open Questions

None at this time.

## See Also

- [`05-invariants.md`](05-invariants.md) — invariants demonstrated by these examples
- [`03-similarity-distance.md`](03-similarity-distance.md) — thresholds used in examples
- [`04-encoder-versioning.md`](04-encoder-versioning.md) — cross-version handling in examples
