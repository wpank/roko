# LERN_08: Wire Budget Enforcement to `roko run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-08`](../ISSUE-TRACKER.md#lern-08)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.8
- Priority: **P1**
- Effort: 4 hours
- Depends on: `LERN_05` (source 7.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`BudgetGuardrail` (at `budget.rs:8`) has 3 scope limits (`max_task_usd`, `max_session_usd`, `max_day_usd`) and returns `BudgetAction` (Ok, Warn, RouteToCheaper, BlockNewSessions, Block). It is never instantiated.

`roko.toml` supports budget config fields (see `roko-core/src/config/schema.rs` for `BudgetConfig`). `CostsDb` (at `costs_db.rs:472`) has `by_session()`, `by_model()`, `total_cost()` but no `aggregate_since()` for daily aggregation.

## Exact Changes

1. Add `aggregate_since(since: chrono::DateTime<Utc>) -> f64` to `CostsDb` that sums `cost_usd` for records with `timestamp >= since`.
2. In `roko run` initialization, load budget config from `Config` (check for `budget.max_task_usd`, `budget.max_session_usd`, `budget.max_day_usd` fields).
3. If any budget field is set, instantiate `BudgetGuardrail` with the configured limits.
4. Initialize `day_spent` from `CostsDb::aggregate_since(today_midnight)`.
5. Before each model dispatch, check `guardrail.record_cost(estimated_cost, "task")`:
   - `BudgetAction::Ok` or `BudgetAction::Warn`: proceed (log at WARN for Warn)
   - `BudgetAction::RouteToCheaper`: set a flag to bias CascadeRouter toward cheaper models
   - `BudgetAction::Block` or `BudgetAction::BlockNewSessions`: return error before dispatch
6. After dispatch, update guardrail with actual cost from `AgentResult`.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-learn/src/budget.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Set `budget.max_task_usd = 0.001` in roko.toml, run a task, verify block or downgrade
- [ ] Budget warnings logged at WARN level
- [ ] `aggregate_since()` returns correct daily total

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Set `budget.max_task_usd = 0.001` in roko.toml, run a task, verify block or downgrade
- Budget warnings logged at WARN level
- `aggregate_since()` returns correct daily total
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
