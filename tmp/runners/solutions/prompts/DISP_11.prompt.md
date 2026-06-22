# DISP_11: Budget Graceful Degradation -- Cheaper Model Fallback

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-11`](../ISSUE-TRACKER.md#disp-11)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.11
- Priority: **P2**
- Effort: 4 hours
- Depends on: `DISP_10` (source 3.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ModelCallService` has `fallback_models: Vec<String>` (line 86) populated by `configured_fallback_models()` from workspace config. When budget is near exhaustion, instead of hard-failing, the service should attempt a cheaper model.

The `BudgetCell` at line 951 tracks cumulative cost and has a `check()` method. It needs a `remaining()` or `fraction_used()` accessor.

## Exact Changes

1. Add `remaining_usd(&self) -> Option<f64>` and `fraction_used(&self) -> Option<f64>` to `BudgetCell`
2. In `ModelCallService::call()`, before dispatch, check budget proximity:
   ```rust
   if let Some(fraction) = self.budget.fraction_used() {
       if fraction > 0.90 {
           // Find cheapest model from fallback list
           if let Some(cheaper) = self.cheapest_fallback_model(&effective_model) {
               tracing::warn!(
                   budget_used_pct = fraction * 100.0,
                   from = %effective_model, to = %cheaper,
                   "budget >90% used, downgrading model"
               );
               effective_model = cheaper;
           }
       }
   }
   ```
3. Add `cheapest_fallback_model(&self, current: &str) -> Option<String>` that uses `CostTable` to find the lowest-cost model from `fallback_models`
4. Emit a `RuntimeEvent::BudgetWarning` when degradation occurs
5. When budget is 100% exhausted, check if `config.budget.on_exceeded` is `"downgrade"` (try cheaper), `"warn"` (proceed but warn), or `"fail"` (hard error). Default to `"fail"`.

## Design Guidance

Graceful degradation should be transparent but not silent. Always log when a model switch happens due to budget pressure. The user should see "budget 92% used, switching from claude-opus to claude-haiku" in stderr/logs. Never silently degrade without notification.

## Write Scope

- `crates/roko-agent/src/model_call_service.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] A unit test demonstrates model downgrade when budget fraction > 0.90
- [ ] Budget exhaustion produces a clear error message (not a panic)
- [ ] `RuntimeEvent::BudgetWarning` is emitted on degradation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A unit test demonstrates model downgrade when budget fraction > 0.90
- Budget exhaustion produces a clear error message (not a panic)
- `RuntimeEvent::BudgetWarning` is emitted on degradation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
