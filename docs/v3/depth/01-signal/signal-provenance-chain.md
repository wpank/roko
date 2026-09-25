# Signal Provenance Chain

> Parent hash chains, attestation, lineage DAG, taint tracking, and the
> Information-Flow Control lattice that governs trust propagation.

**Sources**: `crates/roko-core/src/provenance.rs`, `crates/roko-core/src/attestation.rs`,
`crates/roko-core/src/signal.rs`

---

## 1. Lineage DAG

Every Signal carries `lineage: Vec<ContentHash>` -- the content hashes of
the Signals it was derived from. This forms a directed acyclic graph (DAG)
for:

- **Audit trails**: trace a gate verdict back to the task prompt and agent
  output that produced it.
- **Autocatalytic metrics**: measure how often a Signal's descendants
  succeed, feeding back into its utility score.
- **Taint propagation**: if an upstream Signal is tainted, downstream
  Signals inherit the taint classification.

### Building Lineage

```rust
// Explicit lineage via builder
let derived = Signal::builder(Kind::GateVerdict)
    .body(Body::text("pass"))
    .lineage([parent_signal.id])
    .build();

// Convenience method: derive() adds self to lineage
let child = parent.derive(Kind::Episode, Body::text("logged"));

// Full chain: derive_verdict() copies all parent lineage + self
let verdict = parent.derive_verdict(Body::text("compile pass"));
```

`Signal::derive()` creates a new builder with `[self.id]` in lineage and
`Provenance::agent("derived")` with the parent's effective taint level.

`Signal::derive_verdict()` extends the lineage to include the parent's
entire chain plus itself (deduplicating). It also copies all parent tags
and applies `Decay::GATE_VERDICT`.

---

## 2. Provenance

Every Signal carries a `Provenance` record answering three questions:
**who produced it**, **how trusted is that producer**, and **is the data
tainted**.

```rust
pub struct Provenance {
    pub author:        String,
    pub trust:         f32,                    // [0..1]
    pub taint:         Taint,
    pub taint_info:    Option<TaintInfo>,       // deprecated legacy
    pub session:       Option<String>,
    pub taint_level:   TaintLevel,
    pub trust_origin:  TrustOriginTaintLevel,
}
```

### Constructors

| Constructor | trust | taint | trust_origin | Use case |
|---|---|---|---|---|
| `Provenance::trusted(author)` | `1.0` | `Clean` | `Trusted` | Gates, composers, orchestrator |
| `Provenance::agent(author)` | `0.75` | `Clean` | `Local` | Internal agent output |
| `Provenance::user(author)` | `0.5` | `UserInput` | `Local` | Human input (tainted for safety) |
| `Provenance::external(author)` | `0.1` | `UnverifiedSource` | `External` | Webhooks, APIs, chain data |

### Trust Checking

```rust
// Boolean check: trust >= threshold AND not tainted
provenance.is_trusted(0.5)  // true for agents, false for external

// Effective taint level (joins taint variant and explicit level)
provenance.effective_taint()  // TaintLevel

// Effective trust origin (joins explicit origin and taint reason)
provenance.effective_trust_origin()  // TrustOriginTaintLevel
```

---

## 3. Taint Classification

### Taint Enum

Nine typed variants replacing the old `tainted: bool`:

```rust
#[non_exhaustive]
pub enum Taint {
    Clean,
    LlmHallucination { detail: String },
    ToolFailure { detail: String },
    UserFlagged { detail: String },
    StaleData { threshold_ms: i64 },
    UnverifiedSource { detail: String },
    Propagated { detail: String, inherited_from: Option<ContentHash> },
    UserInput { detail: String },
    Custom(String),
}
```

| Method | Returns | Purpose |
|---|---|---|
| `is_tainted()` | `bool` | `true` for any variant except `Clean` |
| `category()` | `&str` | Machine-readable label for logging |
| `detail()` | `Option<&str>` | Human-readable explanation |
| `inherited_from()` | `Option<&ContentHash>` | Upstream taint source hash |

### Taint Propagation

When a Signal is derived from a tainted parent, the taint propagates:

```rust
let tainted_parent = Signal::builder(Kind::Task)
    .provenance(Provenance::external("webhook"))
    .build();

// derive() carries forward the parent's effective taint level
let child = tainted_parent.derive(Kind::Episode, Body::text("processed"));
// child.provenance.taint_level >= parent.provenance.effective_taint()
```

The `Propagated` taint variant explicitly records the upstream hash:
```rust
Taint::Propagated {
    detail: "inherited from webhook signal".into(),
    inherited_from: Some(parent.id),
}
```

---

## 4. Information-Flow Control Lattice

### TaintLevel (Data Classification)

Four-tier security classification with `join` (least upper bound) and
`meet` (greatest lower bound) operations:

```
Public < Internal < Confidential < Secret
```

```rust
pub enum TaintLevel {
    Public = 0,       // unrestricted
    Internal = 1,     // internal use only
    Confidential = 2, // sensitive business data
    Secret = 3,       // strict access control
}
```

`can_flow_to(target)` enforces the no-write-down rule: data can only
flow to equally or more classified contexts.

```rust
TaintLevel::Public.can_flow_to(TaintLevel::Secret)       // true
TaintLevel::Secret.can_flow_to(TaintLevel::Public)       // false
TaintLevel::Confidential.can_flow_to(TaintLevel::Confidential) // true
```

### TrustOriginTaintLevel (CaMeL Origin)

Orthogonal to TaintLevel. Tracks **who** produced the data rather than
**how sensitive** it is:

```
Trusted < Local < External < Untrusted
```

This is the CaMeL (Caution against Malicious Language) trust-origin tag.
`join` returns the less-trusted of two origins.

### Effective Taint

`Provenance::effective_taint()` combines the `Taint` variant and the
explicit `taint_level` field:

| Taint variant | Implied minimum level |
|---|---|
| `Clean` | `Public` (defers to explicit level) |
| `UserInput`, `Propagated` | `Internal` |
| All others (`LlmHallucination`, `ToolFailure`, `UnverifiedSource`, etc.) | `Confidential` |

The result is `join(taint_level, implied_minimum)` -- the explicit level
is never downgraded by a clean taint variant.

---

## 5. Attestation

Optional Ed25519 cryptographic proof of origin layered on top of content
identity. Intentionally **excluded** from the content hash so the same
Signal can be attested after creation.

```rust
pub struct Attestation {
    pub signature:         Ed25519Signature,    // 64 bytes
    pub public_key:        PublicKey,           // 32 bytes
    pub chain_attestation: Option<ChainAttestation>,
}

pub struct ChainAttestation {
    pub chain_id:     u64,
    pub tx_hash:      [u8; 32],
    pub block_number: u64,
}
```

### Signing and Verification

```rust
use roko_core::attestation::{sign, verify};

let attestation = sign(&signal, &signing_key);
assert!(verify(&signal, &attestation));
```

`sign()` computes the Signal's content hash and signs it with Ed25519.
`verify()` recomputes the hash and checks the signature. Tampered
content (different hash) fails verification.

### Witness Hash

`attestation.witness_hash()` hashes the signature + public key,
excluding `chain_attestation`. This provides a stable proof identifier
before and after on-chain publication.

### Chain Attestation

An optional on-chain witness that the Signal hash existed at a specific
block. Attached post-signing via `with_chain_attestation()`. The witness
hash is stable across attachment.

---

## 6. Provenance Coherence

`Provenance::coherence_check()` validates internal consistency:

```rust
pub struct ProvenanceCoherenceCheck {
    pub issues: Vec<ProvenanceCoherenceIssue>,
}

pub enum ProvenanceCoherenceIssue {
    MissingAuthor,
}
```

`coherence_score()` returns `(1.0 - 0.25 * issues.len()).clamp(0.0, 1.0)`.
`is_coherent()` returns `true` when no issues are found.

Currently checks only for blank authors. Future versions may add checks
for trust/taint consistency, session validity, etc.

---

## 7. Content Hash and Provenance Interaction

The content hash includes the author string and the taint boolean but
**not** the trust score, taint_level, or trust_origin. This means:

- Same author + same taint status = same hash (regardless of trust score)
- Different authors = different hash
- Tainted vs untainted = different hash
- Different taint levels = same hash (classification is mutable metadata)

This design allows trust scores and classification levels to evolve
without changing Signal identity.

---

## 8. Lineage Walking

To walk the full provenance chain, query the Store for each hash in
the lineage:

```rust
async fn walk_lineage(store: &dyn Store, signal: &Signal) -> Vec<Signal> {
    let mut chain = Vec::new();
    for hash in &signal.lineage {
        if let Ok(Some(parent)) = store.get(hash).await {
            chain.push(parent);
        }
    }
    chain
}
```

The CLI provides `roko replay <hash>` for interactive DAG walking.

---

## 9. Source Files

| File | Contents |
|---|---|
| `crates/roko-core/src/provenance.rs` | Provenance, Taint, TaintLevel, TaintInfo, TrustOriginTaintLevel |
| `crates/roko-core/src/attestation.rs` | Attestation, sign, verify, ChainAttestation |
| `crates/roko-core/src/signal.rs` | Signal.lineage, derive(), derive_verdict() |
| `crates/roko-core/src/extension.rs` | CamelTaintLevel (aliased as TrustOriginTaintLevel) |
| `crates/roko-core/src/hash.rs` | ContentHash |
