+++
id = "gap-2c9412"
kind = "gap"
title = "A try that ends at its rung's bound is failed without its checks: the climb reads turn_cap instead of the failing acceptance check, and work that was done is thrown away"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "d4abcdfaa"
source = "team-lead, round v6 (dogfood-desktop's showcase rehearsal, 2026-10-17)"
discovered_from = "gap-3e770b (b0b57f6dd)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::dispatch", "crates/roko-cli/src/graph_task_dispatch/ladder.rs::publish_attempt_settled", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/try_allowance.rs::TryAllowance"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-3e770b", "bug-f6b99f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_bounded_try_is_judged_by_its_checks' crates/roko-cli/src && cargo test -p roko-cli --lib a_bounded_try_is_judged_by_its_checks"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T19:17:37Z"
commit = "090aa2fef"
forced = false
evidence = "a_bounded_try_is_judged_by_its_checks and its _when_streaming twin pass. On the merged tree d4abcdfaa: roko-cli lib 3812 passed, 0 failed; clippy clean on roko-core, roko-cli and roko-serve; fmt clean. Zero-cost plan runs with a scripted model that keeps going past its 30-turn bound (tmp/hermes-roko-v5/roko-caps/e2e-judged, merged binary). Right work: the bounded try on mid passes its check, attempt_settled says 'passed its checks after reaching its 30-turn bound', answer.txt=42 is delivered, exit 0. Wrong work: two gate_failed/agent tries on mid (failed_check verify:0/test). The second prompt carries 'AssertionError: answer.txt holds 41, not 42' and the resume note. The task climbs to strong, which passes, exit 0."
+++

## Problem

Since gap-3e770b (b0b57f6dd), a try on a rung the task can still climb from is bounded. A try that uses up the rung's
turns or spend returns before the verify steps run, so the task's checks never judge the work it left. dogfood-desktop
found two consequences while rehearsing the showcase:
- The climb reads "turn cap", not the failing acceptance check.
- A try whose work was done, but that kept going until the bound, fails without being checked.

The pitch is "escalate when the acceptance checks fail", so the check has to be what fails the try.

## Why it matters

The showcase's climb take needs this. The climb must show the check that failed, with its key line in the next try's
retry feedback. And cheap work that passes its checks must not be thrown away and paid for again on a dearer rung.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs::dispatch`: the `!dispatch.result.success` branch settles a bounded
  stop (a turn cap at the rung's bound, `TryStop::Rung` for its spend) and returns before `settle_task_verification`.
  So does the interrupted call of an agent that runs its own loop, which the watchdog stops at the rung's spend
  (`AttemptInterrupted::AllowanceSpent`).
- `crates/roko-cli/src/graph_task_dispatch/streaming.rs`: the streaming path verifies only a successful result.
- `crates/roko-cli/src/graph_task_dispatch/try_allowance.rs`: the rung's bound.

## Plan

1. When an unsuccessful try ended at its rung's bound (its turns, or its spend), run the task's verify steps on the work
   it left, as for a finished try:
   - if they pass, the try passes and its work is accepted, and its `attempt_settled` says it passed after reaching
     its bound;
   - if they fail, the try fails on its check (`gate_failed`, blamed on the agent), the check's lines feed the next
     try's retry feedback, and two such tries climb as before.
2. The next try still gets the resume note: its bound and its work on disk.
3. The run's cap keeps the last word. A try the plan's budget stopped, or one that ended while the run was stopping,
   is not verified.
4. The top rung, and a pinned model, keep the tier's cap and its raise after a turn-cap stop, unchanged.

## Done when

`a_bounded_try_is_judged_by_its_checks` passes:
- a scripted model that keeps going past the bound with correct work passes on its first try;
- one with wrong work fails its check twice, with the check's line in the second prompt, and climbs.

A zero-cost plan run with a scripted model shows the same. The `[[verify]]` command passes.
