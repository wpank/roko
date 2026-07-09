# Score — 7-Axis Quality Assessment

> Score quantifies the quality of an Engram across 7 axes. This folder is the canonical reference.

**Status**: Shipping  
**Crate**: `roko-core`  
**Last reviewed**: 2026-04-19

---

## What Is Score?

A `Score` is a struct with up to 7 floating-point axes, each in [0.0, 1.0], that together
describe the quality of an Engram. The score is not part of the Engram's identity hash —
it can be recomputed at any time by a Scorer without changing the Engram's id.

The 4 stable axes (`confidence`, `novelty`, `utility`, `reputation`) are always present.
The 3 extended axes (`precision`, `salience`, `coherence`) are optional and typically
computed by specialized Scorers.

---

## Contents

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| [00](00-overview.md) | Overview | What Score is; the 7 axes; effective formula | Shipping |
| [01](01-axes-stable.md) | Stable axes | confidence, novelty, utility, reputation | Shipping |
| [02](02-axes-extended.md) | Extended axes | precision, salience, coherence | Shipping |
| [03](03-arithmetic.md) | Arithmetic | How axes combine; effective score formula | Shipping |
| [04](04-constants.md) | Constants | All named constants with values and rationale | Shipping |
| [05](05-api-reference.md) | API reference | Score struct, methods, accessors | Shipping |
| [06](06-examples.md) | Examples | Scoring an Engram through each layer | Shipping |
| [07](07-invariants.md) | Invariants | Axis bounds, monotonicity | Shipping |
| [08](08-rationale.md) | Rationale | Why 7 axes; what was dropped | Shipping |

---

## Suggested Reading Order

**First time:** 00 → 01 → 03  
**Implementing a Scorer:** 01 → 02 → 03 → 04 → 05  
**Debugging unexpected scores:** 06 → 07  

---

## See Also

- [`reference/01-engram/08-scoring-fields.md`](../../01-engram/08-scoring-fields.md) — how Score attaches to Engram
- [`reference/05-operators/scorer/`](../../05-operators/scorer/README.md) — Scorer trait (Cluster B)
