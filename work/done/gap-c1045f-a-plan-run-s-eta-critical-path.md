+++
id = "gap-c1045f"
kind = "gap"
title = "A plan run's ETA (critical_path_eta_updated) is elapsed / done x remaining, sent only when a task completes: it ignores the running tasks, the DAG and task sizes (run c91b63ea: 12, 26, 14 min when 74, 52, 38 were left)"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "M"
subsystem = ["roko-cli/runner"]
created = 2026-10-20
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "e3d26d10e"
source = "round-v7-hermes-pilot#E-p2-run-c91b63ea"
discovered_from = "agent:hermes-pilot (round v7 phase 2, run c91b63ea)"
anchors = ["crates/roko-cli/src/runner/graph_tui_bridge.rs::EtaTracker"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn eta_counts_running_parallel_tasks_and_the_task_after_them' crates/roko-cli/src && cargo test -p roko-cli --lib -- eta_counts_running_parallel_tasks_and_the_task_after_them"

[closed]
at = 2026-10-21
at_ts = "2026-10-20T23:48:09Z"
commit = "e3d26d10e"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-20T22:27:33Z"
forced = false
evidence = "e3d26d10e. runner/eta.rs: each unfinished task's duration on the rung the ladder starts it on is log-normal, fitted to rung_stats::task_times (first try's start to its verified pass, by start rung and tier) over the workspace's last 20 runs, this run's passed tasks added live: history (3+ of the same rung and tier), scaled (the rung's or every rung's, by TaskTier::max_loc), or guess (half the attempt timeout); a running task counts the conditional median and p90 of D - a given D > a; the plan's estimate is the longest path along depends_on and at least the total over [conductor] max_agents. GraphTuiBridge takes the plan's tasks (plan_tasks, from plan_runner with GraphTaskDispatcher::planned_start_rung) and publishes on every task start and finish and, through refresh_eta in the status poll, every 60 s while tasks run; critical_path_eta_updated keeps eta_minutes (the median) and gains eta_p90_minutes, computed_at_ms and basis {source, tasks, runs}; a plan without known tasks keeps the proportional figure. The literal verify passes on the branch: eta_counts_running_parallel_tasks_and_the_task_after_them (T1 short, T2 and T3 in parallel, then T4: after T1 the estimate is about 30 min where elapsed x 3 says 12; after T2 it counts T3's 20 minutes of running); also a_plan_with_known_tasks_publishes_a_range, durations_fall_back_from_history_to_a_guess, task_times_run_from_the_first_try_to_the_pass, normal_cdf against normal_quantile. Full lib suites: roko-cli 3945, roko-core 2045, roko-learn 1391, roko-serve 1075; fmt and clippy clean. $0 e2e (tmp/hermes-roko-v7/roko-speed/bench/eta-bench.sh, the C++ fixture, a 4-task diamond on a 30 s-a-turn stand-in): a fresh workspace said 'about 45 min, up to 118, a guess' and refreshed every minute, then 'about 2 min, history (3 tasks)' once three tasks had passed, with 64 s left; a second run with the first's records said 'about 4 min, up to 6, history (4 tasks, 1 run)' at its start with 4.2 min left, then 3, 2, 2, 1 min as 2.6, 1.6, 1.1 and 0.1 min were left."
+++

## Problem

Round v7, Undertow phase 2, fresh run `c91b63ea` (roko d7d1fbb8b). It had 4 tasks: T1, then T2 and T3 in parallel,
then T4. All ran on glm-4.7. roko published `critical_path_eta_updated` three times, and each figure was far too low:

| When | `eta_minutes` | Time actually left |
|---|---|---|
| 21:15:21 (T1 done) | 12 | 74 min |
| 21:37:14 (T2 done) | 26 | 52 min |
| 21:51:05 (T3 done) | 14 | 38 min |

The run ended at 22:29:25. Each figure was 3 to 6 times too low.

## Why

`EtaTracker::eta_minutes` (`crates/roko-cli/src/runner/graph_tui_bridge.rs:39`) computes elapsed / done × remaining.
The event is named "critical path", but the estimate:

- ignores which tasks are running, and for how long (T2 and T3 had run 22 min when the first figure came out);
- ignores the DAG: T4 can't start until T3 ends, and T2 and T3 run side by side;
- ignores task sizes and tiers: T1 is integrative and took 3.8 min, so it set the pace for two architectural tasks that
  took 22 and 36 min;
- is published only when a task completes (`node_completed_with_verdict`), so it is never refreshed in between.

Readers show the latest value as "~N min left". The Hermes plugin's run page showed 21:15's "~12 min left" at 21:33,
and "~14 min left" at 48 min elapsed with 30 min still to go. The page side, showing the figure's age or counting it
down, is plugin-wow's P-38.

## Plan

- Estimate the remaining time along the DAG's critical path. For a running task, take the larger of its elapsed time
  and an expected duration for its tier and rung: the per-rung `try_s` median the summary already computes, or the
  learned history. For a waiting task, take its expected duration. The run's estimate is the longest path through
  the tasks still to finish.
- Publish it on a timer as well, every minute or so while tasks run, or put a timestamp on the event so readers can
  age it.

## Done when

On a plan with a short first task and two long parallel tasks before a last one, the ETA after the first task
counts the parallel tasks' running time and the last task. Checked by the test in `[[verify]]`.
