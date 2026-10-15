+++
id = "q-61b3f8"
kind = "question"
title = "Should plan-set parallelism stay opt-in at 1, or run independent plans side by side by default?"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "b883db4e6"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-7c9e48"
anchors = ["crates/roko-cli/src/graph_execution/plan_set.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-7c9e48"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n 'max_parallel_plans' crates/roko-core/src/config/"

[closed]
at = 2026-10-15
at_ts = "2026-10-15T16:30:30Z"
commit = "b883db4e6"
by = "status-quo-2026-10-15"
executor = "claude-session"
via = "manual"
forced = true
evidence = "2026-10-15 decision (Will delegated it to the coordinator's recommendation, tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md, decision 9; he can veto it): Stay opt-in: the default remains max_parallel_plans = 1, and a preset or an explicit setting turns it up. Re-ask after per-plan worktrees land (find-d1a883 option d) and a measured plan-set run shows what side-by-side costs in dollars and disk. Why: The side-by-side scheduler works and is tested (gap-7c9e48, test independent_plans_run_side_by_side), but Will's decision 3 on find-d1a883 keeps plan-set footprints conservative now and leaves per-plan worktrees for later: today the independence check trusts declared files and reads verify commands word by word, so two plans judged independent can still collide. Running them side by side by default would also multiply per-run cost and disk use, which the item names as the trade-off. The opt-in already exists (crates/roko-core/src/config/presets.rs:165 sets 4 in a preset; :92 keeps 1), so nothing is lost. If vetoed: With side-by-side on by default, the conductor default becomes, for example, 2 or 4. Plan sets then run concurrently, with higher peak cost and disk, and collisions are caught only by footprints and merge."
+++

## Problem

gap-7c9e48 found that the side-by-side plan scheduler works and is tested (`independent_plans_run_side_by_side`, plus the new diamond test), but the default still runs one plan at a time. Turning it on by default changes cost and disk use per run.

## Plan

Will decides the default. If it changes, update the config default and docs, and keep the order test for `max_parallel_plans = 1`.

## Done when

- The decision is recorded, and the default matches it.

## Notes

- Reported on 2026-10-01 by wk-planrun, working on gap-7c9e48, during the evening close-out round.

2026-10-15 (status-quo assessment at 7b6a5af44, finding 41): the verify can never pass: `grep -n 'max_parallel_plans' crates/roko-core/src/config/` greps a directory without `-r` (rc 2). Use `grep -rqn` and, better, a check of the decided default (for example the `max_parallel_plans` default in `crates/roko-core/src/config/`) once the question is answered.
