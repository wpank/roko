# Signal Type Anatomy

> Field-by-field documentation of the `Signal` struct, the single noun of the
> Roko system. Every event, datum, agent output, and gate verdict is a Signal.

**Source**: `crates/roko-core/src/signal.rs`

---

## 1. The Struct

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub id:              ContentHash,
    pub fingerprint:     Option<HdcFingerprint>,
    pub kind:            Kind,
    pub body:            Body,
    pub created_at_ms:   i64,
    pub decay:           Decay,
    pub provenance:      Provenance,
    pub score:           Score,
    pub lineage:         Vec<ContentHash>,
    pub tags:            BTreeMap<String, String>,
    pub attestation:     Option<Attestation>,
    pub emotional_tag:   Option<EmotionalTag>,
    pub balance:         f64,
    pub status:          SignalStatus,
    pub access_count:    u32,
    pub demurrage_paid:  f64,
}
```

The backward-compatible alias `pub type Engram = Signal` is defined in
`crates/roko-core/src/engram.rs`, which re-exports everything from
`crate::signal`.

---

## 2. Field Reference

### 2.1 `id: ContentHash`

Content-addressed identity. A BLAKE3 hash of the Signal's identity fields:
kind, body, author, taint flag, lineage, and tags. Score, decay, timestamp,
attestation, and emotional metadata are **excluded** so they can change
without changing identity.

`ContentHash` is a `[u8; 32]` wrapper defined in `crates/roko-core/src/hash.rs`.
It serializes as a hex string.

### 2.2 `fingerprint: Option<HdcFingerprint>`

Hyperdimensional Computing semantic vector plus encoder version metadata.
Used for similarity search (`Store::query_similar`), clustering, and the
HDC bind/bundle/permute algebra.

```rust
pub struct HdcFingerprint {
    pub vector: HdcVector,       // roko-primitives, 128-dim binary vector
    pub encoder_version: u32,    // monotonic; currently ENCODER_VERSION_TEXT_V1 = 1
}
```

Computed lazily via `Signal::compute_fingerprint()` or
`Signal::ensure_fingerprint()`. Text and JSON bodies are encoded via
`HdcVector::from_seed(body_bytes)`. Empty bodies produce a zero vector.
Binary bodies are skipped. Optional in serialization (`skip_serializing_if`).

### 2.3 `kind: Kind`

What this Signal represents. Determines how the body should be interpreted
and which protocol implementations can consume it. Defined in
`crates/roko-core/src/kind.rs` as a `#[non_exhaustive]` enum with 27
built-in variants across six groups:

| Group | Variants |
|---|---|
| Agent runtime | `ProcessSpawn`, `ProcessExit`, `AgentMessage`, `AgentOutput`, `TokenUsage`, `ApprovalRequested` |
| Verification | `GateVerdict`, `TestResult`, `CompileDiagnostic` |
| Tasks and plans | `Task`, `Plan`, `PlanPhase` |
| Context assembly | `PromptSection`, `ContextPack`, `Prompt` |
| Routing and learning | `RouterChoice`, `RouterFeedback` |
| Memory | `Episode`, `PlaybookRule`, `Skill`, `Compound(Vec<Kind>)` |
| Observability | `Metric`, `ExperimentResult`, `ToolInvocation`, `ToolHealthDegraded` |
| Chain | `Insight`, `Pheromone`, `Bounty`, `Transaction`, `Service`, `Prediction` |
| Extension | `Custom(String)` |

`Compound(Vec<Kind>)` allows a single Signal to carry multiple semantics.
`Kind::matches()` supports containment checks.

### 2.4 `body: Body`

The Signal's payload. An enum with four variants:

```rust
pub enum Body {
    Empty,                                   // marker signal
    Text(String),                            // UTF-8 text
    Json(serde_json::Value),                 // structured JSON
    Bytes(#[serde(with = "base64_bytes")] Vec<u8>),  // raw binary
}
```

`Body::canonical_bytes()` returns the deterministic byte representation
used in content hashing. `Bytes` are base64-encoded in JSON serialization.

### 2.5 `created_at_ms: i64`

Unix millisecond timestamp of first emission. Set by `SignalBuilder` to
`chrono::Utc::now().timestamp_millis()` unless pinned with
`.created_at_ms(t)`. Used by `Decay::apply()` to compute age.

### 2.6 `decay: Decay`

Time-based weight function. See `signal-lifecycle-and-serialization.md`
for full Decay documentation. Four variants:

| Variant | Formula | Use case |
|---|---|---|
| `None` | `1.0` always | Config, schemas, identity signals |
| `HalfLife { half_life_ms }` | `0.5^(age/hl)` | Pheromones, verdicts |
| `Ttl { ttl_ms }` | `1.0` before TTL, `0.0` after | Offers, bounties |
| `Ebbinghaus { strength, scale_ms }` | `exp(-age/(strength*scale))` | Memory entries |

### 2.7 `provenance: Provenance`

Producer attribution and trust metadata. See `signal-provenance-chain.md`
for full documentation. Key fields:

- `author: String` -- identifier of the producer
- `trust: f32` -- trust score in `[0.0, 1.0]`
- `taint: Taint` -- typed contamination classification (9 variants)
- `taint_level: TaintLevel` -- IFC security classification lattice
- `trust_origin: TrustOriginTaintLevel` -- CaMeL trust-origin tag

### 2.8 `score: Score`

Seven-axis quality assessment. See `signal-scoring-dimensions.md` for
the full formula. Default is `Score::NEUTRAL` (confidence 0.5, reputation 1.0,
all others zero).

### 2.9 `lineage: Vec<ContentHash>`

Content hashes of parent Signals that this Signal derived from. Forms a
DAG for audit trails and autocatalytic metrics. See
`signal-provenance-chain.md`.

### 2.10 `tags: BTreeMap<String, String>`

Arbitrary string metadata. `BTreeMap` guarantees stable iteration order
for deterministic content hashing. Tags are included in the content hash.
Common tags: `plan_id`, `task_id`, `gate`, `pulse_topic`, `pulse_seq`.

### 2.11 `attestation: Option<Attestation>`

Optional Ed25519 signature over the Signal's content hash. Defined in
`crates/roko-core/src/attestation.rs`. Carries a `PublicKey`, an
`Ed25519Signature`, and an optional `ChainAttestation` for on-chain
timestamped publication. Excluded from the content hash so attestation
can be added post-creation.

### 2.12 `emotional_tag: Option<EmotionalTag>`

Optional PAD (Pleasure-Arousal-Dominance) vector from the affect engine
(`roko-daimon`). Carries `pad: PadVector`, `intensity: f32`,
`trigger: String`, and `mood_snapshot: PadVector`. Used by the Daimon
subsystem for affect-modulated dispatch.

### 2.13 `balance: f64`

Demurrage balance in `[0.0, 1.0]`. Starts at `1.0`. Decays over time via
the demurrage tick; refreshed to `1.0` on access via `Signal::touch()`.
When the balance reaches zero, the Signal is eligible for GC.

### 2.14 `status: SignalStatus`

Lifecycle tier. Four monotonically increasing tiers:

```
Transient -> Working -> Consolidated -> Persistent
```

Each tier has a different retention guarantee. Graduation is forward-only.
See `signal-lifecycle-and-serialization.md` for transition rules.

### 2.15 `access_count: u32`

Number of times this Signal has been accessed. Incremented by
`Signal::touch()`. Used as a precondition for
`promote_to_persistent(min_age_secs, min_accesses)`.

### 2.16 `demurrage_paid: f64`

Cumulative demurrage paid over the Signal's lifetime. Monotonically
increasing -- even if novelty gains partially offset decay, this field
only grows. Used for economic accounting.

---

## 3. Construction

Signals are built via `Signal::builder(kind: Kind)`:

```rust
let signal = Signal::builder(Kind::Task)
    .body(Body::text("implement login"))
    .provenance(Provenance::agent("planner"))
    .score(Score::new(0.9, 0.5, 0.0, 1.0))
    .decay(Decay::HalfLife { half_life_ms: 86_400_000 })
    .tag("plan_id", "plan-42")
    .lineage([parent.id])
    .build();
```

The builder defaults: `Body::Empty`, current time, `Decay::None`,
`Provenance::trusted("roko")`, `Score::NEUTRAL`, empty lineage,
`balance = 1.0`, `status = Transient`, `access_count = 0`.

The `id` field is computed at build time via `Signal::content_hash()`.

---

## 4. Key Methods

| Method | Returns | Purpose |
|---|---|---|
| `content_hash()` | `ContentHash` | Recompute identity from kind+body+author+taint+lineage+tags |
| `weight_at(now_ms)` | `f32` | `score.effective() * decay.apply(age)` |
| `age_ms(now_ms)` | `i64` | `(now_ms - created_at_ms).max(0)` |
| `touch()` | `()` | Reset balance to 1.0, increment access_count |
| `promote_to_working(min_score)` | `Result<(), GraduationError>` | Transient -> Working |
| `promote_to_consolidated()` | `Result<(), GraduationError>` | Working -> Consolidated |
| `promote_to_persistent(min_age, min_accesses)` | `Result<(), GraduationError>` | Consolidated -> Persistent |
| `compute_fingerprint()` | `()` | Set HDC fingerprint from body content |
| `ensure_fingerprint()` | `()` | Idempotent fingerprint computation |
| `to_pulse(topic, seq)` | `Pulse` | Lossy projection to ephemeral Bus transport |
| `derive(kind, body)` | `SignalBuilder` | New Signal with this one in lineage |
| `derive_verdict(body)` | `SignalBuilder` | Derived gate verdict with full lineage |
| `from_pulse_synthetic(pulse)` | `Signal` | Quick lossy promotion (no audit tags) |
| `from_pulses(pulses)` | `Signal` | Batch summary from multiple Pulses |
| `bind(other)` | `Option<HdcVector>` | HDC bind of two fingerprints |
| `bundle(signals)` | `Option<HdcVector>` | HDC consensus of multiple fingerprints |
| `at_position(n)` | `Option<HdcVector>` | Positional permutation of fingerprint |
| `tag(key)` | `Option<&str>` | Look up a tag value |
| `is(kind)` | `bool` | Kind equality check |
| `effective_taint()` | `TrustOriginTaintLevel` | Effective trust-origin taint |

---

## 5. Verification Commands

```bash
# Confirm Signal struct fields
grep 'pub ' crates/roko-core/src/signal.rs | head -20

# Run Signal-related tests
cargo test -p roko-core signal -- --nocapture
cargo test -p roko-core engram -- --nocapture
```

---

## 6. Source Files

| File | Contents |
|---|---|
| `crates/roko-core/src/signal.rs` | Signal struct, SignalBuilder, SignalStatus, GraduationError, HdcFingerprint |
| `crates/roko-core/src/engram.rs` | `pub use crate::signal::*` backward-compat re-export |
| `crates/roko-core/src/hash.rs` | ContentHash newtype (`[u8; 32]`) |
| `crates/roko-core/src/kind.rs` | Kind enum (27 variants + Custom + Compound) |
| `crates/roko-core/src/body.rs` | Body enum (Empty, Text, Json, Bytes) |
| `crates/roko-core/src/score.rs` | Score struct (7 axes) |
| `crates/roko-core/src/decay.rs` | Decay enum (4 variants) |
| `crates/roko-core/src/provenance.rs` | Provenance, Taint, TaintLevel, TaintInfo |
| `crates/roko-core/src/attestation.rs` | Attestation, Ed25519Signature, PublicKey, ChainAttestation |
| `crates/roko-core/src/affect.rs` | EmotionalTag, PadVector |
