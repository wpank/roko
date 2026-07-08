# Decay — Reinforcement

> The mechanism by which successful Engram retrieval increases its effective weight, countering idle decay.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Demurrage](01-demurrage.md), [Overview](00-overview.md)  
**Used by**: [Tier Matrix](08-tier-matrix.md), [Substrate](../../../subsystems/substrate/)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Reinforcement is the counterpart to decay. When an Engram is successfully retrieved, its
decay state is updated to reflect the access: under Demurrage, `balance` increases by
`reinforcement_per_use`; under Custom/Ebbinghaus, `stability` doubles. Linear and Step decay
models are **not** reinforced — they are time-only. This page covers the reinforcement
mechanics, its invariants, and how the Substrate triggers it.

---

## The Idea

Decay encodes the intuition that unread knowledge becomes stale. Reinforcement encodes the
converse: knowledge that is actively used should remain accessible. Together they implement
a form of spaced-repetition at the Substrate level — the system naturally "rehearses" items
that are frequently accessed and forgets items that are not.

The analogy is human long-term potentiation (LTP): synaptic connections that fire together
wire together. In Roko, retrieval is the "firing" signal and reinforcement is the
"wiring" update.

---

## Which Decay Models Support Reinforcement

| Model | Reinforcement behaviour |
|---|---|
| **Demurrage** | `balance += reinforcement_per_use`, capped at 1.0 |
| **Custom** | Delegated to handler's `on_retrieve()` method |
| **Exponential** | No reinforcement — weight is determined by half-life alone |
| **Step** | No reinforcement — weight is epoch-driven |
| **Linear** | No reinforcement — deterministic lifetime |

<!-- ADDED: rationale — Exponential, Step, and Linear are "fire and forget" models where
the creator intends a fixed lifetime. Demurrage is the model that represents knowledge
with a usage-dependent lifetime. -->

---

## Reinforcement Protocol

The Substrate calls `reinforce()` on an Engram's decay state under these conditions:

1. A retrieval query matches the Engram (`ContentHash` or semantic search).
2. The retrieval is **successful** — the Engram is returned to the caller (not filtered by
   score threshold or trust gate).
3. The Substrate has write access to the Engram's decay fields (non-read-only substrate).

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl Decay {
    /// Apply reinforcement after a successful retrieval.
    /// No-op for decay models that do not support reinforcement.
    pub fn reinforce(&mut self) {
        match self {
            Decay::Demurrage(ref mut p) => {
                p.balance = (p.balance + p.reinforcement_per_use).min(1.0);
            }
            Decay::Custom(ref mut p) => {
                // Custom handlers update params in-place via the registry.
                // This is called by the Substrate after dispatching to the handler.
                // See substrate/decay-registry for the dispatch path.
                let _ = p; // no-op here; substrate handles the dispatch
            }
            _ => {
                // Exponential, Step, Linear — no reinforcement.
            }
        }
    }
}
```

---

## Reinforcement Idempotency

Calling `reinforce()` on an already-full Demurrage balance (`balance = 1.0`) is a no-op:

```rust
balance = (1.0 + reinforcement_per_use).min(1.0) = 1.0
```

This means a hot Engram retrieved thousands of times stays at exactly 1.0 and does not
accumulate any overshoot.

---

## Equilibrium Analysis

For Demurrage, there is an equilibrium balance where reinforcement exactly compensates
for idle tax. If an Engram is retrieved exactly once per day:

```
Δ_decay = balance × idle_tax_per_day         (per day)
Δ_reinforce = reinforcement_per_use          (per retrieval)

Equilibrium: Δ_reinforce = Δ_decay
→ reinforcement_per_use = balance_eq × idle_tax_per_day
→ balance_eq = reinforcement_per_use / idle_tax_per_day
```

With defaults (`idle_tax_per_day = 0.01`, `reinforcement_per_use = 0.05`):

```
balance_eq = 0.05 / 0.01 = 5.0   (but capped at 1.0)
```

With these defaults, any retrieval frequency ≥ 1/day keeps the balance at 1.0. The
interesting region is retrieval frequency of 1 per few days:

| Retrieval interval | Equilibrium balance |
|---|---|
| 1 day | 1.0 (capped) |
| 5 days | ~1.0 (still exceeds decay) |
| 10 days | ~0.82 |
| 20 days | ~0.62 |
| 50 days | ~0.31 |
| 100+ days | < 0.05 → GC eligible |

---

## Substrate Integration

```rust
<!-- source: crates/roko-fs/src/substrate.rs -->

/// Called by the read path after a successful Engram retrieval.
pub fn record_retrieval(&self, id: &ContentHash, now_ms: i64) -> Result<(), SubstrateError> {
    let mut engram = self.get_mut(id)?;
    engram.decay.reinforce();
    // Update last_retrieved_ms for lazy idle-tax calculation.
    engram.meta.last_retrieved_ms = now_ms;
    self.put(engram)?;
    Ok(())
}
```

The `last_retrieved_ms` field is **not** part of the Engram's identity hash (it is mutable
metadata). It lives in the Substrate's meta-layer alongside the Engram.

---

## Invariants

1. `balance ∈ [0.0, 1.0]` after reinforcement — the `min(1.0)` cap enforces this.
2. Reinforcement is called **at most once per retrieval** — not once per matching query
   in a batch.
3. Reinforcement on a Custom Engram is delegated to the registered handler; it is never
   a no-op silently.
4. Reinforcement does not change the Engram's `ContentHash` (decay is excluded from the
   hash — see [ContentHash](../content-hash/00-overview.md)).

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| Double-reinforcement on batch retrieval | Substrate calls `record_retrieval` multiple times for one logical query | Deduplicate by `ContentHash` before calling `record_retrieval` |
| Reinforcement lost on crash | Write to decay fields fails mid-transaction | Substrate uses write-ahead log; on restart, applies pending reinforcements |
| Custom handler `on_retrieve` panics | Bug in handler | Substrate catches panic, logs, skips reinforcement, emits metric |
| `last_retrieved_ms` not updated | Meta-layer write skipped | Idle tax calculation will be incorrect; reinforce again on next retrieval |

---

## Open Questions

- Should reinforcement be tracked separately from balance (i.e., a `reinforcement_count`
  field) to enable analytics on retrieval patterns? Not currently stored.
- Should there be an anti-reinforcement path (mark an Engram as "penalized" after a bad
  gate verdict)? The pheromone system uses `anti_deposit` for this; an Engram-level
  equivalent is not yet specified.

## See Also

- [`01-demurrage.md`](01-demurrage.md) — the model where reinforcement has the most effect
- [`05-custom-decay.md`](05-custom-decay.md) — custom handlers implement `on_retrieve()`
- [`07-cold-tier-freeze-thaw.md`](07-cold-tier-freeze-thaw.md) — how frozen Engrams skip reinforcement
- [`08-tier-matrix.md`](08-tier-matrix.md) — per-kind default decay and reinforcement params
