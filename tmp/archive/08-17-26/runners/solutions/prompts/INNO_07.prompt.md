# INNO_07: Wire cost-aware Pareto routing into CascadeRouter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-07`](../ISSUE-TRACKER.md#inno-07)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.7
- Priority: **P0**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

CascadeRouter at `crates/roko-learn/src/cascade_router.rs` has `route()`,
`route_with_knowledge()`, `route_with_cfactor()`, and
`route_with_knowledge_among()` methods. It already computes Pareto frontier via
`crates/roko-learn/src/pareto.rs` but does not use cost data for model selection.

Research: RouteLLM -- 85% cost cut on MT-Bench retaining 95% of GPT-4 quality.
FrugalGPT -- 98% cost reduction. Princeton HAL: 50x cost variation between
agents at similar accuracy.

## Exact Changes

1. In `CascadeRouter::route()`, retrieve the Pareto frontier of non-dominated
   models (quality vs cost).
2. Accept `budget_pressure: Option<f64>` parameter (computed as
   `remaining_budget / remaining_tasks` by caller).
3. Filter candidates to `expected_quality >= quality_floor` (configurable,
   default 0.7). Use existing `quality_judge.rs` or bandit observations.
4. Among viable candidates, sort by cost-per-token ascending when budget
   pressure is high (budget_pressure < 1.0), by quality descending when budget
   is unconstrained.
5. Return `CascadeModel` with the selected model. The existing `escalation`
   field on the return type can encode the fallback model.
6. Add `quality_floor: f64` to `CascadeRouterConfig` with default 0.7.

## Write Scope

- `crates/roko-learn/src/cascade_router.rs`

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

- [ ] With a tight budget ($0.10/task), the router selects Haiku or Cerebras over Opus/Sonnet
- [ ] With an unconstrained budget, the router selects the highest-quality model
- [ ] After 50+ observations, dominated models (high cost, low quality) are not selected
- [ ] Verify via `cascade-router.json` that Pareto routing data is persisted

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- With a tight budget ($0.10/task), the router selects Haiku or Cerebras over Opus/Sonnet
- With an unconstrained budget, the router selects the highest-quality model
- After 50+ observations, dominated models (high cost, low quality) are not selected
- Verify via `cascade-router.json` that Pareto routing data is persisted
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
