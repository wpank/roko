# CONF_05: Complete `[budget]` Schema and Wire `BudgetGuardrail`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-05`](../ISSUE-TRACKER.md#conf-05)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.5
- Priority: **P1**
- Effort: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Two separate issues:

1. `BudgetConfig` at `crates/roko-core/src/config/budget.rs` has only 3 fields:
   `max_plan_usd` (default 25.0), `max_turn_usd` (default 3.0), `prompt_token_budget`
   (default 10000). Missing: per-task, per-session, per-day, warn threshold,
   route-to-cheaper threshold.

2. `BudgetGuardrail` at `crates/roko-learn/src/budget.rs` implements 3-scope limits
   with 5 graduated actions (Ok, Warn, RouteToCheaper, BlockNewSessions, Block).
   It exists in two crates (`roko-learn/src/budget.rs:8` and
   `roko-agent/src/task_runner.rs:196`). The roko-learn version is never instantiated
   in any live runner path. The roko-agent version is used only inside `TaskRunner`
   (which is itself only used in orchestrate.rs).

   Runner v2 has basic budget checks at `event_loop.rs:349` (per-turn) and
   `event_loop.rs:1764` (per-plan), but these are simple threshold comparisons,
   not the graduated guardrail.

## Exact Changes

1. Add fields to `BudgetConfig`:
   - `max_cost_per_task: Option<f64>` (default: None)
   - `max_cost_per_session: Option<f64>` (default: None)
   - `max_cost_per_day: Option<f64>` (default: None)
   - `warn_threshold: f64` (default: 0.8)
   - `route_to_cheaper_threshold: f64` (default: 0.9)
2. Instantiate `BudgetGuardrail` in `RunnerConfig::from_roko_config()`, populating from
   the extended `BudgetConfig`.
3. In event loop, before each dispatch, call `guardrail.check()`.
   - On `Warn`: log warning with current spend.
   - On `RouteToCheaper`: override model selection to cheapest available.
   - On `Block`: return error with clear message.
4. Replace the inline budget checks at `event_loop.rs:349` and `event_loop.rs:1764`
   with `guardrail.check()` calls.

## Write Scope

- `crates/roko-core/src/config/budget.rs`
- `crates/roko-learn/src/budget.rs`
- `crates/roko-cli/src/runner/types.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko config show` displays the budget section with all fields.
- [ ] Setting `budget.max_plan_usd = 0.01` causes a plan run to hit budget warning or block.
- [ ] `BudgetGuardrail.check()` is called at least once per task dispatch.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko config show` displays the budget section with all fields.
- Setting `budget.max_plan_usd = 0.01` causes a plan run to hit budget warning or block.
- `BudgetGuardrail.check()` is called at least once per task dispatch.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
