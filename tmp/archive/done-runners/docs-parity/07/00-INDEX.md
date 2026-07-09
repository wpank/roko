# 07-Conductor Parity Refresh

Audit-aligned refresh of `docs/07-conductor/` against the current codebase.

Generated: 2026-04-18

---

## Purpose

The previous parity pass was directionally right but overscoped. It treated
section `07` as a runtime-hardening backlog. This refresh narrows the batch
back to docs parity:

- describe the shipped conductor honestly,
- separate implemented support modules from target-state theory,
- keep real follow-up items visible without presenting them as already built.

This batch is **docs-only**. It does not plan or authorize code edits outside
`tmp/docs-parity/07/`.

---

## Current Picture

### Shipped now

- The conductor core is real: `Conductor::new()` wires all 10 watchers and
  `Conductor::evaluate()` runs the breaker -> watcher pass ->
  `WorstSeverityPolicy` decision flow.
- The decision surface is real: `Severity` is `Info | Warning | Critical`,
  `ConductorDecision` is `Continue | Restart | Fail`, and the orchestrator
  consumes those outcomes.
- The diagnosis surface is real: `DiagnosisEngine` ships 34 built-in patterns,
  20 `ErrorCategory` variants, and 9 `SuggestedIntervention` variants.
- Support modules are real: `HealthMonitor`, `SystemSnapshot`,
  `StuckDetector`, `MetaCognitionAssessment`, `PhaseTransition`, and
  `ProcessSupervisor` all exist in code.
- Retry-path learning is real: `ConductorBandit` is wired into the retry path;
  the missing piece is the broader learned-policy replacement described in the
  frontier docs.

### Corrections this refresh applies

- Stop describing theory chapters as implementation specs. OODA, Good
  Regulator, and Yerkes-Dodson stay as framing unless backed by code.
- Stop describing conductor federation and self-healing as present-tense
  architecture. They are Phase 2+.
- Stop understating the live conductor surface. `RoutingBias`, the 10-watcher
  ensemble, the circuit breaker, diagnosis, and retry-path bandit are all
  implemented.
- Fix stale wording around signal kinds and ownership seams. The live emitted
  kinds are `conductor:alert:<watcher>`, `conductor.decision`, and
  `conductor.circuit_breaker`.

### Real follow-ups that remain

- `roko-conductor -> roko-learn` is a real layer violation via
  `context_window_pressure.rs`.
- The clean architectural fix is still modest: add a generic `Bus<E>` trait to
  `roko-core` and converge on shared `RokoEvent` usage rather than inventing
  new kernel nouns.
- Process ownership is still split between `ProcessSupervisor` and
  `roko-agent/src/process/registry.rs`.
- Some support modules exist without being the main hot-path contract. The docs
  must reflect that nuance rather than flattening everything into either
  "fully wired" or "missing."

---

## File Guide

| File | Refresh focus |
|------|---------------|
| [A-architecture.md](A-architecture.md) | Shipped conductor architecture, `RoutingBias`, layer violation, real carry-forward |
| [B-watchers-signals.md](B-watchers-signals.md) | All 10 watchers implemented; real signal names; `CognitiveSignal` deferred |
| [C-decision-space.md](C-decision-space.md) | Live decision contract, breaker semantics, bounded doc caveats |
| [D-diagnosis-stuck.md](D-diagnosis-stuck.md) | Diagnosis and stuck-detection surfaces that already exist |
| [E-health-adaptive.md](E-health-adaptive.md) | Health/process/timeouts as implemented support surfaces with honest caveats |
| [F-theory-learning.md](F-theory-learning.md) | Theory chapters marked informational or Phase 2+ |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | Refreshed anchors for the current source layout |
| [BATCHES.md](BATCHES.md) | Narrow docs-refresh batches only |
| [AUDIT-LOG.md](AUDIT-LOG.md) | Initial generation log plus this refresh summary |
| [context-pack/](context-pack/) | Short operator context aligned to the narrowed scope |
| [run-docs-parity.sh](run-docs-parity.sh) | Runner text updated for docs-refresh execution |

---

## What To Defer

Keep these visible, but label them clearly as later work:

- Generic `Bus<E>` introduction and event unification follow-up
- Process ownership cleanup between runtime supervisor and agent registry
- Learned conductor as an `InterventionPolicy` replacement
- Conductor-learning federation, self-healing, and multi-level conductor
- Typed `CognitiveSignal`
- Yerkes-Dodson pressure tuning, flow detection, and model pressure profiles
- Good Regulator self-model metrics and calibration machinery

---

## Success Definition

This parity refresh is successful when:

- every file under `tmp/docs-parity/07/` describes the conductor in the right
  tense,
- shipped conductor runtime surfaces are called shipped,
- theory-heavy chapters are marked informational or future work,
- and the remaining follow-ups are small, concrete, and explicitly deferred.
