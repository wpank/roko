+++
id = "bug-809e22"
kind = "bug"
title = "The streaming dispatch path records no diff base, so its attempts' changed_files stay empty"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "9c0b9aed0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tamper's report, branch work/gap-b72761 at 7531304ca)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch.rs"]
lane = "rust-cold"
parent = "spec-9230a9"
links = { depends_on = ["gap-b72761"], blocks = [], related = ["gap-b72761", "gap-6d172d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streaming_attempts_record_their_diff_base_and_changed_files' crates/roko-cli/src/ && cargo test -p roko-cli --lib streaming_attempts_record_their_diff_base_and_changed_files"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 670a3bde2. Streamed attempts record their diff base (record_diff_base) and report changed_files from the pre-verify screen's kept paths. Batch 15a gate on 9005da604, re-assembled as 8a2ee8bca with only settle's rustfmt commit changing two files' formatting (MAIN 9c0b9aed0 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-compose -p roko-core -p roko-learn -p roko-serve --keep-going -D warnings clean; lib tests pass: roko-cli 3190 (two flakes, the turn_policy escalated-timeout test and graph_run_routing_observations_survive_a_crash, pass alone and in their module), roko-agent 2268, roko-core 1952, roko-learn 1204, roko-serve 986, roko-compose 560; cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

On gap-b72761's branch, the batch dispatch path calls `self.record_diff_base(…)` before running an attempt (`graph_task_dispatch.rs:863`), so the attempt's diff and `changed_files` can be computed afterwards. The streaming path (`graph_task_dispatch/streaming.rs`) never calls it, and builds its outcomes with `changed_files: Vec::new()` (:396, :418). Every check that reads the diff (empty-diff rejection, scope, tamper checks) sees nothing for streamed attempts.

## Why it matters

Check each attempt's diff for tampering and scope (epic spec-9230a9): a streamed attempt escapes every diff-based check.

## Where

`streaming.rs`, next to where it starts the attempt, and `record_diff_base`.

## Plan

1. Call `record_diff_base` in the streaming path at the same point as the batch path (a one-line change), and fill `changed_files` from the diff.
2. Add `streaming_attempts_record_their_diff_base_and_changed_files`.

## Done when

- [ ] Streamed attempts have a diff base and their changed files.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-b72761's branch.
- **wk-tamper (2026-09-30):** Implemented on `work/bug-809e22` at `81de82626`; cargo verification deferred to the
  batch check. `streaming.rs` has two one-line changes. It calls `record_diff_base(&attempt_key, &lease.path, None)`
  right after minting the attempt key, where the batch path makes its call; the streaming `TaskLease` has no base
  revision, so the base is a snapshot. The success outcome now takes `changed_files: self.take_changed_files(..)`.
  The screen keeps each attempt's changed paths when it computes the diff. They are kept in `diff_snapshot.rs`,
  keyed by attempt and replacing the task's earlier attempts, because the base is forgotten once verify passes. The
  screen now computes the diff before its other checks, so a rejected attempt still reports what it changed. Test:
  `streaming_attempts_record_their_diff_base_and_changed_files`. A streamed attempt that changes nothing is now
  rejected as `pre_verify:no_changes`, and one that changes `src/lib.rs` reports it in `changed_files`.
