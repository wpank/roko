# Policy

> `Policy` is the reactive control operator. It monitors agent behaviour and intervenes
> when necessary: tripping circuit breakers, escalating to humans, enforcing safety
> constraints, and routing prediction errors to learning.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Score](../../10-types/score.md), [Bus](../../04-bus/README.md)
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | Status |
|---|---|---|
| 00 | [Overview](./00-overview.md) | Shipping |
| 01 | [Trait Surface](./01-trait-surface.md) | Shipping |
| 02 | [Semantics](./02-semantics.md) — reactive: circuit breakers, escalation, safety | Shipping |
| 03 | [Implementation](./03-implementation.md) | Shipping |
| 04 | [API Reference](./04-api-reference.md) | Shipping |
| 05 | [Invariants](./05-invariants.md) | Shipping |
| 06 | [Failure Modes](./06-failure-modes.md) | Shipping |
| 07 | [Performance](./07-performance.md) | Shipping |
| 08 | [Examples](./08-examples.md) | Shipping |
| 09 | [Policy vs. Calibrator](./09-policy-vs-calibrator.md) — target-state split | Shipping |
| 10 | [Rationale](./10-rationale.md) | Shipping |
