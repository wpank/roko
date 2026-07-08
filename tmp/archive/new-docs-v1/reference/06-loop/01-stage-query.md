# QUERY — Stage 1 of the Cognitive Loop

> Retrieve candidate Engrams from the Substrate in response to the current stimulus.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Substrate trait](../03-substrate/README.md),
[Engram](../01-engram/README.md), [Pulse](../02-pulse/README.md)
**Used by**: [SCORE](02-stage-score.md), [loop\_tick()](09-loop-tick-code.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

QUERY is the first stage of every tick. It takes the incoming stimulus (a `Pulse` or a
structured query derived from it), asks the `Substrate` for matching `Engram` records,
and returns a raw candidate set. QUERY does not rank, filter by policy, or make routing
decisions — those are SCORE and ROUTE's jobs. QUERY's only job is retrieval.

---

## The Idea

Before an agent can think about what to do next, it must surface what it already knows.
The QUERY stage is that surfacing operation. It is analogous to the fast, automatic
recall that happens in human cognition before conscious evaluation: a stimulus triggers
an associative sweep through long-term memory.

In Roko, "memory" is the `Substrate` — a typed key-value store with hybrid HDC/vector
search. The QUERY stage calls `Substrate::query()` with a `QuerySpec` derived from the
incoming `Pulse`. The spec encodes:

- **Semantic similarity** — HDC distance to the stimulus embedding
- **Temporal window** — how far back to look (controlled by the speed tier)
- **Kind filter** — e.g., only retrieve `Engram`s of kind `Observation` or `Plan`
- **Provenance filter** — only retrieve from trusted sources (see
  [Provenance](../10-types/provenance.md))
- **Budget cap** — maximum number of candidates to surface (enforced by Substrate)

The result is a `Vec<Engram>` — unranked, unfiltered, but already deduplicated
(the Substrate handles deduplication by content hash).

---

## Specification

```rust
// source: crates/roko-agent/src/loop/query.rs
pub struct QuerySpec {
    pub stimulus:    HdcFingerprint,
    pub time_window: Duration,
    pub kind_filter: Option<EngramKindSet>,
    pub prov_filter: Option<ProvenanceFilter>,
    pub max_results: usize,
}

pub trait QueryStage {
    fn query(
        &self,
        substrate: &dyn Substrate,
        spec: &QuerySpec,
    ) -> Result<Vec<Engram>, QueryError>;
}
```

The default implementation shipped in `roko-agent` calls `substrate.query(spec)` and
applies no further transformation. Custom implementations may pre-filter, expand
(query-by-association), or re-weight the spec before calling the substrate.

---

## Semantics

1. The stage receives the `Pulse` that triggered this tick.
2. It extracts the HDC fingerprint from the Pulse's payload.
3. It constructs a `QuerySpec` from the fingerprint plus tick-level parameters
   (time window from the speed tier, budget cap from the Harness config).
4. It calls `substrate.query(spec)`.
5. It returns `Vec<Engram>` to SCORE. An empty vec is valid — the tick continues.

The `QuerySpec.time_window` is set by the speed tier:
- Gamma ticks look back at most 60 s.
- Theta ticks look back at most 10 min.
- Delta ticks look back at most 24 h.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| `QueryError::Timeout` | Substrate did not respond within the stage budget | Return empty vec; log warning; continue tick |
| `QueryError::Unavailable` | Substrate is down or saturated | Abort tick; publish `substrate.unavailable` Pulse |
| `QueryError::Malformed` | `QuerySpec` invalid (e.g., zero max_results) | Panic in debug; return `Err` in release; tick fails |
| Empty result set | No matching Engrams in window | Normal; tick proceeds with empty context |

A timeout in QUERY does **not** abort the tick — the agent proceeds with an empty
candidate set. This is intentional: agents must be able to act on a new stimulus even
when prior knowledge is unavailable. The `query.timeout` Pulse is published so that
the monitoring layer can track substrate health.

---

## Performance

| Metric | Target | P99 budget |
|---|---|---|
| Wall time | < 8 ms | < 20 ms |
| Substrate round-trips | 1 | 1 |
| Candidates returned | ≤ 64 | — |
| Memory allocations | O(N) where N = candidates | — |

The candidate cap (default 64) is the primary lever for controlling QUERY latency.
For Gamma ticks the cap is typically 16; for Delta ticks it may be 256 or higher since
latency constraints are looser.

HDC similarity search at 100 K entries takes ≈ 170 µs with SIMD. With a 64-candidate
cap, substrate time is dominated by HDC, not I/O for in-process substrates.

---

## Examples

### 1. Simple reactive query

A user sends: `"What is the current price of KORAI?"`. The incoming Pulse has kind
`UserMessage`. QUERY surfaces up to 16 recent `Observation` Engrams that match the HDC
fingerprint of "KORAI price". It returns them unranked; SCORE will rank them next.

### 2. No prior knowledge

An agent receives a stimulus about a completely novel topic. QUERY returns an empty
vec. The tick continues; COMPOSE will assemble a context from only the stimulus itself.
This is correct behavior — the agent reasons from first principles.

### 3. Provenance-filtered query

A research agent is configured to only trust `Observation` Engrams attested by
verified external sources. `QuerySpec.prov_filter` excludes self-generated Engrams.
The returned candidates are all externally sourced facts.

---

## See also

- [SCORE](02-stage-score.md) — ranks the candidates returned here
- [Substrate trait](../03-substrate/README.md) — the backing store
- [Engram](../01-engram/README.md) — the data type returned
- [HDC Fingerprint](../10-types/hdc-fingerprint.md) — how similarity is computed
- [Performance](14-performance.md) — aggregate budget breakdown across all stages
