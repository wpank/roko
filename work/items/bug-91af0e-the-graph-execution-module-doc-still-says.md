+++
id = "bug-91af0e"
kind = "bug"
title = "The graph_execution module doc still says delivery is backed by MergeQueue and GitHubWorkflow"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "207f91da2"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-merge-safety's report on bug-a3760a)"
anchors = ["crates/roko-cli/src/graph_execution/mod.rs:5"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["bug-a3760a"], blocks = [], related = ["spec-f830c4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE 'MergeQueue|GitHubWorkflow' crates/roko-cli/src/graph_execution/mod.rs"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Fixed by wk-integrate's module-doc correction (d5f4ff350), merged in 207f91da2. Batch 16d gate on dd58c3db2 (MAIN 207f91da2 has the same code), after the coordinator's scope fix for plan_verify (cfed1c6f2): cargo check --workspace --tests, nightly fmt, clippy -p roko-cli -p roko-core -p roko-execution --keep-going -D warnings clean; lib tests pass: roko-cli 3236 (two known load flakes, turn_policy's 1 s test and gate_rows' writer wait), roko-core 1953, roko-execution 245; integration: --test plan_branch_integration 2 passed (C3 kill-and-resume, C4 whole-plan gate), --test merge_proof 4, --test runner_integration 6. Verify: static check passes on MAIN."
+++

## Problem

The module doc of `crates/roko-cli/src/graph_execution/mod.rs` says the host adapters are "backed by the existing `WorktreeManager`, `MergeQueue`, and `GitHubWorkflow`". After bug-a3760a (`809ae920d`), no module under `graph_execution/` uses `MergeQueue`: delivery merges with git plumbing and is serialized by the delivery service's merge slot. No module there has ever used `GitHubWorkflow` either, because delivery publishes with `git push`.

## Why it matters

A doc that names the wrong mechanism sends readers and agents to the wrong code. This is how spec-f830c4's plan came to route merges through `PlanMerger`. Epic spec-9a3131.

## Where

`crates/roko-cli/src/graph_execution/mod.rs`, lines 1-5 (the module doc).

## Current state

`git grep -n 'MergeQueue\|GitHubWorkflow' work/bug-a3760a -- crates/roko-cli/src/graph_execution/` finds only this doc comment.

## Plan

1. After bug-a3760a merges, rewrite the doc to name what the submodules actually use:
   - `WorktreeManager` for workspaces;
   - git plumbing in `GitDeliveryBackend` for delivery merges;
   - `git push` for publication.

## Done when

- [ ] The doc names no mechanism that the module doesn't use.
- [ ] The `[[verify]]` command passes.
