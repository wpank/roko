+++
id = "gap-2118f0"
kind = "gap"
title = "The portal never says which task blocked another, although TaskRun carries blocked_by"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report on gap-f59fe9)"
anchors = ["apps/portal/src/lib/runState.ts", "apps/portal/src/lib/taskRows.ts"]
lane = "frontend"
links = { depends_on = [], blocks = [], related = ["gap-f59fe9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'blocked by' apps/portal/src/lib/taskRows.ts"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:35Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:50Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

gap-f59fe9 added DashboardEvent::TaskBlocked with blocked_by, and the portal counts such tasks, but no portal view shows 'blocked by T1'.

## Why it matters

Visibility: a blocked task looks merely skipped.

## Plan

Show the blocker on the task row and in the task detail; add a taskRows test.

## Done when

- [ ] A blocked task shows which task blocked it
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-filer4): implemented on work/gap-2bc1b9; the portal typecheck and vitest run are deferred to the coordinator's gate (TypeScript isn't closed early). TaskRowModel gains `blocked` (blockedLabel(blockedBy, blockedReason), such as "blocked by T1: T1 failed") for tasks whose run phase is `blocked`. TaskList shows it as a line under the row, and as a "blocked" section in the expanded detail. Tests in taskRows.test.ts cover a blocked task, an ordinary skipped one, and the label's three forms. The static verify (`grep 'blocked by' taskRows.ts`) passes.
