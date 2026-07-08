# Scorer

> `Scorer` assigns a 7-axis `Score` to an `Engram`. It is the appraisal step of the
> cognitive loop — the operator that decides how valuable, confident, novel, and useful a
> piece of memory is.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Engram](../../01-engram/README.md), [Score](../../10-types/score.md)
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](./00-overview.md) | What a Scorer does, the scoring model | Shipping |
| 01 | [Trait Surface](./01-trait-surface.md) | Rust trait signature | Shipping |
| 02 | [Semantics](./02-semantics.md) | What each score axis means; accumulation | Shipping |
| 03 | [Implementation](./03-implementation.md) | Shipping Scorer implementations | Shipping |
| 04 | [API Reference](./04-api-reference.md) | Quick-reference | Shipping |
| 05 | [Invariants](./05-invariants.md) | What must always be true | Shipping |
| 06 | [Failure Modes](./06-failure-modes.md) | Scorer panics, NaN, score overflow | Shipping |
| 07 | [Performance](./07-performance.md) | Scoring cost per tick | Shipping |
| 08 | [Examples](./08-examples.md) | Usage patterns | Shipping |
| 09 | [Composition Patterns](./09-composition-patterns.md) | Stacking Scorers; chained scoring | Shipping |
| 10 | [Rationale](./10-rationale.md) | Why a trait; axis choices | Shipping |

## See Also

- [Gate](../02-gate/README.md) — consumes the Score to make a pass/fail verdict
- [Score Type](../../10-types/score.md) — the 7-axis Score struct
