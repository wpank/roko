# Engram — Builder Pattern

> EngramBuilder is the only supported way to construct a new Engram. It computes the ContentHash and fingerprint, enforces invariants, and provides ergonomic defaults.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Struct reference](01-struct-reference.md)  
**Used by**: every component that emits Engrams  
**Last reviewed**: 2026-04-19

---

## TL;DR

Never construct `Engram` directly. Use `EngramBuilder`. The builder computes the
`id` and `fingerprint` automatically, validates Kind-Body consistency, and fills
in defaults for optional fields. All required fields must be set before `build()`
or the call returns `Err`.

---

## The Idea

`Engram` has ten fields, most of which have sensible defaults. Direct struct
construction forces callers to supply all fields explicitly, or forget one and get
a wrong `id` because the hash computation wasn't run. `EngramBuilder` makes the
right thing easy and the wrong thing a compile error.

The builder also centralises the hash and fingerprint computation. There is exactly
one place in the codebase where `id = blake3(...)` is called for Engrams: inside
`EngramBuilder::build()`.

---

## Specification

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

pub struct EngramBuilder {
    kind: Option<Kind>,
    body: Option<Body>,
    decay: Decay,                            // default: Decay::default()
    provenance: Provenance,                  // default: Provenance::anonymous()
    score: Score,                            // default: Score::default()
    lineage: Vec<ContentHash>,               // default: empty
    tags: BTreeMap<String, String>,          // default: empty
    created_at_ms: Option<i64>,              // default: SystemTime::now()
    skip_fingerprint: bool,                  // default: false
}

impl EngramBuilder {
    pub fn new() -> Self { /* ... */ }

    // --- Required ---
    pub fn kind(mut self, kind: Kind) -> Self;
    pub fn body(mut self, body: Body) -> Self;

    // --- Optional ---
    pub fn decay(mut self, decay: Decay) -> Self;
    pub fn provenance(mut self, provenance: Provenance) -> Self;
    pub fn score(mut self, score: Score) -> Self;
    pub fn lineage(mut self, lineage: Vec<ContentHash>) -> Self;
    pub fn parent(mut self, parent: ContentHash) -> Self;       // adds one parent
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn tags(mut self, tags: BTreeMap<String, String>) -> Self;
    pub fn created_at_ms(mut self, ms: i64) -> Self;
    pub fn skip_fingerprint(mut self) -> Self;                  // test use only

    // --- Terminal ---
    pub fn build(self) -> Result<Engram, EngramBuildError>;
}
```

### Required Fields

| Field | Why required |
|-------|-------------|
| `kind` | Determines Body interpretation; no sensible default |
| `body` | The payload; no sensible default |

### Defaults for Optional Fields

| Field | Default |
|-------|---------|
| `created_at_ms` | `SystemTime::now()` as Unix ms |
| `decay` | `Decay::Demurrage(DemurrageParams::default())` |
| `provenance` | `Provenance::anonymous()` — must be overridden for production Engrams |
| `score` | `Score::default()` — all axes at 0.5 |
| `lineage` | `vec![]` — root Engram |
| `tags` | `BTreeMap::new()` — no metadata |
| `skip_fingerprint` | `false` — compute fingerprint |

---

## Semantics

### build() Steps

```
1. Validate: kind and body are set
2. Validate: body variant matches kind
3. Validate: created_at_ms > 0
4. Validate: lineage entries are distinct and do not contain the computed id
5. Compute id = blake3(canonical_encode(kind, body, created_at_ms, provenance.author, lineage, tags))
6. Compute fingerprint = hdc_encode(kind, body) unless skip_fingerprint = true
7. Return Ok(Engram { id, fingerprint, kind, body, created_at_ms, decay, provenance, score, lineage, tags })
```

Step 5 and 6 happen after all validation. If validation fails, no hash is computed.

### EngramBuildError

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

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
```

---

## Examples

### Minimal AgentOutput

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

let engram = EngramBuilder::new()
    .kind(Kind::AgentOutput)
    .body(Body::AgentOutput(AgentOutputBody {
        text: "The answer is 42.".to_string(),
        model: "claude-3-7-sonnet".to_string(),
        prompt_tokens: 512,
        completion_tokens: 8,
        finished_normally: true,
    }))
    .build()?;
// id and fingerprint computed automatically
```

### GateVerdict Derived from AgentOutput

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

let verdict = EngramBuilder::new()
    .kind(Kind::GateVerdict)
    .body(Body::GateVerdict(GateVerdictBody {
        passed: false,
        gate_name: "hallucination_check".to_string(),
        confidence: 0.88,
        rationale: "Claims cite non-existent paper DOI 10.xxxx/yyyyyy".to_string(),
        rung: 3,
    }))
    .parent(agent_output.id)       // link to parent
    .provenance(Provenance::from_agent("gate-agent-v1"))
    .build()?;
```

### KnowledgeEntry with Custom Decay

```rust
<!-- source: crates/roko-core/src/engram_builder.rs -->

let knowledge = EngramBuilder::new()
    .kind(Kind::KnowledgeEntry)
    .body(Body::KnowledgeEntry(KnowledgeEntryBody {
        text: "Rust async functions return Futures; await suspends execution.".to_string(),
        structured: None,
        domain_tags: vec!["rust".to_string(), "async".to_string()],
        validation_tier: 2,
    }))
    .decay(Decay::Demurrage(DemurrageParams {
        balance: 1.0,
        idle_tax_per_day: 0.01,    // 1% per day if not used
        reinforcement_per_use: 0.05,
    }))
    .tag("session_id", "sess-abc123")
    .build()?;
```

---

## Invariants

1. `EngramBuilder::build()` is the only valid way to construct an Engram with a correct id
2. `body` variant must match `kind`
3. `skip_fingerprint()` is test-only; production code must not call it

---

## Failure Modes

| Failure | Cause | `EngramBuildError` variant |
|---------|-------|----------------------------|
| Missing kind | `build()` called without `.kind()` | `MissingKind` |
| Missing body | `build()` called without `.body()` | `MissingBody` |
| Body-Kind mismatch | Body variant inconsistent with Kind | `BodyKindMismatch` |
| Bad timestamp | `created_at_ms <= 0` | `InvalidTimestamp` |
| Duplicate lineage | Same ContentHash appears twice in lineage | `DuplicateLineage` |

---

## See Also

- [`01-struct-reference.md`](01-struct-reference.md) — the Engram struct
- [`12-invariants.md`](12-invariants.md) — invariants enforced here
- [`13-examples.md`](13-examples.md) — more complete examples
