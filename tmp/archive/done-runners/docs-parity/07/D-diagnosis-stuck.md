# D — Diagnosis And Stuck Detection

Refresh of parity for `docs/07-conductor/04`, `05`, and `14`.

Generated: 2026-04-18

---

## Bottom Line

Both diagnosis and stuck detection already exist in the codebase. The audit
issue here was not absence. It was that the earlier parity brief mixed live
watcher behavior, richer support modules, and aspirational catalog language
into one backlog.

This file narrows that down.

---

## Shipped Diagnosis Surface

`DiagnosisEngine` is implemented and should be described that way.

Key facts:

- `ErrorCategory` has 20 variants at
  `crates/roko-conductor/src/diagnosis.rs:22-67`
- `SuggestedIntervention` has 9 variants at
  `crates/roko-conductor/src/diagnosis.rs:71-94`
- `DiagnosisEngine` is constructed from built-in patterns at
  `crates/roko-conductor/src/diagnosis.rs:147-153`
- the built-in pattern registry lives in
  `crates/roko-conductor/src/diagnosis.rs:277-531`

Parity wording should therefore say:

- the diagnosis engine exists,
- the pattern/category/intervention counts are concrete,
- and doc examples should use the current enum names rather than inventing
  near-miss variants.

---

## Shipped Stuck-Detection Surface

`StuckDetector` is also implemented and should be described in present tense.

Key facts:

- `StuckKind` covers 6 heuristics at
  `crates/roko-conductor/src/stuck_detection.rs:30-47`
- default thresholds live at
  `crates/roko-conductor/src/stuck_detection.rs:105-132`
- `check_stuck()` and `check_all()` live at
  `crates/roko-conductor/src/stuck_detection.rs:171-233`
- `meta_cognition()` produces a theta-frequency assessment at
  `crates/roko-conductor/src/stuck_detection.rs:235-273`

That is enough to say the stuck-detection subsystem exists.

---

## Important Nuance

### Live watcher vs richer support module

The 10-watcher hot path includes `StuckPatternWatcher`, but that is not the
same thing as saying the full `StuckDetector` surface is the main live watcher
contract.

Use this distinction:

- `StuckPatternWatcher` is part of the current watcher ensemble.
- `StuckDetector` and `MetaCognitionAssessment` are implemented support
  modules.

That keeps the docs honest without downgrading real code to "missing."

### Use real diagnosis names

Avoid parity text that names categories or interventions not found in the code.
The canonical names are the ones in `diagnosis.rs`, for example:

- `ImportError`
- `DependencyError`
- `RetryWithContext`
- `AutoFix`

---

## Carry-Forward

This file should leave later agents with a clean split:

- diagnosis exists,
- stuck detection exists,
- the production failure catalog should use current enum names,
- and deeper hot-path wiring questions belong to later code work, not this
  docs refresh.
