# The Universal Cognitive Loop — Overview

> Every Roko agent runs one processing cycle, one tick at a time.
> That cycle has exactly eight stages. No agent skips a stage; no agent invents new ones.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Score](../10-types/score.md), [Engram](../01-engram/README.md),
[Substrate](../03-substrate/README.md), [Operators](../05-operators/README.md)
**Used by**: [Three Cognitive Speeds](../07-speeds/README.md),
[Cross-Cuts](../09-cross-cuts/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko agents think in a fixed eight-stage cycle called the **universal cognitive loop**.
The stages are: **QUERY → SCORE → ROUTE → COMPOSE → ACT → VERIFY → PERSIST → REACT**.
Every tick is an atomic unit of cognition; every tick produces at least one persistent
outcome (a new or updated Engram). The loop is the unit of testability, the unit of
tracing, and the unit of budget accounting.

---

## The Idea

Cognitive agent frameworks often grow organically: one callback here, another hook
there, a dozen interleaved async tasks. The result is hard to trace, hard to test, and
hard to reason about under failure. Roko takes the opposite approach: **one loop,
always the same shape.**

The eight stages map directly onto what any purposeful intelligent system must do:

1. **QUERY** — retrieve relevant prior knowledge
2. **SCORE** — rank that knowledge by relevance, recency, trust, and more
3. **ROUTE** — decide which capability should handle the current task
4. **COMPOSE** — assemble the context for that capability
5. **ACT** — execute the capability and collect its output
6. **VERIFY** — check whether the output meets policy requirements
7. **PERSIST** — write the outcome back to long-term memory
8. **REACT** — propagate consequences into the environment and schedule the next tick

This decomposition is not arbitrary. It mirrors the perception–action loop from
cognitive science, the predict–act–update cycle from active inference, and the
read–evaluate–print loop from computer science — but grounds all three in a
concrete, typed, auditable Rust implementation.

---

## The Eight Stages at a Glance

```
Incoming stimulus (Pulse or external event)
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  TICK n                                                         │
│                                                                 │
│  QUERY ──► SCORE ──► ROUTE ──► COMPOSE ──► ACT                 │
│                                              │                  │
│  REACT ◄── PERSIST ◄── VERIFY ◄─────────────┘                  │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
Outgoing Pulses → Bus → other agents / next tick
```

Each arrow is a typed handoff — the output of QUERY is `Vec<ScoredEngram>`, the output
of SCORE is a ranked slice of that, and so on. The types are defined in
[`09-loop-tick-code.md`](09-loop-tick-code.md).

---

## Why This Shape?

### Separation of concerns

Each stage has exactly one responsibility. SCORE never touches the substrate directly;
QUERY never evaluates output quality. This means bugs are localized, operators are
substitutable, and the loop can be instrumented at any seam.

### Deterministic ordering

The stages always run in the same order. There is no conditional branching that skips
VERIFY, no fast path that elides PERSIST. When a stage is not needed (e.g., the task
requires no substrate retrieval), QUERY returns an empty result set — it still runs.
This guarantees that every tick produces a complete trace.

### Composable extension

The loop is parameterized by trait objects. Swapping the Scorer changes how candidates
are ranked. Swapping the Router changes which sub-agent is selected. Swapping the Gate
inside VERIFY changes what "acceptable output" means. None of these substitutions
require changes to the loop itself.

### Budget accounting

Every stage reports its wall-clock time and allocated cost. The budget controller
(part of the [Harness layer](../08-layers/03-L3-harness.md)) aggregates stage costs
into a per-tick budget. When the tick budget is exceeded, the loop aborts at the stage
boundary and publishes a `budget.exceeded` Pulse.

---

## Relationship to Cognitive Speeds

The loop runs at three different tempos depending on urgency:

| Tier | Speed | Typical period | Loop shape |
|---|---|---|---|
| T0 | Gamma (reactive) | 5–15 s | Full loop, minimal COMPOSE |
| T1 | Theta (reflective) | ~75 s | Full loop, deep QUERY + rich COMPOSE |
| T2 | Delta (consolidation) | hours | QUERY + SCORE + PERSIST only (no ACT) |

See [Three Cognitive Speeds](../07-speeds/README.md) for full detail.

---

## Relationship to Active Inference

Before QUERY runs, the active inference layer publishes a **prediction Pulse** — a
forward model of what the tick will find and decide. After PERSIST, it compares the
prediction against the actual outcome and publishes a **prediction.error Pulse**. These
two Pulses are the learning signal for online adaptation.

See [Active Inference](11-active-inference.md) for the full mechanism.

---

## Relationship to the Five-Layer Stack

`loop_tick()` is implemented in `roko-agent` (Layer 2, Scaffold). It depends on
trait objects that are injected from Layer 1 (Framework) and populated by Layer 3
(Harness). The loop never calls Layer 4 (Orchestration) directly — that direction of
dependency is forbidden.

See [Five-Layer Taxonomy](../08-layers/README.md) and
[loop\_tick() reference](09-loop-tick-code.md).

---

## Invariants

The loop maintains several invariants per tick. The full list is in
[Invariants](12-invariants.md). The two most important:

1. **Every tick produces at least one Engram write.** A tick that produces no output
   is a bug; the PERSIST stage will write a `tick.null` Engram if nothing else was
   created.
2. **VERIFY always runs before PERSIST.** A result that fails verification is never
   persisted, even partially. The failed attempt is itself persisted as a
   `verify.failure` Engram.

---

## See also

- [loop\_tick() reference](09-loop-tick-code.md) — the canonical Rust implementation
- [Dual-Process](10-dual-process.md) — how T0/T1/T2 map onto System 1 / System 2
- [Failure Modes](13-failure-modes.md) — what happens when a stage errors or times out
- [Performance](14-performance.md) — latency budgets per stage
- [Examples](15-examples.md) — worked scenarios from real agent runs
