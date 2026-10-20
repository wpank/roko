+++
id = "gap-fefdf5"
kind = "gap"
title = "A retry continues from the last try's tree even after it broke a step an earlier try passed, so retries compound a regression and lose the passing code (Undertow phase 2 run 82b7e069: T3 try 2 broke P2.10, tries 3-4 started from the broken tree, exhausted)"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-roko-ladder#best-try-ratchet"
discovered_from = "agent:roko-ladder (round v7, run 82b7e069 analysis)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/step_ratchet.rs", "crates/roko-cli/src/graph_task_dispatch/diff_snapshot.rs", "crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs"]
links = { depends_on = [], blocks = [], related = ["gap-6dba88", "bug-ba0c40", "gap-ff2c38"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_retry_after_a_regression_starts_from_the_best_try' crates/roko-cli/ && cargo test -p roko-cli --lib -- a_retry_after_a_regression_starts_from_the_best_try"
+++

## Problem

A retry continues from the tree the last try left, in the same checkout, even when that try broke a step an earlier
try passed. So retries compound a regression, and the passing state is lost.

Undertow phase 2 (run `82b7e069`): T3 (the output stage, 3 step groups) ran all four tries in one worktree
(`attempt-a05633dc…`; tries 2-4 record `workspace.reused = true`).
- **Try 1:** passed verify[0] (P2.09/10) and failed verify[1] (P2.11/12).
- **Try 2:** edited try 1's code and broke verify[0]. The P2.10 saturation THD was -147.6 dB against ≥ -26 dB: the
  clipper went linear.
- **Tries 3 and 4:** started from try 2's broken tree and failed verify[0] again. T3 exhausted, and T4 was blocked.
- **T2:** went 0/4, 2/4, 0/4 (verify[0] regressed), 2/4.

The step ratchet (gap-6dba88) saw each regression (`gate_regression` events) and wrote "Regression: verify[0:test]
passed on an earlier attempt and fails now" into the next prompt. It is advisory only, and nothing kept try 1's tree.
Fail-fast also skipped verify[1] on try 2, so nobody knows whether try 2 fixed P2.11/12 while breaking P2.10.

## Why it matters

The regressed tries cost $0.38 of the run's $0.93 (T3's tries 2-4, T2's try 3) and left the plan unfinished. Each try
is a new agent session, so the agent can't remember the passing code. Only roko can keep it.

## Where

- `crates/roko-cli/src/graph_task_dispatch/step_ratchet.rs`: each task's passed steps across tries, and the
  regression note.
- `crates/roko-cli/src/graph_task_dispatch/diff_snapshot.rs`: the tree snapshot each try already writes for the
  pre-verify screen.
- `crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs`: the retry prompt's feedback.
- `crates/roko-cli/src/graph_task_dispatch.rs`: the retry path and the verify run (fail-fast).

## Current state

- Retries share the task's checkout by design (`attempt_workspace.rs`: "a retry resumes the work its predecessor
  left").
- A task moves to a fresh checkout of the plan branch only on a plan-branch conflict, a tamper or scope rejection, or
  a plan-check hand-back.
- No config chooses the starting point.

## Plan

The lead decided on 2026-10-20 that it works as proposed:
1. **Score** each failed try by the verify steps it passed, and pin its tree snapshot under
   `refs/roko/tries/<plan>/<task>/<n>` until the task ends.
2. **Restore:** before the next try, when the last try scored below the best, put the best try's tracked files back
   in the checkout. Untracked files the best tree lacks are cleaned; ignored build trees stay. The checkout's base
   stays the plan branch. Otherwise continue in place, as now; ties go to the latest.
3. **Full step run:** when a try fails a step an earlier try passed, run its remaining steps instead of failing fast,
   so a trade (one step each way) shows. A trade keeps the latest.
4. **Feedback:** a short block says what passed, what broke and that roko restored the best try's files. It is
   followed by the regressing try's diffstat and its patch for the files it changed (bounded), so a deliberate
   redesign isn't lost.
5. **Nothing passed:** a try that passed nothing, with no better try before it, continues in place.

Pass the restore through a seam that avoids `attempt_workspace.rs` while bug-ba0c40 is open. If that's impossible,
ask the lead first.

## Done when

- A $0 test reproduces T3's shape: try 1 passes 2 of 3 steps, try 2 breaks a passing step, and the ratchet restores
  try 1's tree for try 3, with the feedback naming it.
- A try that passed nothing continues in place.
- Checked by the `[[verify]]` test.

## Notes

- Decided by the lead on 2026-10-20 as p1, to build after the start-unreachable rule (round v7).
- Risk: a restore only goes back to a verified-better tree, so the worst case is today's behaviour plus one restore.
  A dead end (a best tree the remaining steps can't pass) is answered by the two-failure climb and by the shown
  patch.
