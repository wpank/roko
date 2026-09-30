+++
id = "bug-633b68"
kind = "bug"
title = "Plan runs root the tool-immune vault at the task lease path, so the workspace quarantine route misses it"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/safety", "roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "0b84bc9fa"
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

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-af1020` at `37bd6034c`; cargo verification deferred to the batch check. Plan option 2: the quarantine and incidents routes also read one vault per checkout directory in `.roko/worktrees/` (where `--worktree-per-task` puts attempt checkouts), and sum their counts. Each entry and incident names its `vault`, and a new `vaults` list shows every vault read. Without `--worktree-per-task` the lease is the workspace, so those runs already landed in the workspace vault. Plan option 1 remains open: rooting plan runs at the workspace needs `immune_root` in `graph_task_dispatch.rs` and `graph_task_dispatch/streaming.rs`, which batch 12b owns. Until then, a checkout's vault disappears when the checkout is cleaned up. Test: `routes::safety::tests::quarantine_route_lists_plan_run_vaults`.
