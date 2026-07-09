# ContentHash — Examples

> Worked examples for ContentHash construction, verification, edge cases, and hex encoding.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## Example 1 — Basic Hash Computation

```rust
<!-- source: crates/roko-core/tests/content_hash_examples.rs -->

let engram = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(Body::Text("BLAKE3 is a fast cryptographic hash function.".into()))
    .provenance(Provenance::local_agent("001"))
    .created_at_ms(1_700_000_000_000)
    .build()?;

// ContentHash is computed in build()
let hash = engram.id;
println!("hash: {}", hash);  // 64-char hex string
assert_eq!(hash.as_bytes().len(), 32);
```

---

## Example 2 — Determinism (same input, same hash)

```rust
<!-- source: crates/roko-core/tests/content_hash_examples.rs -->

let e1 = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(Body::Text("deterministic".into()))
    .provenance(Provenance::local_agent("001"))
    .created_at_ms(1_700_000_000_000)
    .build()?;

let e2 = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(Body::Text("deterministic".into()))
    .provenance(Provenance::local_agent("001"))
    .created_at_ms(1_700_000_000_000)
    .build()?;

assert_eq!(e1.id, e2.id);
```

---

## Example 3 — Different Authors → Different Hashes

```rust
let e1 = make_engram("text", "agent-001", 1_700_000_000_000);
let e2 = make_engram("text", "agent-002", 1_700_000_000_000);
assert_ne!(e1.id, e2.id);
```

---

## Example 4 — Mutable Field Changes Preserve Hash

```rust
<!-- source: crates/roko-core/tests/content_hash_examples.rs -->

let engram = make_test_engram();
let original_id = engram.id;

// Mutate decay (excluded from hash)
let mut mutated = engram.clone();
if let Decay::Demurrage(ref mut p) = mutated.decay {
    p.balance = 0.01;  // nearly expired
}
assert_eq!(original_id, ContentHash::compute(&mutated));

// Mutate score (excluded from hash)
mutated.score.confidence = 0.99;
assert_eq!(original_id, ContentHash::compute(&mutated));

// Escalate trust (excluded from hash)
mutated.provenance.trust = TrustLevel::ChainWitness;
assert_eq!(original_id, ContentHash::compute(&mutated));
```

---

## Example 5 — Hex Encoding and Decoding

```rust
<!-- source: crates/roko-core/tests/content_hash_examples.rs -->

let engram = make_test_engram();
let hex = engram.id.to_hex();
assert_eq!(hex.len(), 64);
assert!(hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));

let recovered = ContentHash::from_hex(&hex).unwrap();
assert_eq!(engram.id, recovered);
```

---

## Example 6 — Tamper Detection via verify()

```rust
<!-- source: crates/roko-core/tests/content_hash_examples.rs -->

let engram = make_test_engram();
// verify() passes for the genuine Engram
assert!(engram.id.verify(&engram));

// Tampering: add a tag not in the original
let mut tampered = engram.clone();
tampered.tags.insert("injected".into(), "data".into());
// Stored id is engram.id; but compute(tampered) differs
assert!(!engram.id.verify(&tampered));
```

---

## Example 7 — Lineage Changes the Hash

```rust
<!-- source: crates/roko-core/tests/content_hash_examples.rs -->

let parent = make_test_engram();

let child_no_lineage = Engram::builder()
    .kind(Kind::Reflection)
    .body(Body::Text("insight".into()))
    .provenance(Provenance::local_agent("001"))
    .created_at_ms(1_700_000_001_000)
    .build()?;

let child_with_lineage = Engram::builder()
    .kind(Kind::Reflection)
    .body(Body::Text("insight".into()))
    .provenance(Provenance::local_agent("001"))
    .created_at_ms(1_700_000_001_000)
    .lineage(vec![parent.id])
    .build()?;

// Adding a parent to lineage changes the hash
assert_ne!(child_no_lineage.id, child_with_lineage.id);
```

---

## Example 8 — from_hex Validation

```rust
<!-- source: crates/roko-core/tests/content_hash_examples.rs -->

// Too short
assert!(ContentHash::from_hex("abc").is_err());

// Uppercase (rejected)
let bad_upper = "A3F2B9".repeat(11);  // 66 chars — also wrong length
assert!(ContentHash::from_hex(&bad_upper).is_err());

// Correct length, correct format
let good: String = "a3".repeat(32);  // 64 chars
assert!(ContentHash::from_hex(&good).is_ok());
```

---

## Open Questions

None at this time.

## See Also

- [`01-canonical-encoding.md`](01-canonical-encoding.md) — exact encoding used in examples
- [`02-api-reference.md`](02-api-reference.md) — method signatures
- [`03-invariants.md`](03-invariants.md) — invariants demonstrated by these examples
