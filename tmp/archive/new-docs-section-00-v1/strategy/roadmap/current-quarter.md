# Current Quarter — Q1 Foundation

> What is targeted this quarter. Keep this short and dated. Replace the entire "Active work" section on each review.

**Snapshot date**: 2026-04-19
**Current quarter**: Q1 — Foundation
**Quarter status**: In progress (pre-Phase-A)

---

## Active work

| Track | Status | Notes |
|---|---|---|
| Doc migration (strategy) | **In progress** | Cluster I: strategy/ tree being written now |
| Doc migration (reference, analysis, research, testing) | **In progress** | Clusters A–H, J–K running in parallel |
| Phase A (architecture narrative rewrite) | **Not started** | Begins after cluster migrations complete enough of the reference vocabulary |
| Phase B (kernel addition) | **Not started** | Blocked on Phase A |
| Phase C (subsystem migration) | **Not started** | Blocked on Phase B |
| Naming track | **In progress** | GLOSSARY.md and ALIASES.md being produced in Cluster D |
| Modularity track | **Not started** | |
| Observability baseline | **Not started** | |

---

## Quarter gate question

> Is the kernel cutover still safer than incremental patching?

This checkpoint occurs at the end of Q1 month 1, once Phase B is drafted and its scope is clear. If the answer is no — i.e., the compatibility risk of introducing `Pulse`/`Bus` outweighs the benefit — the plan falls back to an incremental patching strategy. This decision should be explicit and dated.

---

## Blockers

- None blocking doc migration work (current step).
- Phase A proper requires sufficient reference vocabulary docs to be written first (Clusters A–H, J–K).

---

## Next review

Review this document when Phase A begins.

---

## See also

- [`strategy/roadmap/milestone-q1-foundation.md`](milestone-q1-foundation.md) — full Q1 scope
- [`strategy/refactor-phases/current-status.md`](../refactor-phases/current-status.md) — phase-level status
- [`strategy/roadmap/beyond-current-quarter.md`](beyond-current-quarter.md) — Q2–Q4 outlook
