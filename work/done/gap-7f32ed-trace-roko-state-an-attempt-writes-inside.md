+++
id = "gap-7f32ed"
kind = "gap"
title = "Trace roko state an attempt writes inside its worktree, route it to the workspace root, and keep .roko/ out of accepted commits"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "4f6ae807f"
source = "bug-412a5e fix (w4-length, 2026-10-03)"
discovered_from = "bug-412a5e"
anchors = ["crates/roko-cli/src/graph_execution/workspaces.rs", "crates/roko-cli/src/dispatch_v2.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-412a5e", "gap-d254a3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn accepted_worktree_attempt_commits_no_roko_state' crates/roko-cli/ && cargo test -p roko-cli accepted_worktree_attempt_commits_no_roko_state"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T17:45:02Z"
commit = "4f6ae807f"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-03T17:16:23Z"
forced = false
evidence = "Gate 9a (work/backlog-batch-9a, merged into main as 4f6ae807f): cargo check --workspace --tests, cargo clippy --workspace -D warnings, nextest --lib 3,538 roko-cli tests, roko-cli bin 436 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), the bench analysis suite 96 passed, the shakedown 8/8 against the batch binary; every [[verify]] passes. Traced with the batch binary in a repo that doesn't ignore .roko/: acceptance (git add --all) committed only learn/efficiency.jsonl and its lock, which de1b9fb48 moved to the root; every other attempt-path writer already uses the workspace root. Test accepted_worktree_attempt_commits_no_roko_state (30d9f9d8c). Excluding agent-written .roko/ files from acceptance unless HEAD tracks them is a policy question left for Will."
+++

## Problem

An attempt in a per-task worktree may write roko's own state under the worktree's `.roko/`, and accepting the
attempt may stage the whole worktree, so that state reaches the plan branch (or is lost when the worktree is
removed). bug-412a5e fixed one writer (the model-call efficiency row now goes to the workspace root via
`immune_root`); nobody has traced the others. Found by w4-length (2026-10-03).

## Why it matters

Learned state written into a worktree is either committed into the user's plan branch, polluting their repository and
causing sibling conflicts (as `.roko/learn/efficiency.jsonl` did in PK03's test), or silently lost.

## Where

Writers under `crates/roko-cli/src/graph_task_dispatch/`, `crates/roko-cli/src/dispatch_v2.rs` and the feedback sinks
that resolve paths from the request's `workdir`; acceptance in `crates/roko-cli/src/graph_execution/workspaces.rs` and
`crates/roko-cli/src/orchestrator/worktree/mod.rs`.

## Current state

Unknown which other writers resolve against the worktree.

## Plan

1. Run a per-task-worktree attempt in a test repo that doesn't ignore `.roko/` and list every file under the
   worktree's `.roko/` after the attempt (and what acceptance stages).
2. Route each writer that belongs to the workspace to the workspace root (the `immune_root` idiom). If anything must
   stay in the worktree, report it: whether acceptance should exclude `.roko/` is a policy question.
3. A test: after an accepted worktree attempt, the plan branch's commit contains no `.roko/` path.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Related: bug-412a5e (closed with gate 9), PK03's conflict test (gap-d254a3).
