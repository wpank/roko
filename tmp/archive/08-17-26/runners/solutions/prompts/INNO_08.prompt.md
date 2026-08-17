# INNO_08: Implement per-plan budget manager

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-08`](../ISSUE-TRACKER.md#inno-08)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.8
- Priority: **P0**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`BudgetGuardrail` at `crates/roko-learn/src/budget.rs` has task/session/day
granularity with `record_cost()` and `BudgetAction` (Ok/Warn/Block). But there
is no plan-level budget cap and no `--max-cost` CLI flag.

Research: Cost visibility is #1 developer pain. Cursor's pinned forum thread
has 117K views. Princeton HAL: 50x cost variation between agents at similar accuracy.

## Exact Changes

1. Create `crates/roko-learn/src/plan_budget.rs`.
2. Define `PlanBudgetManager` struct: `plan_budget_usd: f64`, `spent_usd: f64`,
   `remaining_tasks: usize`, `task_costs: HashMap<String, f64>`.
3. Implement `budget_for_task(task: &str, complexity_multiplier: f64) -> TaskBudget`:
   - `target_usd = (plan_budget_usd - spent_usd) / remaining_tasks * complexity_multiplier`
   - `hard_cap_usd = target_usd * 3.0`
   - `allow_escalation = spent_usd < plan_budget_usd * 0.7`
4. Implement `record_cost(task_id: &str, cost_usd: f64)` and
   `is_exceeded() -> bool`.
5. Implement `budget_pressure(&self) -> f64` returning
   `remaining_budget / remaining_tasks` (input to Task 11.7).
6. Implement serde for persistence: serialize to `.roko/state/plan-budget.json`
   so budget state survives `--resume`.
7. Add `--max-cost <USD>` flag to `roko plan run` and `roko run` in
   `crates/roko-cli/src/main.rs`.
8. Wire `PlanBudgetManager` into the runner event loop: before dispatching each
   task, call `is_exceeded()`. If true, halt with error message:
   "Budget exceeded: ${spent} spent of ${budget} budget."
9. Add `pub mod plan_budget;` to `crates/roko-learn/src/lib.rs`.

## Write Scope

- `crates/roko-learn/src/lib.rs`
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko plan run --max-cost 1.00` halts when cumulative cost reaches $1.00
- [ ] Error message shows: "Budget exceeded: $X.XX spent of $1.00 budget."
- [ ] Budget state persists across `--resume` runs
- [ ] Without `--max-cost`, no budget enforcement (backward compatible)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run --max-cost 1.00` halts when cumulative cost reaches $1.00
- Error message shows: "Budget exceeded: $X.XX spent of $1.00 budget."
- Budget state persists across `--resume` runs
- Without `--max-cost`, no budget enforcement (backward compatible)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
