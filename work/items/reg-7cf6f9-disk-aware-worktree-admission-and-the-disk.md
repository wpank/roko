+++
id = "reg-7cf6f9"
kind = "regression"
title = "Disk-aware worktree admission and the disk_budget_remaining metric were lost with Runner-v2"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/graph-execution", "roko-fs/resources"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "1bf49188d"
source = "gaps-md#resource-and-disk-lifecycle-e47----resolved-2026-08-13"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs:3374", "crates/roko-cli/src/graph_execution/workspaces.rs::WorktreeExecutionWorkspaceProvider::acquire", "crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-conductor/src/watchers/worktree_count.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn disk_admission_blocks_under_low_budget' crates/roko-cli/ && cargo test -p roko-cli --lib disk_admission_blocks_under_low_budget"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:00:21Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20c gate on fcdaf32ae/ca5645373 (MAIN 1bf49188d has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/gate/learn/neuro/serve; lib tests roko-cli 3273, roko-agent 2278, roko-core 1962, roko-learn 1209, roko-serve 989, roko-gate 692, roko-neuro 239, roko-acp 199, roko-dreams 100 all pass; extras: C1 1/1, C7 2/2, learn_paths 7, cost_comparison 1, bin 429, verify loop 10/10, speclint 91, including disk_admission_blocks_under_low_budget; plan run refuses below [resources] min_free_disk_mb and per-task worktrees reserve 3 GB each. Merged 2260cc7fe."
+++

E47 was recorded as RESOLVED. Runner-v2 did pre-plan log rotation, stale-target cleanup and filesystem GC, measured worktree growth, reserved aggregate disk headroom before dispatch, serialised admission under pressure, and published `worktree_count` and `disk_budget_remaining`. On 2026-09-28 nothing in `crates/` matches `disk_budget_remaining` or implements disk admission. Only `roko doctor disk`, preflight checks and the conductor watchers remain. Parallel Graph plan runs can therefore keep creating attempt worktrees and build artifacts until the disk is full.

Fix: add disk-headroom admission to Graph task dispatch. Reserve space before creating an attempt worktree, serialise under pressure, publish the canonical metrics, and restore post-gate artifact cleanup. Add a test in which admission blocks under a simulated low disk budget.
