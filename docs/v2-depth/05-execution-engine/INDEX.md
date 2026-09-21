# 05-execution-engine — Depth Index

Depth for [05-EXECUTION-ENGINE.md](../../unified/05-EXECUTION-ENGINE.md)

> **Implementation status (2026-09-10):** The Graph engine (`roko-graph`) is the sole execution engine. The cognitive loop as a Hot Graph (described in `cognitive-loop-as-graph.md`) is the architectural target — the 7-Cell loop, Gamma/Theta/Delta nested timescales, and T0 short-circuit as a conditional edge are design targets. The Graph engine currently runs plans as standard Flows; Hot Graph per-agent execution is future work. Resilience patterns (`resilience-and-numerics.md`) describe the algebraic retry model and circuit breaker logic, which are implemented in `roko-conductor`.

---

## Source docs (3)

### Runtime loop

| Source doc | Status |
|---|---|
| `docs/00-architecture/09-universal-cognitive-loop.md` | Covered |

### Error handling and performance

| Source doc | Status |
|---|---|
| `docs/00-architecture/22-error-handling-recovery.md` | Covered |
| `docs/00-architecture/21-performance-numerical-stability.md` | Covered |

---

## Depth docs

| Doc | Covers | Source docs |
|---|---|---|
| [cognitive-loop-as-graph.md](cognitive-loop-as-graph.md) | 7-step loop as a concrete Hot Graph with typed Cells, Workflow/Activity split for resumability, composing nested loops, Byzantine Cell defenses | `09-universal-cognitive-loop.md` |
| [resilience-and-numerics.md](resilience-and-numerics.md) | Resilience algebra (4 error kinds with algebraic retry rules), circuit breakers as React-protocol state machines, graceful degradation ladder, f32/f64 precision decisions, hot-path budget table | `22-error-handling-recovery.md`, `21-performance-numerical-stability.md` |
