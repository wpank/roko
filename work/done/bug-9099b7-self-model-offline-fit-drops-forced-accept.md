+++
id = "bug-9099b7"
kind = "bug"
title = "Self-model offline fit drops forced_accept verdicts; the live sink counts them as failures"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/self-model", "roko-learn/telemetry", "roko-cli/runtime-feedback"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "08caddc4c"
source = "wave-8 follow-up reports 2026-10-03 (PK49 gap-7ec3ef)"
discovered_from = "gap-7ec3ef"
anchors = ["crates/roko-learn/src/self_model/ingest.rs", "crates/roko-learn/src/telemetry/records.rs::learning_label_for", "crates/roko-cli/src/runtime_feedback/self_model.rs::SelfModelOutcomeSink"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn forced_accept_labels_agree_between_offline_fit_and_the_live_sink' crates/roko-learn/ && cargo test -p roko-learn forced_accept_labels_agree_between_offline_fit_and_the_live_sink"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T05:50:53Z"
commit = "08caddc4c"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-04T04:53:00Z"
forced = false
evidence = "Gate 14a (merged 08caddc4c): roko-learn and roko-cli lib tests pass; verify forced_accept_labels_agree_between_offline_fit_and_the_live_sink passes. One label rule, Label::of_verdict (self_model/mod.rs), read by the offline fit and the live sink: a forced_accept teaches nothing on either path, per S01's learning label and SC3."
+++

## Problem

The self-model's offline fit and its live production sink disagree on how `forced_accept` verdicts count.

- **Offline fit** (`crates/roko-learn/src/self_model/ingest.rs`, ~line 210): `let passed =
  verdict.learning_success()?;` — the `?` means a verdict whose `learning_success()` returns `None` makes the
  whole function return `None`, dropping the record entirely; no `Unit` is built for it.
  `AttemptVerdictRecord::learning_success` (`crates/roko-learn/src/telemetry/records.rs:1046`) returns `None`
  exactly when `learning_label` is `None`, and `learning_label_for`'s own doc comment
  (`records.rs:454-456`) is explicit: "`None` when the attempt carries no learning signal (unverified, **forced
  accept**, infra and harness outcomes)." So the offline fit silently excludes every `forced_accept` attempt from
  its training/scoring data — it is treated as "teaches nothing," not as a fail.
- **The live `SelfModelOutcomeSink`** (`crates/roko-cli/src/runtime_feedback/self_model.rs:73`): its own doc
  comment says "The gate label of `verdict` (S01 §4.1): its learning label, with `forced_accept` a fail." —
  i.e. the production sink counts a `forced_accept` as an explicit failure (`Some(false)`), not an excluded
  record. `self_model/mod.rs:233`'s `Label.y_gate` field doc agrees with this: "`forced_accept` is a fail."

So the two paths that are supposed to produce the same kind of labelled unit treat the same verdict kind
differently: one drops it, the other fails it.

## Why it matters

Goal: cybernetic, M3 self-model (S04). The offline fit (used to calibrate and validate the self-model before
promotion) and the live sink (used to keep it updated in production) training on different label rules for
`forced_accept` means the offline-fit calibration doesn't actually describe what the live model is doing —
exactly the mismatch S04's calibration gate (`CalibrationGate`) is supposed to prevent by scoring against real
outcomes.

## Where

- `crates/roko-learn/src/self_model/ingest.rs` (the offline fit's unit-building, ~line 210).
- `crates/roko-learn/src/telemetry/records.rs::learning_label_for`, `::learning_success` (the shared label rule
  the offline fit relies on, which excludes forced_accept).
- `crates/roko-cli/src/runtime_feedback/self_model.rs::SelfModelOutcomeSink` and
  `crates/roko-learn/src/self_model/mod.rs::Label` (the live path's "forced_accept is a fail" convention).

## Current state

Confirmed both conventions exist, in the exact call paths described, disagreeing on the same verdict kind.

## Plan

1. Decide which convention is correct for S04's purposes (exclude forced_accept entirely, matching the shared
   `learning_label_for` rule everything else in the codebase uses, or count it as a fail, as the self-model's own
   docs currently claim) — this is a decision, not just a code fix, since `learning_label_for`'s rule is shared
   infrastructure other learners rely on too.
2. Make the offline fit and the live sink agree, whichever way is decided.
3. Add a regression test: a `forced_accept` verdict produces the same training signal (excluded, or labelled
   fail) through both the offline ingest path and the live `SelfModelOutcomeSink`.

## Done when

- The offline fit and the live sink treat `forced_accept` identically.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK49's M3 self-model work (gap-7ec3ef, done).

## Progress

- bug-9099b7: implemented at 42200b6b9 on `work/bug-9099b7`; cargo verification deferred to the batch gate.
  Decision: a forced accept teaches nothing (the shared S01 §4.1 learning label, null for
  `forced_accept`, and S01's SC3), not a fail. `Label::of_verdict` (`self_model/mod.rs`) is the one rule; the
  offline fit (`ingest.rs`) and the live sink (`runtime_feedback/self_model.rs`) both call it, and the sink's
  special case is gone. S04 §4.1's "`forced_accept` counts as 0" line now disagrees with the code and should
  follow S01.
