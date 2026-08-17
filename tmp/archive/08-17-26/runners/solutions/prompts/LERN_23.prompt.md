# LERN_23: Wire Cross-Session Cost Aggregation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-23`](../ISSUE-TRACKER.md#lern-23)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.23
- Priority: **P3**
- Effort: 3 hours
- Depends on: `LERN_08` (source 7.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`CostsDb` (at `costs_db.rs:472`) has rich querying (`by_model`, `by_provider`, `by_role`, `by_plan`, `summary_by_model`) but no time-bounded aggregation. `BudgetGuardrail` needs daily totals. `roko learn efficiency` should show daily/weekly/monthly breakdowns.

## Exact Changes

1. Add `aggregate_since(since: DateTime<Utc>) -> CostSummary` to `CostsDb` that filters `records` by timestamp and calls `CostSummary::from_records()`.
2. Add `aggregate_range(from: DateTime<Utc>, to: DateTime<Utc>) -> CostSummary`.
3. In `commands/learn.rs` under the `Efficiency` arm, add cost breakdown output:
   - Today: `costs_db.aggregate_since(today_midnight)`
   - This week: `costs_db.aggregate_since(week_start)`
   - This month: `costs_db.aggregate_since(month_start)`
   - By model (all time): `costs_db.summary_by_model()`
4. Initialize `BudgetGuardrail.day_spent` from `aggregate_since(today_midnight)` (connects to Task 7.8).

## Write Scope

- `crates/roko-learn/src/costs_db.rs`
- `crates/roko-cli/src/commands/learn.rs`

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

- [ ] `roko learn efficiency` shows daily/weekly/monthly cost breakdowns
- [ ] Aggregation matches sum of individual records
- [ ] Budget guardrail initializes with correct daily spend

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko learn efficiency` shows daily/weekly/monthly cost breakdowns
- Aggregation matches sum of individual records
- Budget guardrail initializes with correct daily spend
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
