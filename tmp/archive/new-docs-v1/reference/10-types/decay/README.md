# Decay — Engram Temporal Dynamics

> The Decay type governs how an Engram's weight changes over time. This folder is the canonical reference.

**Status**: Shipping  
**Crate**: `roko-core`  
**Last reviewed**: 2026-04-19

---

## What Is Decay?

Every Engram has a `decay: Decay` field that determines how its effective weight evolves
over time. A weight of 1.0 means "full strength." A weight approaching 0.0 means "nearly
gone." The Substrate multiplies an Engram's score by its decay weight at retrieval time.
Engrams at or near 0.0 are GC candidates.

The primary model is **Demurrage**: an idle tax plus reinforcement on retrieval. The system
rewards knowledge that is used and penalizes knowledge that sits idle.

---

## Contents

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| [00](00-overview.md) | Overview | Why Engrams decay; demurrage framing | Shipping |
| [01](01-demurrage.md) | Demurrage | balance, idle-tax, reinforcement | Shipping |
| [02](02-exponential-decay.md) | Exponential | Classic half-life decay | Shipping |
| [03](03-step-decay.md) | Step decay | Epoch-based drops | Shipping |
| [04](04-linear-decay.md) | Linear decay | Simple time-linear | Shipping |
| [05](05-custom-decay.md) | Custom decay | User-defined functions | Shipping |
| [06](06-reinforcement.md) | Reinforcement | How useful knowledge stays warm | Shipping |
| [07](07-cold-tier-freeze-thaw.md) | Cold tier | Long-lived store behavior | Shipping |
| [08](08-tier-matrix.md) | Tier matrix | The decay-tier cross product | Shipping |
| [09](09-invariants.md) | Invariants | Monotonicity, idempotence, stability | Shipping |
| [10](10-api-reference.md) | API reference | Decay enum/trait, methods | Shipping |
| [11](11-examples.md) | Examples | Worked examples | Shipping |

---

## Suggested Reading Order

**First time:** 00 → 01 → 08  
**Implementing a new decay model:** 05 → 09 → 10  
**Debugging stale Engrams:** 08 → 07 → 09  

---

## See Also

- [`reference/01-engram/09-decay-fields.md`](../../01-engram/09-decay-fields.md) — decay field on Engram
