# 07-Conductor Parity Analysis

Gap analysis of `docs/07-conductor/` against the current conductor, orchestrator, runtime-process, learning, and agent-process codepaths that actually consume or emit conductor signals.

Generated: 2026-04-16

---

## How To Use This Batch

This batch should be treated as **conductor runtime activation + control-surface contract cleanup**, not as a license to implement every Yerkes-Dodson, Good Regulator, cognitive-signal, federation, or self-healing idea described across docs `00`-`15`.

- Prefer wiring already-shipped conductor subsystems into `orchestrate.rs` before adding new control theory.
- Treat `crates/roko-cli/src/orchestrate.rs`, `crates/roko-conductor/src/{health,stuck_detection,circuit_breaker,state_machine}.rs`, and the process-accounting split between `roko-runtime` and `roko-agent` as the main runtime seams.
- Keep Yerkes-Dodson, Good Regulator, typed `CognitiveSignal`, CEP composition, and conductor federation explicitly bounded unless a batch says otherwise.
- Every batch should be able to stop with a clear `PASS`, `FAIL`, or `BLOCKED` result and leave behind evidence: files changed, commands run, outputs, and explicit deferrals.

Recommended single-agent serial order inside batch `07`:

`C1 -> C2 -> C3 -> C4 -> C5 -> C6 -> C7 -> C8`

Reasoning:

- `C1` and `C2` activate the two biggest dark runtime seams first.
- `C3` and `C4` resolve the two biggest state-accounting failures after that.
- `C5` turns the half-wired state-machine / timeout story into a bounded runtime contract.
- `C6` and `C7` clean up the remaining conductor-specific code/docs drift once the runtime decisions are settled.
- `C8` is the final frontier-truth pass after the live surfaces are clarified.

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-architecture.md](A-architecture.md) | 00 | A.01-A.13 | 7 DONE / 6 PARTIAL / 0 NOT DONE |
| [B-watchers-signals.md](B-watchers-signals.md) | 01, 09 | B.01-B.25 | 17 DONE / 5 PARTIAL / 3 NOT DONE |
| [C-decision-space.md](C-decision-space.md) | 02, 03 | C.01-C.17 | 12 DONE / 3 PARTIAL / 2 NOT DONE |
| [D-diagnosis-stuck.md](D-diagnosis-stuck.md) | 04, 05, 14 | D.01-D.20 | 10 DONE / 9 PARTIAL / 1 NOT DONE |
| [E-health-adaptive.md](E-health-adaptive.md) | 06, 10, 12, 13 | E.01-E.30 | 8 DONE / 9 PARTIAL / 13 NOT DONE |
| [F-theory-learning.md](F-theory-learning.md) | 07, 08, 11, 15 | F.01-F.28 | 13 DONE / 5 PARTIAL / 10 NOT DONE |
| [BATCHES.md](BATCHES.md) | — | 8 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |
| [AUDIT-LOG.md](AUDIT-LOG.md) | — | Initial generation log | Historical reference |
| [run-docs-parity.sh](run-docs-parity.sh) | — | Batch runner | Launcher |

Doc `INDEX.md` is absorbed into this file.

---

## Overall Parity: 67/133 items DONE (50%)

The conductor batch is in a split but favorable state:

- the **core conductor loop is real and already production-relevant**,
- but several important support systems are still **built but not really in path**,
- and the later theory docs still mix **shipping control surfaces**, **half-used runtime seams**, and **design-only theory** in one voice.

### Tier 1 — Should Exist Now (runtime-critical)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| E.05 | `HealthMonitor` is built but still not on a real orchestrator tick path | NOT DONE | HIGH |
| D.11 | `StuckDetector` + `MetaCognitionHook` are built but still dark at runtime | NOT DONE | HIGH |
| E.14 | `ProcessSupervisor` exists on `PlanRunner`, but live agent spawns bypass it | PARTIAL | HIGH |
| C.09 | plan-level circuit-breaker state does not persist across snapshots or restarts | NOT DONE | HIGH |

### Tier 2 — Should Exist Soon (operational quality)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| E.09 | typed `PhaseTransition` exists but orchestrator still emits raw JSON | PARTIAL | MEDIUM |
| E.10 | `adaptive_timeout_ms` exists but has no production consumer | PARTIAL | MEDIUM |
| E.15 | attempt-tracking / stale-exit race fix is still absent | NOT DONE | MEDIUM |
| C.16 | promised 120 s per-plan-per-watcher cooldown is absent | NOT DONE | MEDIUM |
| D.09 | category-to-intervention table drifts from the actual built-in pattern actions | PARTIAL | MEDIUM |
| D.17 | Doc 14 still names variants and interventions that do not exist | PARTIAL | MEDIUM |
| B.19 | doc-09 `CognitiveSignal` surface is still design-only despite the banner | NOT DONE | MEDIUM |
| F.21 | doc 15 still understates `ConductorBandit` wiring status | DONE (doc stale) | MEDIUM |

### Tier 3 — Future / Theory / Frontier

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| B.17-B.18 | CEP watcher composition, isolation forest, streaming anomaly stack | NOT DONE | LOW |
| E.22-E.30 | Yerkes-Dodson pressure dial, flow detection, cooperation roll-up, stigmergy pressure | NOT DONE | LOW |
| F.05-F.14 | LivenessMonitor, ImplicitGuidance, self-model accuracy, Bayesian threshold learner, Kalman, forward predictor | NOT DONE / PARTIAL | LOW |
| F.22, F.24-F.26 | learned-conductor wrapper, federation, self-healing, triple-loop learning | NOT DONE | LOW |

### Already Shipped

| ID | Title | Status |
|----|-------|--------|
| A.02-A.07, A.12-A.13 | composite policy, ten-watcher ensemble, breaker, severity mapping, diagnosis/stuck modules, orchestrator integration, module layout | DONE |
| B.01-B.16, B.20-B.25 | watcher catalog, decision pipeline, replacement of cognitive-signal semantics with `ConductorDecision`, signal emission, breaker and routing-bias coupling | DONE / PARTIAL where noted |
| C.01-C.08, C.10-C.15, C.17 | breaker core, decision-space invariants, severity defaults, real intervention signals | DONE / PARTIAL where noted |
| D.01-D.08, D.10, D.12, D.15-D.20 | diagnosis engine, stuck taxonomy, failure-catalog mapping, watcher-side coverage | DONE / PARTIAL where noted |
| E.02, E.08, E.11-E.13, E.17-E.21 | health status enum, timeout matrix, graceful shutdown, resource-account helpers, process isolation basics | DONE / PARTIAL where noted |
| F.01-F.04, F.09-F.10, F.15-F.21 | OODA framing, anomaly detector, adaptive thresholds, provider health, retry-path conductor bandit | DONE / PARTIAL where noted |

---

## Execution Boundaries

These are valid findings, but they should usually be handled outside the core runtime-hardening work of batch `07`:

| Item | Better Home | Why |
|------|-------------|-----|
| Yerkes-Dodson pressure dial, `PressureBandit`, `FlowDetector`, pressure profiles | later learning / pressure-tuning pass | current conductor runtime works without them |
| Good Regulator Brier/Kalman/ForwardPredictor self-model work | later self-model pass | the base conductor loop still has dark runtime seams |
| typed `CognitiveSignal` enum and unified algedonic channel | later signal-channel redesign | current `ConductorDecision` and emitted kinds already drive production paths |
| watcher CEP composition, isolation forest, CUSUM | later anomaly research pass | existing watcher ensemble plus `AnomalyDetector` already ship |
| federated conductors, self-healing, triple-loop learning | later governance / meta-learning pass | batch `07` should focus on single-plan infrastructure first |
| Linux cgroup resource limits | later deployment-hardening pass | current resource-accounting is in-process only |

Batch `07` should usually produce:

- one real runtime caller for dark conductor modules,
- one canonical owner for agent/process accounting,
- a snapshot-safe breaker contract,
- a bounded state-machine/timeout story,
- and cleaner truth-in-advertising across the theory-heavy docs.

---

## Critical Conductor Issues

1. **Two substantial conductor modules are still dark.** `HealthMonitor` and `MetaCognitionHook` are both production-shaped but still not called from the orchestrator.
2. **Agent/process accounting is split across two owners.** `PlanRunner` thinks it has a supervisor, while live agent spawn uses a separate registry path.
3. **The breaker contract is not restart-safe.** The docs promise persistence across snapshots, but a relaunch reconstructs a fresh breaker.
4. **The state-machine story is only half real.** Typed `PhaseTransition` and adaptive timeout computation both exist, but production still underuses them.
5. **Theory and status docs still overstate or understate important surfaces.** Doc 12 and doc 09 overclaim; doc 15 understates live retry-path learning.

---

## Key Insight

Batch `07` does **not** mainly need more conductor theory.

It needs a tighter contract between:

- the **real conductor loop** already running in production,
- the **dark or half-wired support systems** around that loop,
- and the **docs** that still blur the line between runtime behavior and design intent.

That means the highest-value work here is usually:

1. wire the shipped support systems that still have no caller,
2. unify state ownership where the runtime currently has two answers,
3. make snapshot, timeout, and phase-transition behavior explicit,
4. defer the pressure/self-model/federation frontier unless a batch clearly owns it.

---

## Batch 07 Success Definition

Batch `07` is successful when:

- `HealthMonitor` and `MetaCognitionHook` are either on a real runtime path or explicitly documented as intentionally out of path,
- agent/process ownership is no longer ambiguous,
- breaker persistence across restarts has one honest answer,
- `PhaseTransition` / timeout semantics are easier to verify from code,
- and the conductor docs no longer mix “shipped”, “half-used”, and “design-only” in misleading ways.

Use [AUDIT-LOG.md](AUDIT-LOG.md) for the detailed initial generation notes, not as the primary execution brief.
