# Engram — Rationale and History

> Why the Engram is designed the way it is; what was rejected; the retirement of the `Signal` name.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

The Engram evolved from a simple `Signal` struct used as an event envelope into a
universal datum with content addressing, scoring, decay, provenance, and HDC fingerprinting.
The key design decisions: one type (not many), content addressing (not id assignment), 
score-excluded-from-hash (not frozen quality), and the Demurrage model (not simple exponential).
The name changed from `Signal` to `Engram` to better reflect the neuroscience analogy
and to distinguish it from `Pulse` (ephemeral events).

---

## The Naming History

### `Signal` (current code identifier — retired conceptual term)

The original type was named `Signal`, following the analogy of neural signals. The name
was accurate for the original use case (an event envelope flowing through a pipeline) but
became misleading as the type grew:

1. "Signal" implies transience — signals happen and are gone. But Engrams are durable.
2. "Signal" implies a narrow purpose. Engrams carry knowledge, verdicts, traces, plans.
3. The planned `Pulse` type (ephemeral events) would be a much better "signal" analogy.

The canonical term is now **Engram**. The Rust code still uses `Signal` pending a rename
refactor. Callers should treat `Signal` and `Engram` as identical.

The retirement of `Signal` is tracked in
[`_migration/cluster-A-engram.md`](../../_migration/cluster-A-engram.md).

### `Engram` (canonical architectural term — current)

"Engram" comes from neuroscience: the hypothetical physical trace of a memory in the brain
(Richard Semon, *Die Mneme*, 1904; Karl Lashley, 1950 engram search experiments; Tonegawa
et al., *Science* 2015, first direct observation of memory engrams). 

In Roko, the analogy is precise:
- A memory engram is content-addressed by the pattern of neural activation.
- A Roko Engram is content-addressed by BLAKE3.
- Memory engrams decay unless reinforced (synaptic potentiation).
- Roko Engrams decay unless retrieved (Demurrage reinforcement).
- Memory engrams have a lineage — one memory triggers recall of related memories.
- Roko Engrams have a lineage DAG.

---

## Design Decision: One Type, Not Many

**Considered**: separate types for `Task`, `Event`, `Message`, `Record`, `Log`.

**Rejected** because:

1. Every new capability would require a new type and storage schema. The history of
   agent frameworks shows this leads to N incompatible storage backends for N types.

2. Composition becomes impossible. If a Scorer only works on `Record` but an operator
   produces `Task`, you cannot score tasks.

3. Audit trails fragment. "What happened?" requires correlating across N type-specific
   stores.

**Chosen**: one type with a `Kind` discriminant. The Kind field carries the semantic
differentiation; the struct carries the lifecycle.

The cost: any operator that expects a specific Kind must check at runtime (or at
operator configuration time) rather than at compile time. The tradeoff is considered
worthwhile given the composability gains.

---

## Design Decision: Content Addressing, Not Id Assignment

**Considered**: monotonically increasing integer ids (like most databases); UUID4 random ids.

**Rejected** because:

1. Integer ids require coordination — either a central counter or distributed id agreement.
   Roko agents run in distributed, occasionally-offline environments. Coordination is expensive.

2. UUID4 ids are not deterministic — two agents independently observing the same fact would
   produce different ids, preventing deduplication.

**Chosen**: BLAKE3 content hash. Two agents observing the same fact produce the same id.
No coordination required. Deduplication is automatic.

The cost: if two Engrams with different content accidentally hash to the same value
(collision), the system would treat them as identical. BLAKE3 collision probability is ≈
2⁻¹²⁸ per pair. This is treated as impossible in practice.

---

## Design Decision: Score Excluded From Identity Hash

**Considered**: including `score` in the identity hash.

**Rejected** because:

1. A Scorer evaluating an Engram it just produced would produce a circular dependency
   (the score depends on the id, the id depends on the score).

2. Outcome-driven scoring (updating `utility` after a gate verdict) would require creating
   a new Engram — the old one would be orphaned in the lineage DAG.

3. Reputation updates (from attestation) would similarly require creating new Engrams.

**Chosen**: score is excluded from the hash. Scorers can recompute freely. Outcome-driven
and reputation-driven score updates modify the score in-place without changing identity.

---

## Design Decision: Demurrage as Primary Decay Model

**Considered**: simple exponential decay as the only model.

**Rejected** because:

1. Exponential decay ignores retrieval. A KnowledgeEntry that is retrieved 100 times per
   day should not decay at the same rate as one never retrieved.

2. "Use it or lose it" is the natural behavior for knowledge: frequently-needed information
   stays available; rarely-needed information fades.

**Chosen**: Demurrage as the primary model (idle tax + reinforcement on retrieval),
with Exponential, Step, Linear, and Custom as alternatives for specific use cases.

The Demurrage model is inspired by the economic concept of demurrage: a holding cost on
currency to discourage hoarding. In Roko, "hoarding" knowledge (having it in the substrate
but never retrieving it) incurs a cost.

---

## Design Decision: Optional HDC Fingerprint (Not Mandatory)

**Considered**: mandatory fingerprint on every Engram.

**Rejected** because:

1. Tests and benchmarks that do not exercise similarity search should not pay the encoding
   cost.

2. Substrates without HDC capability (e.g., a simple in-memory store for unit tests)
   should not require the HDC encoder to be available.

**Chosen**: `fingerprint: Option<HdcFingerprint>`. Production substrates always compute
the fingerprint. Test substrates can skip it with `EngramBuilder::skip_fingerprint()`.

---

## What Was Rejected in Full

| Candidate | Rejected reason |
|-----------|----------------|
| `Task` type | Too narrow; agents produce non-task information constantly |
| `Record` type | Too generic; loses the "scored, decaying, provenance-stamped" lifecycle |
| `Signal` as the canonical name | Implies transience; conflicts with planned `Pulse` type |
| Integer ids | Require coordination; non-deterministic under concurrency |
| UUID4 ids | Non-deterministic; no deduplication |
| Score in identity hash | Circular dependency; prevents score mutation |
| Mandatory fingerprint | Too expensive for tests; requires encoder availability |
| Simple exponential as the only decay | Does not reward retrieval; unfit for knowledge |

---

## History Timeline

| Date | Change |
|------|--------|
| Early Roko (pre-2025) | `Signal` struct: event envelope, no scoring, no decay |
| 2025 Q1 | Score added to Signal (4 axes: confidence, novelty, utility, reputation) |
| 2025 Q2 | Decay added; Exponential model only |
| 2025 Q3 | HDC fingerprint added; BLAKE3 content addressing |
| 2025 Q4 | Provenance (author, trust, taint); Demurrage decay model |
| 2026 Q1 | Lineage DAG formalized; 3 extended score axes added |
| 2026 Q2 (current) | `Signal` → `Engram` rename in architecture docs; code rename pending |

---

## Open Questions

- When will the code rename from `Signal` to `Engram` occur? (Tracked as a refactor task)
- Should `body` sub-structs be moved to `roko-types` crate for cleaner dependencies?
- Should Body have a `Raw(Vec<u8>)` catch-all variant for forward compatibility?

---

## References

- Semon, R. (1904). *Die Mneme*. Engelmann, Leipzig.
- Lashley, K. S. (1950). In search of the engram. *Symposia of the Society for Experimental Biology*, 4, 454–482.
- Tonegawa, S., Liu, X., Ramirez, S., & Redondo, R. (2015). Memory engram cells have come of age. *Neuron*, 87(5), 918–931.
- Kanerva, P. (2009). Hyperdimensional computing: An introduction to computing in distributed representation with high-dimensional random vectors. *Cognitive Computation*, 1(2), 139–159.

---

## See Also

- [`00-overview.md`](00-overview.md) — the Engram today
- [`reference/02-pulse/00-overview.md`](../02-pulse/00-overview.md) — the `Pulse` type that replaced `Signal` for ephemeral events
- [`reference/10-types/decay/01-demurrage.md`](../10-types/decay/01-demurrage.md) — Demurrage rationale
