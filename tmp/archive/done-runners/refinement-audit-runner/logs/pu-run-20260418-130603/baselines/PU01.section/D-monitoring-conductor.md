# D — Monitoring & Conductor (Doc 11)

Covers: conductor integration and background monitoring.

The audit correction here is twofold:

- conductor wiring is real,
- and the real defect is a layering seam, not the absence of a conductor subsystem.

---

## D.01 — Conductor Integration (Doc 11) — WIRED

The conductor baseline is already live:

- watcher set exists,
- background `WatcherRunner` exists,
- routing bias exists,
- per-plan conductor checks exist,
- and the runtime already uses conductor output in real execution paths.
- local orchestration event logging already records `InterventionFired`

The pack should stop describing this as an unwired or mostly theoretical feature.

## Real Remaining Seams

### 1. Background findings still stop at logging too often

The next useful batch is small:

- pick one background finding,
- turn it into one bounded runtime effect beyond logging,
- and stop there.

### 2. The layer crossing is real

This is confirmed in code:

- `crates/roko-conductor/Cargo.toml` depends on `roko-learn`
- `crates/roko-conductor/src/watchers/context_window_pressure.rs` imports `AgentEfficiencyEvent`
- `crates/roko-cli/src/orchestrate.rs` also imports learned conductor policy types directly

That does not mean conductor is fake. It means the boundary is messy and should be named honestly.

The background runner is also real, not hypothetical: it polls recent signals on a 30-second cadence and feeds them through `conductor.check_all(&signals)`.

---

## Post-Audit Summary

| Item | Status | What matters now |
|------|--------|------------------|
| Conductor baseline | Done | keep it described in present tense |
| Background watcher runner | Done | add one bounded runtime effect if needed, not a new remediation stack |
| Diagnosis/remediation | Partial | do not widen into autonomous remediation |
| conductor -> learn boundary | Real issue | document it directly instead of hiding it |

---

## Batch Guidance

### O4 — Background Conductor Response

Good batch outcome:

- one background finding leads to one bounded runtime effect,
- the path is observable,
- and the change does not become a remediation framework project.

### What To Defer

- conductor policy redesign
- learning-driven remediation overhaul
- layer cleanup that spans architecture-wide event and policy work

### Important Distinction

- `EventKind::InterventionFired` in the local event log is real.
- A shared, broad runtime intervention event taxonomy is not.
