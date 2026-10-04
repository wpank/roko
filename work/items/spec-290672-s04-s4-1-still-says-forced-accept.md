+++
id = "spec-290672"
kind = "spec"
title = "S04 S4.1 still says forced_accept counts as 0; the shipped rule (S01, SC3) treats it as missing"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self-model"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-14 follow-up reports 2026-10-04 (bug-9099b7)"
discovered_from = "bug-9099b7 (closed; the fix deliberately moved to S01's rule, S04 never updated)"
anchors = ["tmp/cybernetic-harness/specs/S04-self-model-routing.md"]
lane = "paper"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

S04 §4.1 says `forced_accept` counts as 0 (`tmp/cybernetic-harness/specs/S04-self-model-routing.md:125`:
"`y_gate` is the gate verdict (`TaskGateVerdict`). `forced_accept` counts as 0 and `unverified`
as missing."), but the shipped code treats `forced_accept` the same as `unverified`: both are
missing (`None`), neither counts as 0. `Label::of_verdict`
(`crates/roko-learn/src/self_model/mod.rs:252-259`) maps through
`AttemptVerdictRecord::learning_success()` (`crates/roko-learn/src/telemetry/records.rs:1052-1062`),
which is `None` unless `learning_label` is exactly `Some(1)` or `Some(0)`; `of_verdict`'s
`.map()` then produces no `Unit`/`Label` at all when that's `None`. `bug-9099b7` (closed, fixed
at `08caddc4c`) made this the single, unified rule for both the offline fit and the live sink,
explicitly to follow S01 §4.1 (the learning label is null for `forced_accept`) and SC3
(`forced_accept` never changes learned state) — i.e. the fix deliberately moved away from
"counts as 0" and S04's line was never updated to match.

## Why it matters

Goal: cybernetic, M3 self-model spec accuracy (S04 §4.1). Two specs (S01 and S04) now disagree
about the same quantity (`y_gate`'s value for a `forced_accept` verdict) in a way that matters
for anyone fitting or auditing the self-model's labels: S01 and the shipped code say "teaches
nothing," S04 still says "teaches a 0" (a concrete failure label, which would actively train
the model against `forced_accept` outcomes rather than skipping them).

## Where

- `tmp/cybernetic-harness/specs/S04-self-model-routing.md:125` (the line to amend).
- `tmp/cybernetic-harness/specs/S01-instrumentation.md` §4.1 (the rule S04 should now match).
- `crates/roko-learn/src/self_model/mod.rs::Label::of_verdict` (read-only reference; the shipped
  behavior).

## Current state

The code and S01 agree (`bug-9099b7`'s fix and its test,
`forced_accept_labels_agree_between_offline_fit_and_the_live_sink`). S04 §4.1 still states the
old "counts as 0" rule it was presumably written against before SC3/S01 §4.1 superseded it.

## Plan

1. Replace S04 §4.1's line 125 with the current rule: `forced_accept` and `unverified` are both
   missing (`None`), not one 0 and the other missing — matching S01 §4.1 and SC3.

## Done when

- S04 §4.1 states the same `forced_accept`/`unverified` label rule as S01 §4.1 and the shipped
  `Label::of_verdict`.

## Notes

- 2026-10-04 (wave-14 follow-up, bug-9099b7, main HEAD `b42c34ff9`): confirmed by reading
  `Label::of_verdict`, `AttemptVerdictRecord::learning_success`, and S04 §4.1's literal text.
  No `[[verify]]` command: the fix is spec-text only, filed as `kind = "spec"`.
