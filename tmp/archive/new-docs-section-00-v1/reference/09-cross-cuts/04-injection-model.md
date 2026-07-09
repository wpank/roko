# Cross-Cut Injection Model

> How cross-cuts attach to the loop; their lifecycle from construction to shutdown.

**Status**: Shipping
**Crate**: `roko-orchestrator` (L3) + `roko-agent` (L2)
**Last reviewed**: 2026-04-19

---

## TL;DR

Cross-cuts are injected into `TickContext` as trait objects by the L3
`TickContextBuilder`. Each cross-cut exposes one or more L1 traits. The loop calls
those traits at the appropriate stages. The cross-cut's lifecycle is tied to the
agent's lifecycle.

---

## Injection Points

Each cross-cut injects into `TickContext` at specific fields:

| Cross-cut | TickContext fields it populates |
|---|---|
| Neuro | `scorer_ctx.utility_tracker`, `substrate` (wrapped with HDC index) |
| Daimon | `scorer_ctx.affective_state`, `router_ctx.urgency_signal`, `composer_ctx.behavioral_state` |
| Dreams | Not in real-time `TickContext`; injected into `DeltaContext` |

### Neuro Injection

```rust
// In TickContextBuilder
.with_cross_cut(Neuro::new(&config.neuro))
// → wraps the substrate with a NeuroSubstrate adapter that:
//   1. intercepts substrate.put() to update the HDC index
//   2. intercepts substrate.query() to use HDC search
// → injects utility_tracker into ScorerContext
```

### Daimon Injection

```rust
.with_cross_cut(Daimon::new(&config.daimon))
// → polls Daimon.current_pad() and injects into:
//   - ScorerContext.affective_state (for Valence axis)
//   - RouterContext.urgency_signal (for threshold adjustment)
//   - ComposerContext.behavioral_state (for system prompt note)
```

### Dreams Injection

```rust
.with_cross_cut(Dreams::new(&config.dreams))
// → registered with the DeltaScheduler, not TickContext
// → DeltaContext.dreams is set when a Delta pass runs
```

---

## Lifecycle

```
Agent startup:
  TickContextBuilder.build() calls cross_cut.initialize()
  Cross-cuts load their state from the Substrate

Per real-time tick:
  TickContextBuilder fills sub-contexts from cross-cut current state
  loop_tick() calls cross-cut trait methods at each stage
  After PERSIST, cross-cuts update their internal state from tick result

Delta consolidation:
  DeltaContext.dreams.run_replay() and run_imagination()
  Dreams updates its internal state from replay results

Agent shutdown:
  cross_cut.flush() persists any buffered state
  cross_cut.shutdown() releases resources
```

Cross-cuts must not retain state that cannot be reconstructed from the Substrate.
If a cross-cut crashes and is reloaded, it rebuilds from substrate data.

---

## Thread Safety

All cross-cut trait objects are `Send + Sync`. Cross-cuts that maintain mutable
state use interior mutability (`Mutex`, `RwLock`, or atomic operations). The HDC
index in Neuro uses an `RwLock<HdcIndex>` — reads are concurrent, writes are exclusive.

---

## Cross-Cut vs. Stage Trait

| Property | Stage (e.g., Scorer) | Cross-cut (e.g., Neuro) |
|---|---|---|
| Injected into | `TickContext.scorer` | Multiple TickContext sub-fields |
| Has own struct? | No (just a trait impl) | Yes (owns state, has lifecycle) |
| Persists state across ticks? | No | Yes (via Substrate or in-memory) |
| Participates in Delta? | Scorer: No | Neuro, Dreams: Yes |

---

## See also

- [L3 Harness](../08-layers/04-L3-harness.md) — where `TickContextBuilder` lives
- [Composition](05-composition.md) — using multiple cross-cuts together
- [Boundaries](06-boundaries.md) — what cross-cuts are allowed to do
- [Neuro](01-neuro.md), [Daimon](02-daimon.md), [Dreams](03-dreams.md) — specific injections
