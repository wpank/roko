# DISP_31: Implement Cost-Optimized Batch Routing

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-31`](../ISSUE-TRACKER.md#disp-31)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.31
- Priority: **P3**
- Effort: 4 hours
- Depends on: `DISP_10` (source 3.10), `DISP_06` (source 3.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_31 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

For plan execution with many tasks, pre-computing an optimal routing plan within a budget constraint can reduce total cost while maintaining quality for critical tasks.

The `CostTable` has per-model pricing. The `CascadeRouter` has per-model quality estimates. Combined, they can produce a cost-quality Pareto-optimal assignment.

## Exact Changes

1. Add `pub fn plan_batch_routing(&self, tasks: &[BatchTask], budget_usd: f64) -> Vec<(String, String)>`:
   - `BatchTask` has `task_id: String`, `complexity: f64`, `required_quality: f64`
   - Returns `Vec<(task_id, model_slug)>` assignment
2. Sort tasks by complexity (descending)
3. For each task, find the cheapest model that meets the quality threshold
4. Track cumulative cost; if budget would be exceeded, downgrade remaining tasks
5. Return the assignment

## Design Guidance

This is a greedy assignment algorithm -- not truly optimal, but good enough. The VCG auction from orchestrate.rs (if extracted in Task 3.26) could provide a more sophisticated allocation, but the greedy approach is simpler and sufficient for the initial implementation.

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

- [ ] Unit test: 10 tasks with $5 budget assigns cheap models to low-complexity tasks
- [ ] Unit test: budget exceeded triggers model downgrade for remaining tasks
- [ ] Total assigned cost does not exceed budget

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_31 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: 10 tasks with $5 budget assigns cheap models to low-complexity tasks
- Unit test: budget exceeded triggers model downgrade for remaining tasks
- Total assigned cost does not exceed budget
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_31 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
