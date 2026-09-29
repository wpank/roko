+++
id = "bug-b0fd73"
kind = "bug"
title = "For accept plans, authored_plan_running reports that tasks.toml no longer matches on every run"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_checkpoint"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-accept's report on gap-d14a43, branch work/gap-d14a43 at 37b6d7c95)"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = ["gap-d14a43"], blocks = [], related = ["gap-1b5636", "gap-ba4d01"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_accept_plan_matches_its_own_tasks_toml' crates/roko-cli/src/ && cargo test -p roko-cli --lib an_accept_plan_matches_its_own_tasks_toml"
+++

## Problem

On `work/gap-d14a43` (37b6d7c95), `graph_checkpoint.rs::authored_plan_running` (:448 on the branch) compares the tasks a run converted with the current tasks.toml. On a mismatch it logs "tasks.toml no longer matches the tasks this run converted" (:440).

For a plan that uses `[task.accept]`, the converted tasks carry the pinned steps and their hashes in `task_def_json`, and the tasks.toml text does not. So the comparison fails on every run, and the warning fires for nothing (reported by wk-accept).

## Why it matters

Specs a cheap model can execute (epic spec-e57870): a warning that always fires hides the case it exists for, a tasks.toml edited while its plan runs. Resume and the checkpoint's authored-plan record rely on the same comparison.

## Where

`authored_plan_running` and `authored_plan` in `graph_checkpoint.rs`, and the fingerprint they compare.

## Current state

This exists only on the unmerged branch, which gap-d14a43 merges.

## Plan

1. Before comparing, strip the pinned accept steps (identified by their `# roko accept:` header) from the converted side. Alternatively, compile the tasks.toml side the same way before comparing.
2. Add `an_accept_plan_matches_its_own_tasks_toml`: an accept plan's run matches its own unchanged tasks.toml, and an edited tasks.toml still mismatches.

## Done when

- [ ] Running an unchanged accept plan logs no mismatch, and editing its tasks.toml still does.
- [ ] The `[[verify]]` command passes.

## Notes

- Merge gap-d14a43 first.
