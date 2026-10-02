+++
id = "gap-1d8a49"
kind = "gap"
title = "The Graph timeout matrix runs only the shared tree and never compares the stop cause with interrupted_by"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "q-1faa0c"
anchors = ["crates/roko-cli/tests/graph_timeout_matrix.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["q-1faa0c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --test graph_timeout_matrix worktree"
+++

## Problem

graph_timeout_matrix pins the shared tree, so nothing exercises the per-task worktree mode, now the default (gap-4ec59f). No case compares the checkpoint's stop cause (`roko.run.stop@1`, gap-fab2cc) with `run.completed.interrupted_by` either.

## Plan

Add a worktree-mode variant of the timeout and SIGTERM cases, and assert the stop cause and interrupted_by agree.

## Done when

- The new cases pass.

## Notes

- Reported on 2026-10-02 by wk-honestbench, working on q-1faa0c, during the overnight close-out round.
