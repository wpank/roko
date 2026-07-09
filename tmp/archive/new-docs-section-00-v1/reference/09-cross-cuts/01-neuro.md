# Neuro — Knowledge Cross-Cut

> Neuro manages the agent's long-term knowledge: HDC indexing, tier progression,
> and knowledge graph connectivity.

**Status**: Shipping
**Crate**: `roko-neuro` (L2)
**Depends on**: [Engram](../01-engram/README.md), [HDC Fingerprint](../10-types/hdc-fingerprint.md),
[Substrate trait](../03-substrate/README.md)
**Used by**: [QUERY stage](../06-loop/01-stage-query.md), [SCORE stage](../06-loop/02-stage-score.md),
[PERSIST stage](../06-loop/07-stage-persist.md), [Delta consolidation](../07-speeds/03-delta-consolidation.md)
**Last reviewed**: 2026-04-19

> **Full implementation**: [`subsystems/neuro/`](../../subsystems/neuro/README.md)
> (populated in a later refactor cluster). This page covers Neuro's role in the loop.

---

## TL;DR

Neuro is the agent's knowledge manager. It maintains the HDC similarity index that
QUERY searches, updates per-Engram utility estimates that SCORE uses, and orchestrates
tier promotion/demotion during Delta consolidation. Without Neuro, the agent has a flat
substrate with no semantic index and no knowledge lifecycle.

---

## The Idea

An agent that accumulates Engrams indefinitely — with no quality tracking, no semantic
organization, and no lifecycle — will degrade over time. The QUERY stage will surface
an undifferentiated mass of old and new, relevant and irrelevant. Neuro prevents this
by managing knowledge as a tiered, indexed, evolving resource.

Neuro has three sub-concerns:

1. **HDC Index** — a 10 240-bit hyperdimensional index that enables sub-millisecond
   semantic similarity search over thousands of Engrams. The index is maintained
   incrementally: new Engrams are indexed at PERSIST time; stale Engrams are removed
   at Delta consolidation time.

2. **Utility tracking** — Neuro observes which Engrams are retrieved and used
   (via `tick.outcome` Pulses), and maintains an EMA (exponential moving average)
   of utility per Engram. This utility score feeds the SCORE stage's Utility axis.

3. **Tier progression** — based on utility history, Neuro promotes Engrams to longer
   half-lives (they decay more slowly; they surface more reliably) or demotes them.
   This is the knowledge lifecycle: frequently-used knowledge becomes durable; unused
   knowledge fades.

---

## HDC Fingerprint

The HDC index uses 10 240-bit binary sparse codes (BSC vectors). Key properties:

- **Similarity search**: XOR + popcount; ≈ 170 µs for 100 K entries with SIMD.
- **Compositional**: bind two concepts with XOR-bind to create a third that is
  near-similar to both parents. This enables Dreams' imagination phase.
- **Compact**: 10 240 bits = 1 280 bytes per Engram. At 100 K Engrams, the full
  index is ~128 MB — entirely in RAM.
- **No GPU required**: purely CPU SIMD. Runs in WASM, edge devices, and air-gapped
  environments.

See [HDC Fingerprint type](../10-types/hdc-fingerprint.md) for the full specification.

---

## Loop Participation

| Stage | How Neuro participates |
|---|---|
| QUERY | Provides `substrate.query()` augmented with HDC index lookup |
| SCORE | Provides `utility_ema[engram_id]` for the Utility axis |
| PERSIST | Indexes new Engrams into HDC index; updates utility on outcome Engrams |
| Delta QUERY | Full-substrate scan for tier promotion/demotion analysis |
| Delta PERSIST | Writes promoted/demoted Engrams; rebuilds degraded index segments |

---

## Neuro Traits (at L1)

```rust
// source: crates/roko-core/src/traits/neuro.rs
pub trait NeuroIndex: Send + Sync {
    fn index(&mut self, engram: &Engram);
    fn search(&self, query: &HdcFingerprint, k: usize) -> Vec<(EngramId, f32)>;
    fn remove(&mut self, id: &EngramId);
}

pub trait UtilityTracker: Send + Sync {
    fn record_use(&mut self, id: &EngramId, reward: f32);
    fn utility_ema(&self, id: &EngramId) -> f32;
}
```

Both are injected into `TickContext` by `TickContextBuilder`.

---

## Configuration

```toml
[neuro]
hdc_bits              = 10240     # fingerprint width
hdc_density           = 0.01      # fraction of bits set (BSC sparsity)
utility_ema_alpha     = 0.05      # EMA learning rate
promotion_threshold   = 0.70      # promote when utility_ema > this
demotion_threshold    = 0.15      # demote when utility_ema < this
deep_retrieval_theta  = true      # expanded retrieval at Theta speed
```

---

## See also

- [`subsystems/neuro/`](../../subsystems/neuro/README.md) — full Neuro implementation
- [HDC Fingerprint](../10-types/hdc-fingerprint.md) — the fingerprint type
- [Delta consolidation](../07-speeds/03-delta-consolidation.md) — where Neuro runs its lifecycle
- [Injection Model](04-injection-model.md) — how Neuro is injected into TickContext
