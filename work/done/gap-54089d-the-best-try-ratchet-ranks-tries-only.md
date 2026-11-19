+++
id = "gap-54089d"
kind = "gap"
title = "The best-try ratchet ranks tries only by verify steps passed, so when no try passes the first (multi-check) step every try ties and the latest wins: phase 3's 131-of-132 tree was overwritten by 77/149 and 0/2"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "7d785e01f"
source = "round-v7-hermes-pilot#M-p3-resume-dfba45e8"
discovered_from = "agent:hermes-pilot (round v7 phase 3 runs bdc4461e and dfba45e8)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/best_try.rs"]
links = { depends_on = [], blocks = [], related = ["bug-62ae1e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tries_that_pass_the_same_steps_rank_by_the_failing_steps_passed_cases' crates/roko-cli/src && cargo test -p roko-cli --lib -- tries_that_pass_the_same_steps_rank_by_the_failing_steps_passed_cases"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T05:47:19Z"
commit = "7d785e01f"
via = "manual"
size = "S"
forced = false
evidence = "7d785e01f: each failed try keeps, beside the steps it passed, how far it got in the first verify step it failed, by that step's own runner (retry_feedback::StepProgress: the step's label, its passed assertions and test cases): doctest and Catch2 summary lines (test cases: N | P passed, assertions: N | P passed) and ctest's (X% tests passed, F tests failed out of N) are read in best_try.rs, cargo/pytest/Jest/go test through roko_gate::parse_test_counts, summed over a step's runs. BestTry::behind ranks: fewer steps, or as many and fewer assertions, then fewer cases, in the same first failing step; ties go to the latest; a try behind the best one gets the best try's files back before the next try, whose feedback says how far each got (\"in `step`, try 1 passed 1 test case and 131 assertions, and this try 0 test cases and 77 assertions\"). The progress is persisted with the best try (serde default, old retry-feedback.json files load); a resumed task starts from its best try when it got anywhere (BestTry::scored: a step passed, or a case or assertion counted) and otherwise continues in place from the last try's tree; gap-586507's clean-to-base start is gone (the lead's decision). Tests, red at 07:09 then green: tries_that_pass_the_same_steps_rank_by_the_failing_steps_passed_cases (one doctest step no try passes: try 1 131/132, try 2 77/149; red: try 3 started from 'bad'; green: from 'good', c.txt gone, prompt told), a_resumed_task_starts_from_its_furthest_try_in_a_failing_step (red: started clean and failed), a_resumed_task_that_got_nowhere_continues_in_place (renamed from ..._starts_clean; red 'missing', green 'X'), a_failing_steps_runner_says_how_far_it_got (doctest, Catch2, cargo over two binaries, pytest, ctest, none), a_try_is_behind_one_that_got_further; the gap-fefdf5/586507 tests and retry_feedback:: pass (15 of 15). Checks: roko-cli lib 3966 passed, 1 failed: the known load flake plan_check_publishes_start_heartbeat_and_finish (bug-e362a1) at load average 160; main's own binary at 3c25a5532 fails it the same way under this load (and passes it at lower load), and the change does not touch plan_verify; clippy -D warnings (roko-cli) clean; nightly rustfmt clean."
+++

## Problem

The best-try ratchet (gap-fefdf5) ranks tries by verify steps passed. When the first step bundles several checks and
no try passes it, every try ties at 0. "Ties go to the latest", so each try continues from the last one's tree, and a
near-pass is overwritten.

Round v7, Undertow phase 3, T1 the sequencer (roko e4643dafd, run `bdc4461e` and its resume `dfba45e8`). Step 1 is
`tools/spec.sh 3 sequencer 'P3.01*,P3.02*'`, two checks in one step:

| Try | Step 1's doctest result | Steps passed |
|---|---|---|
| T1#3 | 131 of 132 assertions, 1 of 2 cases | 0 |
| T1#4 | 131 of 132 | 0 |
| T1#5 | 77 of 149 | 0 |
| T1#6 | 0 of 2 | 0 |
| T1#7 | 0 of 2 | 0 |
| T1#8 | 86 of 149 | 0 |

All six tied at 0 steps, so try 4's 131-of-132 tree was never restored. A resume on a build with gap-586507 would
start clean, since no try passed a step, and lose it too.

## Where

- `crates/roko-cli/src/graph_task_dispatch/best_try.rs` (module docs): "When a try passes fewer steps than the task's
  best try, the best try's files are put back… A try that passes as many steps or more continues in place, as before
  (ties go to the latest), and so does a try that passed nothing with no better try before it." The kept best try
  carries its `passed_steps`.
- `crates/roko-cli/src/graph_task_dispatch/verification.rs`: the step results, which have the step's output, the
  doctest or ctest summary lines.

## Plan

Rank tries within a step too. When two tries pass the same steps, compare the first failing step's own progress:
doctest's `test cases: … passed` and `assertions: … passed`, ctest's `N% tests passed`, cargo's `test result: … N
passed`. Restore the better tree when the new try regresses there, and keep that progress beside the best try so a
resume can restore it.

The lead's decisions (2026-10-21), on roko-ladder's proposal:
- Order: steps passed first, then assertions, then test cases, in the same first failing step, from each runner's
  own summary lines (cargo, pytest and ctest count only tests, so tests decide there). Ties go to the latest.
- On resume: start from the best try that got anywhere (a step passed, or a case or assertion counted). Otherwise
  continue in place from the last try's tree, as a retry in one process does. gap-586507's clean-to-base start is
  dropped: it is what would have thrown phase 3's near-done sequencer away.

## Done when

Two tries that pass the same verify steps are ranked by the first failing step's passed test cases or assertions,
and a try that regresses there leaves the next try to start from the better tree. Checked by the test in
`[[verify]]`.
