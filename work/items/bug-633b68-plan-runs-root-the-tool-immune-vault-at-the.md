+++
id = "bug-633b68"
kind = "bug"
title = "Plan runs root the tool-immune vault at the task lease path, so the workspace quarantine route misses it"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/safety", "roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/routes/safety.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn quarantine_route_lists_plan_run_vaults' crates/roko-serve/src/ && cargo test -p roko-serve --lib quarantine_route_lists_plan_run_vaults"
+++

## Problem

`GET /api/safety/quarantine` (`crates/roko-serve/src/routes/safety.rs`) lists the review vault that the tool-immune boundary writes, at `roko_agent::quarantine_vault_path` under the workspace root (:3-8). Graph dispatch sets `immune_root` to the task's lease path (`graph_task_dispatch/streaming.rs:179`, `Some(lease.path.clone())`, and `graph_task_dispatch.rs:1057`), so a plan run's quarantined results land in per-task worktrees. The route never sees them.

## Why it matters

Release blockers: withheld tool results from plan runs can't be reviewed where the API says they are. p3.

## Where

The vault path in `routes/safety.rs`, and `immune_root` in Graph dispatch.

## Plan

1. Write plan-run vault entries under the workspace root (with the plan and task in each entry), or make the route also scan the plan runs' vaults.
2. Add `quarantine_route_lists_plan_run_vaults`.

## Done when

- [ ] Quarantined results from plan runs appear in `GET /api/safety/quarantine`.
- [ ] The `[[verify]]` command passes.
