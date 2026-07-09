# B — Watchers And Signals

Refresh of watcher and signal parity for `docs/07-conductor/01` and `09`.

Generated: 2026-04-18

---

## Bottom Line

All 10 conductor watchers are implemented and wired into `Conductor::new()`.
The parity problem here is not missing watchers. It is stale wording around
signal names, input examples, and the still-planned `CognitiveSignal` layer.

---

## Shipped Watcher Surface

The current watcher set is fixed and real:

1. `GhostTurnWatcher`
2. `ReviewLoopWatcher`
3. `IterationLoopWatcher`
4. `TestFailureBudgetWatcher`
5. `CompileFailRepeatWatcher`
6. `ContextWindowPressureWatcher`
7. `SpecDriftWatcher`
8. `CostOverrunWatcher`
9. `TimeOverrunWatcher`
10. `StuckPatternWatcher`

Source anchors:

- module list: `crates/roko-conductor/src/watchers/mod.rs:8-28`
- constructor wiring: `crates/roko-conductor/src/conductor.rs:82-99`

That means the parity docs should now say plainly: **the watcher ensemble is
implemented**.

---

## Real Signal Contract

### What the conductor emits

- Per-watcher alerts use `conductor:alert:<watcher>` from
  `crates/roko-conductor/src/interventions.rs:123-144`.
- Final policy decisions use `conductor.decision` from
  `crates/roko-conductor/src/conductor.rs:226-249`.
- Circuit-breaker refusal emits `conductor.circuit_breaker` from
  `crates/roko-cli/src/orchestrate.rs:4706-4708`.

### Why this matters

Older parity text blurred this into a generic intervention channel. That
overstates a cleaner signal taxonomy than the code currently has. The actual
names above are what later readers should use.

---

## `CognitiveSignal` Status

`CognitiveSignal` is still a planned extension.

Use this wording:

- the watcher ensemble is shipped,
- the typed cognitive-interrupt vocabulary is not,
- and the current live intervention surface remains
  `ConductorDecision` plus the emitted alert/decision engrams.

Do not call Doc 09 "built" in parity materials.

---

## Advanced Material To Defer

These should stay out of the shipped watcher contract:

- CEP watcher composition
- isolation-forest or CUSUM fusion layers
- Bayesian watcher fusion
- typed `CognitiveSignal`

They are useful future directions, but none is required to describe the
current conductor correctly.

---

## Carry-Forward

This file should leave later agents with one clear answer:

- the 10-watchers story is real,
- the current signal names are concrete,
- and the typed interrupt redesign remains future work.
