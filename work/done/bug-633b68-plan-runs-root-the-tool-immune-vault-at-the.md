+++
id = "bug-633b68"
kind = "bug"
title = "Plan runs root the tool-immune vault at the task lease path, so the workspace quarantine route misses it"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes/safety", "roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "2ae9d2a7f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report, checked on work/bug-928add at 090b81f3e)"
anchors = ["crates/roko-serve/src/routes/safety.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-928add"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn quarantine_route_lists_plan_run_vaults' crates/roko-serve/src/ && cargo test -p roko-serve --lib quarantine_route_lists_plan_run_vaults"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 1c7442b58. Plan-run immune state is rooted at the workspace (5d637b7fd), and the quarantine routes also read older vaults in .roko/worktrees/*. Batch 14 gate: first run on 4e030ab47 (check, clippy clean; tests pass: roko-cli 3171, roko-agent 2263, roko-core 1952, roko-learn 1203, roko-serve 986, roko-graph 472, roko-fs 259, roko-neuro 239), then re-gated on 8ce3bb131 (same code as MAIN 2ae9d2a7f) after the coordinator's rustfmt commits and serve-sec's bug-633b68 root fix: check, nightly fmt, clippy -p roko-cli -p roko-serve -p roko-core -p roko-agent -p roko-learn --keep-going -D warnings clean; roko-cli lib 3172 passed (one sibling-settle race flake passes alone, bug-779ae7); --test secret_canary 11 passed; --test secrets_and_git_guard_canary 1 passed, 1 ignored (bug-0d9ac4). Verify: its test passes in that run and its static checks pass on MAIN."
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
- 2026-09-30 (wk-serve-sec): Root fix on `work/bug-af1020` at `5d637b7fd`, after merging the working branch (batch 12b, `b128de876`); cargo verification deferred to the batch check. `immune_root` in `graph_task_dispatch.rs` and `graph_task_dispatch/streaming.rs` is now the dispatcher's workspace, not the lease path. Plan-run vault entries, immune evidence and tool controls therefore survive checkout cleanup, and a tool isolated in one attempt stays isolated in the next. `workdir` is unchanged. The route's `.roko/worktrees/*` read (`37bd6034c`) stays for vaults written before this change. No new roko-cli test: the request is built deep inside dispatch, and the edits were kept minimal because other streams share those files.
