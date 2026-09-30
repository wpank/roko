+++
id = "bug-f81e9b"
kind = "bug"
title = "ProviderModelOutcomeRecord::from_efficiency_event reads the efficiency row's outcome, which carries no learning label"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/provider_model_outcome"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-settle's report on gap-88c547, branch work/gap-88c547 at 24b23580d)"
anchors = ["crates/roko-learn/src/provider_model_outcome.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = ["gap-88c547"], blocks = [], related = ["gap-88c547", "gap-8f6206", "gap-eb82c9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn outcome_records_carry_the_settled_learning_label' crates/roko-learn/src/ && cargo test -p roko-learn --lib outcome_records_carry_the_settled_learning_label"
+++

## Problem

gap-88c547 (branch `work/gap-88c547`, not merged at d2cc43346) moves the learning consumers from `success` to the settled learning label. One consumer can't follow: `ProviderModelOutcomeRecord::from_efficiency_event` (`crates/roko-learn/src/provider_model_outcome.rs:220` on the branch) builds its record from the efficiency row, and keys on `event.outcome` (:224). The efficiency row carries no learning label, so the record learns from the old outcome, in which an unverified attempt can count as a success.

## Why it matters

Cybernetic core (epic spec-6ac537): provider and model outcome statistics feed routing, so they should learn from the same settled label as everything else. p3, because the other consumers are fixed.

## Where

`from_efficiency_event`, and the efficiency event (`AgentEfficiencyEvent`) that lacks the label.

## Plan

1. Put the settled learning label (and blame) on efficiency events when they are written, or join each event to its episode's label by attempt key.
2. Make `from_efficiency_event` use the label.
3. Add `outcome_records_carry_the_settled_learning_label`.

## Done when

- [ ] Provider and model outcome records use the settled learning label.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-88c547's branch.
- Implemented on `work/bug-f81e9b` at `1086064d7`; cargo verification deferred to the batch check.
- The premise was partly off. `from_efficiency_event` keyed its status on `gate_passed == Some(true)`, and read
  `outcome` only as `task_type`. So an unverified Graph attempt counted as a failure, not a success. So did every
  other Graph dispatch row, including a verified pass, whose pass sits on its own gate row. roko-serve's
  projection derives provider outcomes from every efficiency row whenever no outcome file exists, and Graph runs
  never write one.
- **Decision (2026-09-30):** a label can't ride on `AgentEfficiencyEvent` without touching its roughly 32
  struct-literal constructors, several of them in batch-12 files. So a row now counts only through its own gate
  verdict: `gate_passed: None` records no outcome, while gate rows and bench rows (`gate_passed` from the SWE-bench
  resolution) keep counting. The settled label reaches provider outcomes through `from_episode` (gap-88c547).
- Left open: `from_efficiency_event` still copies `outcome` into `task_type`.
