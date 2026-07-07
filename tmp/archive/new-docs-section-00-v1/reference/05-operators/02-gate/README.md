# Gate

> `Gate` makes a pass/fail/abstain verdict on an `Engram` and its `Score`. It is the filter
> step — the operator that stops low-quality, unsafe, or irrelevant information from
> proceeding through the cognitive loop.

**Status**: Shipping
**Crate**: `roko-gate`
**Depends on**: [Scorer](../01-scorer/README.md), [Score](../../10-types/score.md)
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | Status |
|---|---|---|
| 00 | [Overview](./00-overview.md) | Shipping |
| 01 | [Trait Surface](./01-trait-surface.md) | Shipping |
| 02 | [Semantics](./02-semantics.md) — Pass/Fail/Abstain, not Error | Shipping |
| 03 | [Implementation](./03-implementation.md) | Shipping |
| 04 | [API Reference](./04-api-reference.md) | Shipping |
| 05 | [Invariants](./05-invariants.md) | Shipping |
| 06 | [Failure Modes](./06-failure-modes.md) — gate failure ≠ verdict | Shipping |
| 07 | [Performance](./07-performance.md) | Shipping |
| 08 | [Examples](./08-examples.md) | Shipping |
| 09 | [Gate Composition](./09-gate-composition.md) — 11-gate/7-rung pipelines | Shipping |
| 10 | [Rationale](./10-rationale.md) | Shipping |

## See Also

- [Scorer](../01-scorer/README.md) — provides the Score that Gate evaluates
- [Router](../03-router/README.md) — receives the engram after Gate passes it
