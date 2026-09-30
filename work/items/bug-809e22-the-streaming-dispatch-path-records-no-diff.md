+++
id = "bug-809e22"
kind = "bug"
title = "The streaming dispatch path records no diff base, so its attempts' changed_files stay empty"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tamper's report, branch work/gap-b72761 at 7531304ca)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch.rs"]
lane = "rust-cold"
parent = "spec-9230a9"
links = { depends_on = ["gap-b72761"], blocks = [], related = ["gap-b72761", "gap-6d172d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn streaming_attempts_record_their_diff_base_and_changed_files' crates/roko-cli/src/ && cargo test -p roko-cli --lib streaming_attempts_record_their_diff_base_and_changed_files"
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
