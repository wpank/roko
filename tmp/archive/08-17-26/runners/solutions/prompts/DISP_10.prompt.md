# DISP_10: Wire BudgetConfig from roko.toml into ModelCallService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-10`](../ISSUE-TRACKER.md#disp-10)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.10
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`BudgetConfig` already exists at `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/budget.rs` with `max_plan_usd: f32` (default $25), `max_turn_usd: f32` (default $3), and `prompt_token_budget: usize` (default 10,000). The `[budget]` section is already in the roko.toml schema.

`ModelCallService::new()` at line 126 creates `BudgetCell::new(None)` -- unlimited by default. The `with_cost_budget(max_cost_usd)` builder exists at line 233 but is never called from `dispatch_v2.rs`.

## Exact Changes

1. Add `per_session_usd: f32` field to `BudgetConfig` with default $10.00. This is the session-level budget for `roko run` and `roko chat`. The existing `max_plan_usd` covers plan execution and `max_turn_usd` covers per-turn limits.
2. In `dispatch_v2.rs`, after loading config (line 63-80), apply budget:
   ```rust
   let budget = config.budget.per_session_usd as f64;
   service = service.with_cost_budget(budget);
   ```
3. In `run.rs`, apply `config.budget.max_plan_usd` when building `ModelCallService` for plan execution
4. Add a `with_turn_budget(max_turn_usd: f64)` builder to `ModelCallService` that sets a per-call (not cumulative) cap. Implement by adding a `turn_budget` field and checking it in `call()` before dispatch.
5. Wire `config.budget.max_turn_usd` through all entry points

## Design Guidance

The cumulative session budget (`per_session_usd`) tracks total spend across all calls in a session. The per-turn budget (`max_turn_usd`) caps a single call. Both should be configurable via roko.toml. When a budget is exceeded, fail with a clear error message including the budget amount, the current spend, and how to increase the limit.

## Write Scope

- `crates/roko-core/src/config/budget.rs`
- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-cli/src/dispatch_v2.rs`

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

- [ ] `BudgetConfig` has `per_session_usd` field
- [ ] `ModelCallService` receives a non-None budget from `dispatch_v2.rs`
- [ ] A test that sets `per_session_usd = 0.001` and dispatches a real model call gets a budget-exceeded error

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `BudgetConfig` has `per_session_usd` field
- `ModelCallService` receives a non-None budget from `dispatch_v2.rs`
- A test that sets `per_session_usd = 0.001` and dispatches a real model call gets a budget-exceeded error
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
