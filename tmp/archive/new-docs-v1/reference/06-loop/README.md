# The Universal Cognitive Loop

> The single processing cycle that drives every Roko agent: eight stages, one tick,
> deterministic composition.

**Status**: Shipping
**Depends on**: [Engram](../01-engram/README.md), [Score](../10-types/score.md),
[Substrate](../03-substrate/README.md), [Bus](../04-bus/README.md),
[Operators](../05-operators/README.md)

---

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](00-overview.md) | The full loop at a glance; why eight stages | Shipping |
| 01 | [QUERY](01-stage-query.md) | Retrieve candidates from Substrate | Shipping |
| 02 | [SCORE](02-stage-score.md) | Appraise each candidate on 7 axes | Shipping |
| 03 | [ROUTE](03-stage-route.md) | Select the winning sub-agent or tool | Shipping |
| 04 | [COMPOSE](04-stage-compose.md) | Assemble the context window | Shipping |
| 05 | [ACT](05-stage-act.md) | Execute — call model / tool / sub-agent | Shipping |
| 06 | [VERIFY](06-stage-verify.md) | Gate the result against policy | Shipping |
| 07 | [PERSIST](07-stage-persist.md) | Write outcome Engrams back to Substrate | Shipping |
| 08 | [REACT](08-stage-react.md) | Publish Pulses; trigger next tick | Shipping |
| 09 | [loop\_tick() reference](09-loop-tick-code.md) | Canonical Rust implementation | Shipping |
| 10 | [Dual-Process](10-dual-process.md) | T0/T1/T2 tiers; System 1 vs System 2 thinking | Shipping |
| 11 | [Active Inference](11-active-inference.md) | predict/publish/correct; prediction.error Pulses | Shipping |
| 12 | [Invariants](12-invariants.md) | What must be true after every tick | Shipping |
| 13 | [Failure Modes](13-failure-modes.md) | Stuck detection, timeouts, partial failure | Shipping |
| 14 | [Performance](14-performance.md) | Per-stage latency budgets, tail-latency control | Shipping |
| 15 | [Examples](15-examples.md) | End-to-end worked scenarios | Shipping |
| 16 | [Open Questions](16-open-questions.md) | Unresolved design decisions | — |

---

## Suggested reading order

**First-time reader**: 00 → 01–08 (skim each stage) → 09 → 10 → 12

**Implementer building a new operator**: 00 → the specific stage you're modifying → 09 → 12 → 13

**Performance engineer**: 14 → 09 → 12 → 13

**Debugging a stuck agent**: 13 → 09 → the stage where it is stuck

---

## See also

- [Three Cognitive Speeds](../07-speeds/README.md) — which speed tier drives each tick
- [Five-Layer Taxonomy](../08-layers/README.md) — where `loop_tick()` lives in the stack
- [Cross-Cuts](../09-cross-cuts/README.md) — how Neuro, Daimon, and Dreams inject into the loop
- [Operators](../05-operators/README.md) — the trait objects that back each stage
