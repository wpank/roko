+++
id = "gap-abbd22"
kind = "gap"
title = "Attempt diff check: flag edits to tests, verify scripts, accept/ or gate config, and to files outside the task"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-gate/attempt_diff", "roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e9"
discovered_from = "tmp/cybernetic-harness/specs/S05-deep-audits.md (§4.3, check A1); tldr/05 P1 #13"
anchors = ["crates/roko-gate/src/attempt_diff.rs", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch.rs:4651"]
lane = "rust-cold"
parent = "spec-9230a9"
links = { depends_on = ["gap-d14a43"], blocks = [], related = ["gap-b72761", "find-d1a883"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn attempt_diff_flags_each_tamper_kind' crates/roko-gate/src/ && cargo test -p roko-gate --lib attempt_diff_flags_each_tamper_kind"

[[verify]]
command = "grep -rqw 'fn tampering_attempt_fails_before_verify' crates/roko-cli/src/ && cargo test -p roko-cli --lib tampering_attempt_fails_before_verify"
+++

## Problem

Nothing checks what an attempt changed. An agent can delete or weaken a test, add `#[ignore]`, edit a verify script,
a pinned `accept/` source or the gate settings, or edit files outside its task's `files`. The task still passes if
its verify commands do. The dispatcher records `changed_files: Vec::new()` for every attempt
(`graph_task_dispatch.rs:4651` and `:4667`).

## Why it matters

Cheap models game visible checks most (`zhao2026specbench`), and coding agents edit tests to pass
(`zhong2025impossiblebench`). tldr/05 P1 #13 and decision 6 (default: changing or deleting existing or
planner-written tests fails the attempt). Gate G5 in assessment W8. Part of epic spec-9230a9.

## Where

- **New file:** `crates/roko-gate/src/attempt_diff.rs`: a pure `check_attempt_diff(changes, policy) ->
  Vec<DiffFinding>`, which S05's audit worker can reuse later.
- `crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::settle_task_verification`: call it before the
  verify steps, next to gap-b72761's red flags, and fill `changed_files` at `:4651`.

## Current state

Checked at `41c7ffbd6`: no tamper or scope code anywhere (S05 §3). `roko_gate::diff_gate` handles vacuous diffs only.

## Plan

1. Input: `git diff --name-status -M`, plus hunks for test files, between the attempt's base and its result.
   - In a worktree, the base is the lease base.
   - In the shared tree, snapshot the tree when the attempt starts, with a temporary index (`GIT_INDEX_FILE` and
     `git write-tree`), and exclude paths declared by sibling tasks in flight.
2. Findings, from S05's A1:
   - a test deleted, renamed or emptied; `#[ignore]` or a skip added; fewer asserts in a changed test;
   - a verify script or `tasks.toml` edited;
   - a `[task.accept]` source or its `dest` edited;
   - gate configuration edited: `roko.toml`'s `[gates]`, CI config, snapshots;
   - a changed path outside `files`.
3. Policy: a tamper finding fails the attempt and becomes its retry feedback. A scope finding is recorded, and fails
   the attempt only when `[gates] diff_scope = "enforce"`.
4. Record the findings on the attempt, in the settled attempt record once epic E4 lands.

## Done when

- [ ] `attempt_diff_flags_each_tamper_kind` has a fixture for each finding kind.
- [ ] An attempt that edits a pinned accept test fails before its verify steps run.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Hot file: write `attempt_diff.rs` first; wire it after the dispatch-file split (E15.4, gap-c8e1f1).
- Reuse gap-b72761's diff snapshot if it has landed. Fail-or-label is the author's call (epic spec-9230a9).
- **Decided 2026-09-29 (Will):** tampering fails the attempt; scope findings are recorded, and enforced only on opt-in.
