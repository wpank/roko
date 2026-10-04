+++
id = "gap-a4fbf8"
kind = "gap"
title = "The model-ladder retry-budget floor overrides the adaptive gate-history suggestion instead of composing with it (QA7)"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch", "roko-gate"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK95 gap-d4a1c1)"
discovered_from = "gap-d4a1c1 (whitepaper re-read, QA7); flagged as uncovered in bug-d74b6b's Notes"
anchors = ["crates/roko-cli/src/graph_task_dispatch/retry_budget.rs::for_task", "crates/roko-cli/src/graph_task_dispatch/retry_budget.rs::TaskRetryBudgets", "crates/roko-gate/src/adaptive_threshold.rs::suggested_max_retries"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-460230", "bug-d74b6b", "find-4b4344"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn adaptive_suggestion_survives_the_ladder_floor' crates/roko-cli/src/graph_task_dispatch/retry_budget.rs && cargo test -p roko-cli --lib adaptive_suggestion_survives_the_ladder_floor"
+++

## Problem

A task's retry budget can come from two signals: the adaptive gate-history suggestion (per-rung EMA pass rate,
`crates/roko-gate/src/adaptive_threshold.rs::suggested_max_retries`) and the model-ladder floor (enough retries
to climb the cheap-model ladder, `gap-460230`). `TaskRetryBudgets::for_task`
(`crates/roko-cli/src/graph_task_dispatch/retry_budget.rs:127-142`) combines them by wholesale replacement, not
by taking the larger of the two while keeping the adaptive source's information:

```rust
let budget = self.suggested(task);                      // adaptive (or default)
let pinned = task.model_hint.is_some() || task.hints.preferred_model.is_some();
if !pinned && budget.max_retries < self.ladder_min_retries {
    return RetryBudget { max_retries: self.ladder_min_retries, source: RetryBudgetSource::Ladder };
}
budget
```

Whenever the ladder's minimum (`ladder_min_retries`, derived from `[gates] adaptive_max_retries`, 5 by default —
see `gap-460230`'s plan step 4: "a task with no authored `max_retries` gets at least 2 × (K_max + 1) − 1 = 5
retries") is greater than or equal to what the gate-history rung suggested, the final `source` becomes `Ladder`
outright and the rung-specific reasoning (`RetryBudgetSource::Adaptive { rung, ema_pass_rate, observations }`) is
discarded, not merged. Since the ladder floor sits at the top of the default range, this happens for essentially
every unpinned task: the per-rung pass-rate signal computed by `suggested_max_retries` never reaches the final
budget. This is whitepaper tag QA7: "the ladder overrode the gate-history retry budget" — in the 2026-10-02 live
run (`a43288b5f`) the gate-history budget changed nothing because of this.

## Why it matters

Goal `learning`: adaptive gate thresholds are one of roko's cybernetic feedback loops (a rung that fails more
should get more retries, one that passes reliably should get fewer). As implemented, that loop is inert for any
task that also routes through the model ladder — which is the common case once the ladder is on — because the
ladder's flat floor silently wins. The mechanism exists and is tested in isolation, but the two signals were never
designed to compose, so the "measured trust" claim (gate history shapes retry budgets) does not hold in the
presence of the ladder. `docs/whitepaper/REVIEW.md:124` and `appendix-status-matrix.md:100` both flag QA7 as
PARTIAL and note no item exists for it; this item is that item. (Do not edit `docs/whitepaper/*` to close this —
that tree is being rewritten by another session; fix the Rust behavior only.)

Related:
- `gap-460230` (done): introduced `ladder_min_retries` and the override above, to fix a different problem (a cheap
  model burning its whole retry budget on one rung instead of climbing). Its fix is correct for that problem; this
  item is about the side effect on the adaptive signal.
- `bug-d74b6b` (done): fixed the *displayed* retry suggestion (dashboard/status/API) to match `[gates]` bounds,
  and explicitly left this behavior gap open in its own Notes: "Not covered: a ladder-routed task's raised floor
  (`TaskRetryBudgets::ladder_min_retries`)". This item is that follow-up.
- `find-4b4344` (done): wired nine other gate-learning closures onto the Graph path; unrelated to this override.

## Where

- `crates/roko-cli/src/graph_task_dispatch/retry_budget.rs::TaskRetryBudgets::for_task` (:127-142) — the override.
- `crates/roko-cli/src/graph_task_dispatch/retry_budget.rs::TaskRetryBudgets::suggested` (:145-174) — computes the
  adaptive suggestion that gets discarded.
- `crates/roko-gate/src/adaptive_threshold.rs::AdaptiveThresholds::suggested_max_retries` (:525-543) — the
  per-rung EMA-to-retries mapping whose output is thrown away once the ladder floor applies.

## Current state

Checked at HEAD (2026-10-02). The override is unconditional: `for_task` returns early with
`RetryBudgetSource::Ladder` and `self.ladder_min_retries` whenever the adaptive suggestion is lower, with no way
for the final budget to reflect both "this rung usually fails, give it more" and "this task needs N retries to
reach the top rung." The two tests near the bottom of `retry_budget.rs` (`budgets(...).with_ladder_min_retries(5)`
and the "generous" adaptive case) each exercise one signal at a time; none covers the case where the adaptive
suggestion is *above* the ladder floor while using `source: Adaptive`, or where a rung's pass rate is low enough
that it should dominate the floor instead of being replaced by it.

## Plan

1. Decide the composition rule: the simplest fix is `max_retries: budget.max_retries.max(self.ladder_min_retries)`
   while keeping `source` as whichever contributed the larger number (or a new `RetryBudgetSource::Combined`
   variant carrying both the rung and the floor, so logging and the dashboard can show both reasons). The minimal
   version is just taking the max value but preserving `Adaptive`'s fields when the adaptive suggestion is the one
   that ends up binding (i.e. already ≥ the floor) and only falling back to `Ladder` when the floor truly is
   larger — which is close to today's behavior for that one case, but the bug is that today's code *always*
   reports `Ladder` once the floor is reached or exceeded, even though the adaptive number might have been even
   higher.
2. Re-derive: `let max_retries = budget.max_retries.max(self.ladder_min_retries); let source = if budget.max_retries >= self.ladder_min_retries { budget.source } else { RetryBudgetSource::Ladder };` — this keeps the adaptive rung/EMA/observations visible in the source whenever the adaptive number is already enough to climb, and only reports `Ladder` when the floor genuinely raises the number. This alone fixes the headline symptom (adaptive data is discarded even when it would have given *more* retries than the floor) without inventing a new variant; note it does not fix the case where the floor raises a *lower* adaptive number — logging a dated note or a `Combined` variant is the fuller fix for that case, left as a follow-on if the simple fix isn't enough.
3. Add a test that a rung with a low EMA pass rate (suggesting, say, 5 retries) keeps `source: Adaptive` with its
   rung/EMA/observations even when `ladder_min_retries` is also 5, and a second test that a rung with a *higher*
   adaptive suggestion than the floor (e.g. adaptive suggests 5, floor is 3) is not silently relabeled `Ladder`.
4. Update `graph_task_dispatch/retry_budget.rs`'s module doc comment (the paragraph starting "While the model
   ladder routes tasks...") to describe the corrected composition.

## Done when

- A task whose gate-history rung suggests at least as many retries as the ladder floor requires keeps
  `RetryBudgetSource::Adaptive` (with its rung/EMA/observations) in `TaskRetryBudgets::for_task`'s result, instead
  of being unconditionally relabeled `Ladder`.
- The `[[verify]]` command passes.

## Notes

- Do not touch `docs/whitepaper/*` or `tmp/cybernetic-harness/paper/*` to resolve this item; `work/items/gap-d4a1c1-*.md`
  (which surfaced QA7 again on re-read) is held for an in-progress paper rewrite by another session — leave it alone.
- `gap-460230`'s own fix is not wrong; don't revert the ladder floor, only how it composes with the adaptive
  suggestion.
- 2026-10-04 (wave-17b follow-up, bug-35a738): related but distinct overlap, filed as
  `q-85792e`. `bug-35a738`'s facet 2 reports B2 (M1's `retry_delta` move) has no room to move on
  a ladder-routed task at default gates, because `adaptive_max_retries` defaults to 5
  (`roko-core/config/gates.rs`) and the ladder's own floor (`LADDER_MIN_RETRIES`,
  `graph_task_dispatch/ladder.rs`) is also 5 — the budget is already at B2's own ceiling before
  any delta applies. Fixing this item's attribution bug (the ladder relabeling `source` to
  `Ladder` even when the adaptive number was already correct) would not by itself give B2 any
  numeric room to move; `q-85792e` is a separate decision (raise the default, or amend decision
  8101's clamp for ladder-routed tasks) that should be made alongside, not instead of, this
  item's fix.
