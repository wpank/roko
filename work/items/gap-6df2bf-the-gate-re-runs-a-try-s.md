+++
id = "gap-6df2bf"
kind = "gap"
title = "The gate re-runs a try's verify steps on a tree its own run_tests already passed: 36 round-v7 tries re-ran 11.2 min of checks (up to 100 s a try on the plugin phases)"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::run_verify_steps", "crates/roko-std/src/tool/builtin/task_checks.rs::read_check_runs", "crates/roko-cli/src/graph_task_dispatch/best_try.rs::BestTries"]
links = { depends_on = [], blocks = [], related = ["gap-56f761", "gap-770cb5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_green_run_tests_on_the_final_tree_is_not_run_again' crates/roko-cli/src && cargo test -p roko-cli --lib a_green_run_tests_on_the_final_tree_is_not_run_again"
+++

## Problem

After an agent hands off, the gate runs the task's verify steps on the try's final tree. It does this even when the
agent's last `run_tests` passed every one of those steps on that same tree moments before. That tool is roko's own,
and with `build: "checks"` it runs exactly the task's verify steps.

In round v7's Undertow runs, 36 tries re-ran checks that their own last self-check had passed:
- 11.2 min of gate time, 2.3% of the round's 481 run-minutes;
- a median of about 19 s a try, and up to 100 s on the plugin phases (6059db44 T1#1's gate took 101.7 s).

Expected: when the try's final tree is the tree a green `run_tests` judged, the gate takes that result, or runs only
the steps that run didn't cover.

## Why it matters

Golden path: every passing try pays for its checks twice, on its critical path. The cost grows with the check. A
JUCE plugin step takes 5–100 s even on a warm tree, and phase 5's checks are plugin checks.

## Where

- `graph_task_dispatch/verification.rs::run_verify_steps`: the gate's step loop after hand-off.
- `roko-std/src/tool/builtin/task_checks.rs::read_check_runs`: the `run_tests` runs kept under
  `.roko/state/task-checks/` (`CheckRun`).
- `graph_task_dispatch/best_try.rs::BestTries`: since gap-56f761 it keeps, with each run, the git tree that run
  judged.

## Current state

- gap-770cb5 gave agents `run_tests` with `build: "checks"`.
- gap-56f761 (merged 51316c1f7) records each run of the try's own checks with the tree it judged. Its only use is
  to put back a better tree when the final one ranks behind it (`restore_checked_best`).
- The numbers: run-analyst's `checks.py` over Undertow's records, in
  `tmp/hermes-roko-v7/run-analyst/data/tries.csv`. A try counts when its last self-check was green and its gate's
  `checks_ms` was still spent.

## Plan

1. At verify time, take the final tree's id the way gap-56f761 does, and find the newest `CheckRun` on that tree.
2. If that run covered every verify step and all of them passed, settle those steps as passed from the self-check.
   Each step keeps its command hash and its duration from the run, plus a marker that the gate didn't run it.
3. Otherwise run only the steps that run didn't pass or didn't run.
4. Only runs roko executed count (`run_tests`). The agent's own bash output never does.
5. The plan check and the delivery regression stay as they are.

There is one design choice. A step can read files outside git, such as `build/` or `artifacts/`. If the tree id
doesn't cover them, either include their hashes or keep this to steps that read only tracked files. That stays
correct.

## Done when

- A try whose last `run_tests` passed all its steps on its final tree settles `passed` without running them again.
- The verdict's `steps[]` say they came from the self-check, with that run's durations.
- A try whose final tree differs from the run's tree runs its steps as today.
- The `[[verify]]` test, `a_green_run_tests_on_the_final_tree_is_not_run_again`, passes.

## Notes

- Evidence and method: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 3 and lever L4.
- Don't weaken the gate for trees the agent changed after its last green run. The ids must match exactly.
