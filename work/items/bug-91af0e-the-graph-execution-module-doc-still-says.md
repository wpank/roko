+++
id = "bug-91af0e"
kind = "bug"
title = "The graph_execution module doc still says delivery is backed by MergeQueue and GitHubWorkflow"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-merge-safety's report on bug-a3760a)"
anchors = ["crates/roko-cli/src/graph_execution/mod.rs:5"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["bug-a3760a"], blocks = [], related = ["spec-f830c4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE 'MergeQueue|GitHubWorkflow' crates/roko-cli/src/graph_execution/mod.rs"
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
