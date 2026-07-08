# Provenance — Examples

> Worked examples for Provenance construction, trust escalation, taint propagation, and hash contracts.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## Example 1 — Local Agent Provenance (simplest case)

```rust
<!-- source: crates/roko-core/tests/provenance_examples.rs -->

let prov = Provenance::local_agent("007");
assert_eq!(prov.author, "agent-007");
assert_eq!(prov.trust, TrustLevel::LocalAgent);
assert!(prov.taint.is_none());
assert_eq!(prov.trust_weight(), 0.25);
```

---

## Example 2 — Full Constructor with Initial Taint

```rust
<!-- source: crates/roko-core/tests/provenance_examples.rs -->

let prov = Provenance::new(
    "external-ingest-42",
    TrustLevel::LocalAgent,
    Some(BTreeSet::from([TaintFlag::UnverifiedSource])),
)?;
assert!(prov.is_tainted());
assert!(prov.has_flag(&TaintFlag::UnverifiedSource));
```

---

## Example 3 — Trust Escalation to PeerVerified

```rust
<!-- source: crates/roko-core/tests/provenance_examples.rs -->

let mut prov = Provenance::local_agent("001");
// Agent self-verifies
prov.escalate_trust(TrustLevel::SelfVerified)?;
assert_eq!(prov.trust, TrustLevel::SelfVerified);

// Two peers confirm
prov.escalate_trust(TrustLevel::PeerVerified)?;
assert_eq!(prov.trust, TrustLevel::PeerVerified);
assert_eq!(prov.trust_weight(), 0.75);
```

---

## Example 4 — Attempted Trust Downgrade (rejected)

```rust
<!-- source: crates/roko-core/tests/provenance_examples.rs -->

let mut prov = Provenance {
    author: "agent-001".into(),
    trust: TrustLevel::PeerVerified,
    taint: None,
};
let result = prov.escalate_trust(TrustLevel::LocalAgent);
assert!(matches!(result, Err(TrustError::CannotDowngrade { .. })));
// Trust is unchanged
assert_eq!(prov.trust, TrustLevel::PeerVerified);
```

---

## Example 5 — Adding Taint After Creation

```rust
<!-- source: crates/roko-core/tests/provenance_examples.rs -->

let mut prov = Provenance::local_agent("001");
assert!(!prov.is_tainted());

prov.add_taint(TaintFlag::PossibleHallucination);
assert!(prov.is_tainted());
assert!(prov.has_flag(&TaintFlag::PossibleHallucination));

// Adding the same flag again is idempotent (BTreeSet deduplicates)
prov.add_taint(TaintFlag::PossibleHallucination);
assert_eq!(prov.taint.as_ref().unwrap().len(), 1);
```

---

## Example 6 — Taint Propagation Through Lineage

```rust
<!-- source: crates/roko-core/tests/provenance_examples.rs -->

let parent_a = Provenance {
    author: "agent-001".into(),
    trust: TrustLevel::PeerVerified,
    taint: Some(BTreeSet::from([TaintFlag::CompromisedAuthor])),
};

let parent_b = Provenance {
    author: "external-99".into(),
    trust: TrustLevel::LocalAgent,
    taint: Some(BTreeSet::from([TaintFlag::UnverifiedSource])),
};

let inherited = Provenance::inherit_taint(&[&parent_a, &parent_b]);

// CompromisedAuthor propagates; UnverifiedSource does not
let flags = inherited.expect("should have flags");
assert!(flags.contains(&TaintFlag::CompromisedAuthor));
assert!(!flags.contains(&TaintFlag::UnverifiedSource));
```

---

## Example 7 — Hash Stability Under Trust Escalation

```rust
<!-- source: crates/roko-core/tests/provenance_examples.rs -->

let body = Body::Text("The sky is blue".into());

let e1 = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(body.clone())
    .provenance(Provenance::local_agent("001"))
    .build()?;

// Same Engram after trust escalation
let mut prov2 = Provenance::local_agent("001");
prov2.escalate_trust(TrustLevel::PeerVerified)?;
let e2 = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(body.clone())
    .provenance(prov2)
    .build()?;

// Different trust, same hash
assert_eq!(e1.id, e2.id);
```

---

## Example 8 — Hash Distinguishes Different Authors

```rust
<!-- source: crates/roko-core/tests/provenance_examples.rs -->

let body = Body::Text("The sky is blue".into());
let e1 = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(body.clone())
    .provenance(Provenance::local_agent("001"))
    .build()?;
let e2 = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(body.clone())
    .provenance(Provenance::local_agent("002"))
    .build()?;

// Different authors → different hashes, even for identical content
assert_ne!(e1.id, e2.id);
```

---

## Example 9 — OutdatedAt Flag Pointing to Successor

```rust
<!-- source: crates/roko-core/tests/provenance_examples.rs -->

// Create successor first
let newer = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(Body::Text("Revised: the sky is blue (corrected)".into()))
    .provenance(Provenance::local_agent("001"))
    .build()?;

// Flag original as outdated
let mut prov_old = Provenance::local_agent("001");
prov_old.add_taint(TaintFlag::OutdatedAt { superseded_by: newer.id });

let older = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(Body::Text("The sky is blue".into()))
    .provenance(prov_old)
    .build()?;

// Gate will redirect queries for older to newer
assert!(older.provenance.has_flag(
    &TaintFlag::OutdatedAt { superseded_by: newer.id }
));
```

---

## Example 10 — Substrate Peer Verification Flow

```rust
<!-- source: crates/roko-fs/tests/trust_escalation.rs -->

// 1. Agent creates an Engram
let engram = make_knowledge_engram("Roko is a cognitive runtime", "agent-001");
substrate.put(engram.clone())?;

// 2. Agent self-verifies
self_verify(&substrate, &engram.id, "001", SelfCheckResult::pass())?;
let e = substrate.get(&engram.id)?;
assert_eq!(e.provenance.trust, TrustLevel::SelfVerified);

// 3. Two peers confirm
peer_verify(&substrate, &engram.id, "peer-agent-002", PeerVerdict::Confirm)?;
peer_verify(&substrate, &engram.id, "peer-agent-003", PeerVerdict::Confirm)?;
// Quorum reached (PEER_VERIFY_QUORUM = 2)
let e = substrate.get(&engram.id)?;
assert_eq!(e.provenance.trust, TrustLevel::PeerVerified);
assert_eq!(e.id, engram.id);  // hash unchanged throughout
```

---

## Open Questions

None at this time.

## See Also

- [`07-invariants.md`](07-invariants.md) — all invariants exercised by these examples
- [`08-api-reference.md`](08-api-reference.md) — method signatures
- [`06-trust-escalation.md`](06-trust-escalation.md) — trust escalation protocol
