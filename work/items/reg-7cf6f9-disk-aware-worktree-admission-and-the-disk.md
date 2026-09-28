+++
id = "reg-7cf6f9"
kind = "regression"
title = "Disk-aware worktree admission and the disk_budget_remaining metric were lost with Runner-v2"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-execution", "roko-fs/resources"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#resource-and-disk-lifecycle-e47----resolved-2026-08-13"
discovered_from = "doc:tmp/work-management/01-gaps-md-audit.md"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -rq --include='*.rs' 'disk_budget_remaining' crates/roko-cli/src'''
+++

E47 was recorded as RESOLVED. Runner-v2 did pre-plan log rotation, stale-target cleanup and filesystem GC, measured worktree growth, reserved aggregate disk headroom before dispatch, serialised admission under pressure, and published `worktree_count` and `disk_budget_remaining`. On 2026-09-28 nothing in `crates/` matches `disk_budget_remaining` or implements disk admission. Only `roko doctor disk`, preflight checks and the conductor watchers remain. Parallel Graph plan runs can therefore keep creating attempt worktrees and build artifacts until the disk is full.

Fix: add disk-headroom admission to Graph task dispatch. Reserve space before creating an attempt worktree, serialise under pressure, publish the canonical metrics, and restore post-gate artifact cleanup. Add a test in which admission blocks under a simulated low disk budget.
