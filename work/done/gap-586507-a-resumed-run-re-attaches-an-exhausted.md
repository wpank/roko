+++
id = "gap-586507"
kind = "gap"
title = "A resumed run re-attaches an exhausted task's last-try checkout and starts from that broken tree: the best-try ratchet (gap-fefdf5) keeps the best try in memory only, so a resume of run 82b7e069 would have put T2 and T3 back on their broken trees"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-20
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "b1f9301a0"
source = "round-v7-roko-ladder#ratchet-resume"
discovered_from = "agent:team-lead (the pilot's phase-2 review), 2026-10-20"
anchors = ["crates/roko-cli/src/graph_task_dispatch/best_try.rs", "crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs"]
links = { depends_on = [], blocks = [], related = ["gap-fefdf5", "bug-ba0c40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_resumed_task_starts_from_its_best_try' crates/roko-cli/ && cargo test -p roko-cli --lib -- best_try"

[closed]
at = 2026-10-21
at_ts = "2026-10-20T22:34:04Z"
commit = "b1f9301a0"
forced = false
evidence = "Tests in graph_task_dispatch/best_try.rs, with two dispatchers standing in for two processes of one checkpoint run (begin_plan_run + attach_retry_feedback on each, so the second re-attaches the kept checkout; $0, fake Claude CLI): a_resumed_task_starts_from_its_best_try (process A: try 1 passes 2 of 3 steps, try 2 breaks one; process B's first try sees a.txt = A and no c.txt, and passes) and a_resumed_task_that_passed_nothing_starts_clean (process A's try passes nothing; process B's first try finds the checkout clean). Both red before the change: the resumed try saw the last try's tree. With them: best_try, retry_feedback and step_ratchet tests 16 passed; roko-cli lib 3942 passed, 1 failed (a_cancel_stops_the_post_merge_regression_check, load-flaky, bug-3203c4); clippy -D warnings and nightly fmt clean; attempt_workspace.rs untouched."
+++

## Problem

The best-try ratchet (gap-fefdf5, e4643dafd) keeps each task's best try in memory only. A resumed run re-attaches
an exhausted task's kept checkout with its failed, uncommitted work, and its first try starts from that last try's
tree:
- Attempt checkout ids are `format_attempt_worktree_id(plan, task, generation)`, and the generation is
  per-process, starting at 0.
- `create_attempt_locked` re-attaches "a checkout of `id` that `run_id` made".

Resuming Undertow phase 2's run 82b7e069 would have put T2 and T3 back on their broken trees: T3's on try 4's 0/3,
not try 1's 1/3. Only a fresh run sets those checkouts aside (the lead's reading of the pilot's phase-2 review,
2026-10-20).

## Why it matters

A resume after an exhausted task is how a person retries it (`roko plan run --resume-plan`, `roko resume`). If the
resume starts from the last try's tree, the regression the ratchet guards against comes back in exactly that case.

## Where

- `crates/roko-cli/src/graph_task_dispatch/best_try.rs`: the ratchet, in memory per process.
- `crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs`: the per-plan `retry-feedback.json` beside the Graph
  checkpoint, which already keeps each task's feedback, attempt numbers and passed steps for a resumed run of the
  same checkpoint run.

## Current state

- The ratchet restores the best try only between tries of one process.
- The retry feedback and step history survive a resume, but the best try's tree and its passed count don't.

## Plan

1. Keep each task's best try (its attempt, its tree, the checkout it ran in and the steps it passed) in
   `retry-feedback.json`, beside the passed-step history. A task's pass drops it. A fresh run ignores it, since it
   is scoped to the checkpoint run.
2. On a task's first try in a process that inherits an earlier process's failed tries (pending feedback for it):
   - in the checkout the best try ran in, start from the best try's tree when that try passed any step;
   - otherwise clean the checkout to its own base (HEAD's tree, the plan branch it was made from);
   - never start from the last try's tree.
   The restore goes through the same scratch-index path, so ignored build trees stay.
3. Keep `attempt_workspace.rs` untouched while bug-ba0c40 is open.

## Done when

Tests with two dispatchers standing in for two processes of one run, sharing the re-attached checkout:
- process A runs T3's tries 1 (2 of 3 steps) and 2 (a regression), and process B's first try starts from try 1's
  tree;
- process A's try passes nothing, and process B's first try starts clean.

## Notes

Asked by the lead on 2026-10-20 as part of the ratchet (gap-fefdf5), which had already merged.
