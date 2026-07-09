# D — Monitoring & Conductor (Doc 11)

Covers: conductor integration and background monitoring.

The audit correction here is twofold:

- conductor wiring is real,
- and the real defect is a layering seam, not the absence of a conductor subsystem.

---

## D.01 — Conductor Integration (Doc 11) — WIRED

The conductor baseline is already live:

- watcher set exists
- background `WatcherRunner` exists
- routing bias exists
- per-plan conductor checks exist
- the runtime already uses conductor output in real execution paths
- local orchestration event logging already records `InterventionFired`

This pack should stop describing conductor as an unwired or mostly theoretical feature.

## Real Remaining Seams

### 1. Background findings still stop at logging too often

The next useful batch is small:

- pick one safe background finding
- turn it into one bounded runtime effect beyond logging
- stop there

### 2. The layer crossing is real

This is confirmed in code:

- `crates/roko-conductor/Cargo.toml:15` depends on `roko-learn`
- `crates/roko-conductor/src/watchers/context_window_pressure.rs:7` imports `AgentEfficiencyEvent`
- the same watcher reads that event shape from engram payloads
- `crates/roko-cli/src/orchestrate.rs` also reads learned conductor policy output directly

That does not mean conductor is fake. It means the boundary is messy and should be named honestly.

The background runner is also real, not hypothetical: `WatcherRunner` polls recent engrams on a 30-second cadence and feeds them through `conductor.check_all(&signals)`.

## Important Boundary To Keep Honest

- local orchestration event logging records `EventKind::InterventionFired`
- the shared runtime bus is still only the 2-variant `RokoEvent` surface
- those are separate surfaces and should not be collapsed in prose

---

## Post-Audit Summary

| Item | Status | What matters now |
|------|--------|------------------|
| Conductor baseline | Done | keep it described in present tense |
| Background watcher runner | Done | add one bounded runtime effect if needed, not a remediation stack |
| Diagnosis/remediation | Partial | do not widen into autonomous remediation |
| `roko-conductor -> roko-learn` | Real issue | document it directly instead of hiding it |
| `orchestrate.rs` integration seam | Real issue | keep changes narrow in the hotspot |

---

## Batch Guidance

### O4 — Background Conductor Response

Good batch outcome:

- one background finding leads to one bounded runtime effect,
- the path is observable,
- the change does not turn into a remediation framework project.

### What To Defer

- conductor policy redesign
- learning-driven remediation overhaul
- architecture-wide event cleanup
- layer cleanup that spans multiple crates

### Important Distinction

- local intervention logging is real
- conductor runtime wiring is real
- the layer boundary is messy
- none of that implies a broad shared intervention event taxonomy already exists
