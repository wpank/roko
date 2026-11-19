+++
id = "gap-fef073"
kind = "gap"
title = "Plan generation's spec check reads no pinned tests: it scores the candidate tasks.toml alone in a scratch directory, so a planner-written test asserting one call to two values (HF6) is refused only when the plan loads"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-ba2841 (worker, 2026-10-27)"
discovered_from = "gap-ba2841"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::spec_scores"]
links = { depends_on = [], blocks = [], related = ["gap-ba2841"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn generation_retries_a_plan_whose_pinned_test_contradicts_itself' crates/roko-cli/src && cargo test -p roko-cli --lib generation_retries_a_plan_whose_pinned_test_contradicts_itself"
+++

## Problem

Plan generation scores each candidate `tasks.toml` with the spec rules and asks the planner again on a hard fail
(`spec_scores` in `crates/roko-cli/src/plan_generate/pipeline.rs`). It writes the candidate alone to a scratch
directory, without the planner's `accept:accept/<file>` blocks, so the spec check reads none of the pinned
acceptance tests the plan names. A planner-written test that asserts one call to two values no one result meets
(HF6, gap-ba2841) therefore passes generation; the plan is written, and only the plan-load gate refuses it when
the plan runs.

## Why it matters

Goal `truth`, and cost. At generation the planner can still fix its own contradictory test with the HF6 line as
feedback, as it fixes a vacuous verify step (HF2) today. Once the plan is written, a person must find and fix it.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::spec_scores`: writes the candidate to `tempfile::tempdir()` and
  calls `roko_gate::spec_quality::lint_files` on it.
- `try_extract_and_validate` in the same file already holds the candidate's accept blocks
  (`accept_blocks::extract`) when it calls `spec_scores`.
- `crates/roko-gate/src/spec_quality/contradiction.rs::pinned_tests`: reads each `[task.accept]` source from the
  plan directory.

## Current state

Since gap-ba2841 the plan-load gate (every Graph run) and `roko plan validate --spec-quality` read pinned tests
for HF6; generation does not.

## Plan

1. Pass the accept blocks to `spec_scores` and write them beside the scratch `tasks.toml` (`accept_blocks::write`),
   with the tests the plan directory already holds on regeneration, as `accept_blocks::check_sources` stages them.
2. A test: a scripted planner whose first plan pins a test asserting `round_to_dollars(150)` to 2 and 3 is asked
   again with the HF6 line, and its second, consistent plan is written.

## Done when

Generation retries a plan whose pinned test contradicts itself, with the HF6 line in the retry prompt; the
`[[verify]]` passes.
