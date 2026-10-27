+++
id = "bug-f6d1a3"
kind = "bug"
title = "After a crash a resumed delivery re-merges onto a new batch branch and reruns the plan check, failed resumes leave empty batch branches, set-aside attempt trees are never removed, and show plans counts no passed task"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-26
updated = 2026-10-27
last_verified = 2026-10-26
source = "round-v7 roko-recovery kill matrix (tmp/hermes-roko-v7/findings/roko-recovery.md, tmp/hermes-roko-v7/roko-recovery/runs/roko-main-4d22b94e8), the lead's ask"
discovered_from = "agent:roko-recovery (round v7, 4d22b94e8)"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs", "crates/roko-cli/src/orchestrator/worktree/acceptance.rs", "crates/roko-cli/src/commands/show.rs"]
links = { depends_on = [], blocks = [], related = ["gap-52df4c", "bug-d63b33", "gap-36f3fb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_resumed_delivery_continues_the_dead_runs_batch' crates/roko-cli/src && cargo test -p roko-cli --lib a_resumed_delivery_continues_the_dead_runs_batch"
+++

## Problem

After a crash, roko leaves branches and checkouts behind, and a resume of a delivery starts over on a new branch.
Seen with the $0 kill matrix in round v7 (`tmp/hermes-roko-v7/roko-recovery/`, 4d22b94e8):

1. **A resume after a crash during delivery doesn't continue it.**
   - Kill points e (the plan check running in the regression checkout) and e' (the batch branch's `update-ref` done,
     roko not yet recording it).
   - The resumed run mints a new batch, `roko/batch/<timestamp>-<uuid>`. It merges onto it, runs the plan check
     again, which for a real project is a full build and test, and moves it.
   - The dead run's `roko/batch/<id>` stays: empty at e, and already holding the delivered merge at e'. Two batch
     branches then carry the same work, and the person must pick one.
   - The delivery receipt (`roko.delivery@1`) is keyed to the old batch, so "resume can continue at the first
     unproved step" (`graph_execution/delivery.rs`) never applies after a crash.
2. **Each failed resume leaves an empty batch branch.** One appears per `roko run plans/<p> --resume-plan` that exits
   2 on an unresolved reservation. The plan runner removes a batch branch only when "the run left no work".
3. **Set-aside orphan trees are never removed.**
   - A run that starts over keeps the dead attempt's checkout as `.roko/worktrees/attempt-<id>-<run8>` (or
     `-graphrec`), with its edits, and branch `roko/attempt/attempt-<id>-<run8>`.
   - Nothing removes them, and gap-52df4c's `worktree_max_age_secs` is read by nothing.
4. **`roko show plans` counts tasks from `tasks.toml`.** After a crash it says `tasks 0/2 done` although the
   checkpoint has both passed.

## Why it matters

Goal core, polish. These leftovers multiply with every crash and every retry of a resume. The duplicate batch
branches confuse the one step a person must do by hand, merging the batch. The re-run plan check costs minutes.

## Where

- `crates/roko-cli/src/graph_execution/delivery.rs` (`reconcile`, `advance_delivery`) and the batch id the plan
  runner mints (`graph_execution/batch.rs`, `plan_runner.rs`).
- `crates/roko-cli/src/graph_execution/plan_runner.rs`, the empty-batch removal ("the run left no work; removed its
  batch branch").
- `crates/roko-cli/src/orchestrator/worktree/acceptance.rs`, which sets a kept checkout aside as `…-<run8>`.
- `crates/roko-cli/src/commands/show.rs` (plans).

## Plan

1. A resumed run reuses the batch branch recorded in the checkpoint's `roko.batch@1`, and continues its receipt. At
   e' `git_move_target` then answers "already has". When it can't reuse it, it deletes the dead run's batch branch
   once the new delivery succeeds.
2. A run that fails before any dispatch deletes the batch branch it created.
3. Remove set-aside checkouts older than `worktree_max_age_secs` when they hold no unsaved work. With gap-36f3fb's
   patch, they never do.
4. `show plans` reads the checkpoint's passed tasks.

## Done when

- After a kill at e or e', a resume leaves exactly one batch branch with the work, and runs no second plan check at
  e'.
- A failed resume leaves no new branch.
- Test `a_resumed_delivery_continues_the_dead_runs_batch` passes.

## Progress

**2026-10-27, part 4 is done** (round v7 roko-recovery). `roko show plans` counts the tasks the checkpoint committed
a pass for and says the plan was interrupted. bug-693bb3's overlay does the counting, and bug-df0e79 reads the state.
In the kill matrix (`tmp/hermes-roko-v7/roko-recovery/runs/fix-df0e79-454815ee6`), e-delivery and e-ref now read
`interrupted | tasks 2/2 done`. Parts 1 to 3 are open.

## Notes

- Related:
  - gap-52df4c: the age key.
  - bug-d63b33: a dead serve's hub files; CLI runs leave them too.
  - gap-36f3fb: the dead attempt's edits.
  - The roko-recovery p1.
