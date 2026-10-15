+++
id = "gap-b1f7b9"
kind = "gap"
title = "A plan's own verify steps feed gate history under the rung they check, so gate-thresholds.json learns from authored test steps"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch", "roko-gate/registry"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-47"
anchors = ["crates/roko-gate/src/registry.rs::rung_for_gate_name", "crates/roko-cli/src/graph_task_dispatch/gate_learning.rs::record_gate_verdict_metrics", "crates/roko-cli/src/graph_task_dispatch/gate_learning.rs::skip_advice"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn authored_test_steps_feed_the_test_rungs_history' crates/roko-cli/ && cargo test -p roko-cli authored_test_steps_feed_the_test_rungs_history"
+++

## Problem

After agent H's four live runs (2026-10-15, HEAD 7b6a5af44; 15 attempts, 15 passing `custom:unit-tests` verify
steps), `.roko/learn/gate-thresholds.json` records 0 of 0 on every rung (H, finding N5). In R3 (2026-10-02) rung 2
reached 8 of 8. A plan's authored verify steps, which are the checks most plans have, feed no gate history.

The cause, as far as the code shows: gate learning keeps a step only when `roko_gate::rung_for_gate_name(phase)`
maps its name to a canonical rung (`crates/roko-cli/src/graph_task_dispatch/gate_learning.rs:149-152` counts only
those; `skip_advice` at `:115-126` does the same). An authored step is named `custom:<label>`, which maps to no rung,
so it is dropped. Not confirmed by a test.

## Why it matters

Goal `learning` (adaptive gate thresholds, the P2-LRN-6 loop). The adaptive thresholds and the temperament skip
advice see none of a plan's own checks, so the loop the papers describe as learning gate pass rates from the
workspace's history has nothing to learn from on plan runs whose checks are authored.

## Where

- `crates/roko-gate/src/registry.rs::rung_for_gate_name` (`:160`): name to canonical rung.
- `crates/roko-cli/src/graph_task_dispatch/gate_learning.rs`: what reaches `GateThresholds` and the metrics.
- `crates/roko-cli/src/graph_task_dispatch/feedback.rs` (`gate_thresholds_path`, `:44-52`): where the file is written.

## Current state

Workspace rungs (compile, lint, test) are observed. Authored steps carry a `phase` (TSS v1: `structural`, `test`,
`plan`, ...) that says which rung they belong to.

## Plan

1. Map an authored step to a rung by its declared `phase` (`test` to the test rung, `structural` and `lint` to theirs).
   Leave a step with no phase that names a known rung unobserved.
2. Keep the canonical names working as they are.
3. Test `authored_test_steps_feed_the_test_rungs_history`: a task whose `custom:unit-tests` step has `phase = "test"`
   passes, and the test rung's observation count goes up by one.

## Done when

`authored_test_steps_feed_the_test_rungs_history` passes (the `[[verify]]` command), and a scripted plan run with
authored test steps leaves non-zero counts in `gate-thresholds.json`.

## Notes

Evidence: `tmp/status-quo/2026-10-15/live/H-live-run.md:332` (learning-data table) and `:350` (N5). Check first how
the step's label and phase reach `gate_learning` (the `phase` argument may already be the label), so the mapping is
added at the right place.
