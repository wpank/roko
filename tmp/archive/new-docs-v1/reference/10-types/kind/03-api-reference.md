# Kind — API Reference and Invariants

> Methods, canonical encoding, and invariants for the Kind enum.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## Methods

```rust
<!-- source: crates/roko-core/src/kind.rs -->

impl Kind {
    /// Return the stable canonical bytes for this variant.
    /// Used in ContentHash computation.
    pub fn as_bytes(&self) -> &[u8];

    /// Return the default Decay for an Engram of this Kind.
    pub fn default_decay(&self) -> Decay;

    /// Return a human-readable name for this Kind.
    pub fn display_name(&self) -> &str;

    /// Return true if this Kind typically produces a non-None fingerprint.
    /// Binary bodies (e.g., raw Observations) may not.
    pub fn is_fingerprintable(&self) -> bool;
}
```

---

## Canonical Bytes Table

| Variant | `as_bytes()` return value |
|---|---|
| `AgentOutput` | `b"agent_output"` |
| `GateVerdict` | `b"gate_verdict"` |
| `ToolTrace` | `b"tool_trace"` |
| `KnowledgeEntry` | `b"knowledge_entry"` |
| `Prediction` | `b"prediction"` |
| `Observation` | `b"observation"` |
| `Plan` | `b"plan"` |
| `Episode` | `b"episode"` |
| `Reflection` | `b"reflection"` |
| `Pheromone` | `b"pheromone"` |
| `Metric` | `b"metric"` |
| `ContextAssembly` | `b"context_assembly"` |
| `ModelSelection` | `b"model_selection"` |
| `ErrorRecord` | `b"error_record"` |
| `Custom(s)` | `s.as_bytes()` |

---

## Invariants

1. `as_bytes()` returns the same bytes for the same variant on every call — it is stable
   and deterministic.
2. No two built-in variants share the same `as_bytes()` value.
3. `Custom(s1)` and `Custom(s2)` have the same bytes iff `s1 == s2`.
4. `Custom(s)` where `s` equals any built-in's bytes creates a hash collision — callers
   must namespace custom names (e.g., `"myapp/my_kind"` rather than `"plan"`).
5. `default_decay()` returns a `Decay` variant consistent with the Kind's tier (see
   [tier matrix](../decay/08-tier-matrix.md)).
6. Kind is included in `ContentHash` — different Kinds for the same body produce different hashes.

---

## Error Types

There are no error types for `Kind` construction — all variants are valid by construction.
The only potential issue is `Custom(String)` with an empty string, which is valid but
semantically meaningless. A lint warning is emitted if empty.

---

## Open Questions

- Should `Custom(String)` be rejected if the string matches a built-in variant's bytes?

## See Also

- [`00-overview.md`](00-overview.md) — Kind overview
- [`01-variant-reference.md`](01-variant-reference.md) — variant descriptions
- [`../content-hash/01-canonical-encoding.md`](../content-hash/01-canonical-encoding.md) — how Kind enters the hash
