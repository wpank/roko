+++
id = "bug-58a1d6"
kind = "bug"
title = "A task that passes a hand-back of the plan's check is reported unverified, and so is its run: the hand-back's watch reports the try's node with no verdict, so phase 6 read \"unverified\" though T4's try passed and the plan check then passed"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "b3e4a8d8e"
source = "round v8 hermes-pilot-v8 and run-analyst-v8: phase 6 run 23837ede invocation 3 (Undertow 8945b3c, roko e722f0e1f), T4 try 3; the lead asked roko-planner-v8 to decide and fix"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::rerun_handed_back_tasks", "crates/roko-cli/src/graph_execution/plan_runner.rs::FlowWatch"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["find-5bcba7", "bug-7e1b6b", "gap-6df2bf"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'a hand-back.s try is reported as its verdict says' crates/roko-cli/src/graph_execution/plan_runner.rs && cargo test -p roko-cli --lib a_failed_plan_check_goes_back_to_the_task_whose_file_it_names"

[closed]
at = 2026-11-13
at_ts = "2026-11-12T23:44:44Z"
commit = "b3e4a8d8e"
by = "roko-planner-v8"
via = "manual"
executor = "claude-agent"
size = "S"
forced = false
evidence = "The hand-back's live watch published its try's node with no verdicts (BTreeMap::new), so a passing try read unverified (bug-7e1b6b's rule) and so did the run; it now holds a completed node back (FlowWatch::holding_completions, hold_back) until the graph's output gives its verdict. Self-checked steps were not the cause (T5 and T6 passed on them). Red first (201274c22 on main 85a96423d): a_failed_plan_check_goes_back_to_the_task_whose_file_it_names saw T1's task_completed outcomes [passed, unverified]; now [passed, passed]. Green on the trial 0083d6ffa (main 2d44b5c79 + work/bug-3e04a6 + work/bug-58a1d6): 61 targeted tests pass (both items' tests, graph_task_dispatch budget, try_allowance, helper_calls and verify_depth), roko-cli lib 4154 passed 0 failed, roko-learn lib 1415 passed, integration suites graph_budget_resume, honest_verdicts_canary, graph_plan_callers, default_engine, runner_facades_e2e and tier_ladder_canary 9 passed, cargo clippy --workspace --no-deps -D warnings and nightly fmt --check clean."
+++

## Problem

A task that passes a hand-back of the plan's check is reported unverified, and so is the run.

Phase 6, run 23837ede invocation 3 (Undertow 8945b3c → b1fd747):
- **What happened.** After T6, the plan check `tools/check.sh --plugin` failed once on T4's step cells, and roko
  handed it back to T4.
- **T4's try 3 passed.** Its verdict in `attempts.jsonl` is `gate_verdict: "passed"`, with three steps, all passed:
  - `custom:structural`, the deletion check, which roko ran;
  - `verify:1/test` (`tools/spec.sh 6 steps`) and `verify:2/test` (`tools/check.sh --plugin`), both self-checked:
    "passed in this try's own checks at 23:11:45, on this tree: not run again".
- **The label.** The run's events say `task_completed {"task_id": "T4", "outcome": "unverified"}`. serve's state
  for the run says "unverified" too (`RunState::of_ended_run`: one unverified task makes the run unverified). The
  Hermes card reads "unverified · gates 5/5 · tasks 5/6", and the portal reads "not checked".
- **The plan check passed.** roko then ran the plan's check again, which passed (exit 0, 78.7 s), and the run ended
  `succeeded` and delivered.

Self-checking is not the cause. T5's try 3 and T6's try 1 also passed on self-checked steps, and they were reported
`passed`.

The cause is the hand-back's watch. `rerun_handed_back_tasks` (`graph_execution/plan_runner.rs`) watches the try's
graph as the plan's graph is watched, but its live `publish` passes `BTreeMap::new` as the tasks' verdicts. The
plan's watch passes the checkpoint's recorded verdicts instead. A completed node with no verdict is reported
unverified (`node_outcome`, bug-7e1b6b). The final report after the graph ends does have `output.gate_verdicts`, but
it sends only what the ticks did not, so it never corrects T4.

## Why it matters

- A run whose every task passed its checks, and whose plan check then passed, reads "unverified" on the portal, in
  Hermes and in serve's run state.
- The guide's phase 6 hero card was changed to end on "not checked" because of it.
- Goal: truth.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs`: `rerun_handed_back_tasks`, `FlowWatch::publish` and
  `hold_back_failures`.

## Plan

1. A watch that has no live source of its tasks' verdicts holds back a task that completed, as a stopping run holds
   back a failure. The hand-back's try graph has no checkpoint, so its watch is one.
2. When the try's graph ends, the final report gives the completed task the verdict in its output: T4 reads
   `passed`.
3. Extend the test `a_failed_plan_check_goes_back_to_the_task_whose_file_it_names`: T1's `task_completed` events are
   `passed` and then `passed` again for the hand-back's try, never `unverified`.

## Done when

- The `[[verify]]` command passes.

## Notes

- Evidence (read-only, in Undertow's `.roko`):
  - `runs/23837ede-1d23-4d53-b8a9-fbfa362760d0/attempts.jsonl` (T4 attempt 3);
  - the run's event index `events-by-run/c41ac029…jsonl` (task_completed, gate_result, the
    `graph.plan_check_handback` lines).
- The lead's two options, a passing plan check settling the task or the card naming why, are not needed. The
  verdict was `passed`, and only the report lost it. A self-checked step counts as a pass by design (gap-6df2bf,
  audited as `passed_self_checked`).
- Related: find-5bcba7 (the hand-back), bug-7e1b6b (a node with no verdict reads unverified).
