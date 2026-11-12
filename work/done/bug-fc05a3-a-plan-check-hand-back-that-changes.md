+++
id = "bug-fc05a3"
kind = "bug"
title = "A plan-check hand-back that changes nothing is settled already_satisfied and accepted while the plan check still fails, so a hand-back can't fix a cross-task failure (phase 4 run c4fa953f: T7#2 already_satisfied, T6#2 $0.79 within its own files, the round $1.13 for nothing; the bone pixels came from T5's Look.cpp)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph-execution"]
created = 2026-10-21
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "9177ae9ac"
source = "round-v7-hermes-pilot#N-p4-run-c4fa953f"
discovered_from = "agent:hermes-pilot (round v7, phase 4 run c4fa953f, the lead ask)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs"]
links = { depends_on = [], blocks = [], related = ["bug-60ad88"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_hand_back_that_changes_nothing_is_not_satisfied_while_the_plan_check_fails' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_hand_back_that_changes_nothing_is_not_satisfied_while_the_plan_check_fails"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T10:42:31Z"
commit = "9177ae9ac"
by = "roko-exact (round v7)"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-21T10:19:31Z"
forced = false
evidence = "9177ae9ac: a failed whole-plan check goes back as one try (plan_runner::hand_back_plan_check, plan_handback). A task whose own verify steps run the failing test leads it (runs_failing_test: phase 4's 'P4.06*' matches the failure), else the highest tier, the last among equals (lead_task). It may change the files of every implicated task (the owners of the files the output names, else every task that changed a file: implicated_tasks), and its task (handback_task) is judged by its own steps, the other tasks' and the plan's [[meta.verify]], so a try that changes nothing while the check fails is rejected pre_verify:no_changes, never already_satisfied. At most 1 retry (HANDBACK_RETRIES, HandbackRetries) and none after a try that changed nothing (GraphTaskDispatcher::changed_nothing, from the pre-verify screen's diff); the excerpt leads with the failure (is_failure_line); a failed step that printed nothing is not handed back. Verify: a_hand_back_that_changes_nothing_is_not_satisfied_while_the_plan_check_fails passes in the shared tree and under --worktree-per-task (T1: passed, gate_failed; result unchanged; nothing reached the plan branch); a_failure_across_two_tasks_goes_back_as_one_try_at_both_files (phase 4's shape: two tasks draw bone, the check names neither file and fails until both are fixed) succeeds after one hand-back led by T2; with a_hand_back_whose_try_fails_ends_the_plan, a_plan_check_is_handed_back_at_most_max_handbacks_times and a_failed_plan_check_goes_back_to_the_task_whose_file_it_names, 5 of 5; plan_handback 8 of 8."
+++

## Problem

When a plan's whole-plan check fails after every task has passed, roko hands the failure back to its tasks. A
hand-back try that changes nothing is settled `already_satisfied` and accepted while the plan check still fails, so the
hand-back can't fix a failure that crosses tasks.

Round v7, Undertow phase 4 run `c4fa953f` (plan `p4-undertow-phosphor-editor`, roko dafb6316f, OpenAI ladder):

- All 7 tasks passed their own checks: T1–T3, T5–T7 on the first try, T4 on strong after two mid tries.
- **09:25:37Z.** The post-merge `tools/check.sh --plugin` failed.
  - From `.roko/state/graph/p4-undertow-phosphor-editor/delivery-check.log`: core 22/22 passed, the plugin suite 27/28.
  - `spec_p4_display` failed P4.06: "before any note the panel shows 1145 bone pixels" (the limit is 91), and "the bone
    pixels span 61 17 1058 715, outside the CHEST".
  - The cause crosses tasks. T5's `plugin/ui/Look.cpp` uses `boneDim` for slider thumbs, tooltips and the static
    layer, and T7's `plugin/ui/Status.cpp` draws status text in `bone`. P4.06 itself is T6's check, and passed in T6's
    own try before T5 and T7 were merged.
- **09:25:38Z.** "handing the plan's failed check back to its tasks", round 1. Six `retry_feedback` events with
  `check: "plan-check"` (`.roko/events.jsonl`) gave T1, T3, T4, T5, T6 and T7 the failure, each with "the file(s)
  this task changed last". T5's names `plugin/ui/Look.cpp`, the real cause.
- **09:25:41–09:29:12Z, T7#2** (gpt-5.4-mini, 3 turns, $0.015): "attempt changed nothing, and its verify steps pass
  on the unchanged tree: the task was already satisfied". It was accepted onto the plan branch (04f86ab), although
  the plan check it had been handed still failed.
- **09:29:15–09:33:13Z, T6#2** (gpt-5.4, 40 turns, `continues: 2`, $0.793): it edited only its own
  `plugin/ui/Displays.cpp` and stayed red on P4.06. T6#3 started at once; the run was cancelled at 09:34:32Z ($0.322
  of T6#3).
- The hand-back round cost $1.13 and fixed nothing. T5, the task whose file causes the failure, had not run by the
  cancel. The run ended cancelled at $2.607 of its $3 cap.

## Why it matters

A plan's final check exists to catch exactly this kind of cross-task break. Here no single task's own checks fail, so
every try that changes nothing is "already satisfied", and roko spends the cap on the tasks it happens to run first.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs`: the plan-check hand-back ("handing the plan's failed check
  back to its tasks").
- `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`: "attempt changed nothing, and its verify steps pass on the
  unchanged tree: the task was already satisfied".
- `crates/roko-cli/src/graph_task_dispatch/verification.rs`: retry feedback, `check: "plan-check"`.

## Plan

Some options for the owner:
- In a plan-check hand-back, a try settles `already_satisfied` only when the plan check passes on its tree. The plan
  check, or at least its failing step, is part of the hand-back try's verify.
- Hand back first to the tasks whose files the failure names, or the file it points at. Here P4.06's "bone pixels"
  maps to the files that use the bone colour, not to the test's owner.
- Stop the round once every handed-back task comes back unchanged, and say that the plan check fails across tasks,
  rather than re-running tries on the strong rung.

## Done when

The test in `[[verify]]` passes: a hand-back try that changes nothing, while the plan check still fails on its tree, is
not settled `already_satisfied`.

## Follow-ups

1. **Start the hand-back's try a rung higher.** The try runs on the rung the lead task's own tries ended on, and
   those tries passed an easier job. A cross-task try is harder than the task's first try: it must find the cause
   in files it didn't write (phase 4's T6 lead, for bone drawn by T5's `Look.cpp` and T7's `Status.cpp`), from the
   output of a whole-plan check rather than its own step, and fix them without breaking the checks of the tasks
   that own them. Not done in bug-fc05a3, on the lead's call (2026-10-21).
2. Bisect the plan branch's accepted commits to find the first commit at which the check fails, to name the cause
   when the output names no file. It needs a build per commit tried.
