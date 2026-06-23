# STAB_17: Wire BudgetGuardrail to live paths

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-17`](../ISSUE-TRACKER.md#stab-17)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.17
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`BudgetGuardrail` in `budget.rs` implements 3-scope budget limits (per-task, per-session,
per-day) with 5 graduated actions (Ok, Warn, RouteToCheaper, BlockNewSessions, Block). It
is only referenced from `orchestrate.rs` (behind legacy feature flag) and `task_runner.rs`
in `roko-agent`. Zero live callers in runner v2 or `roko run`.

## Exact Changes

1. Load budget config from `roko.toml` (`[budget]` section).
2. Instantiate `BudgetGuardrail` at session start in:
   - `roko run`: before model dispatch in `run.rs`
   - `roko chat`: in session setup in `chat_session.rs`
   - `roko plan run`: before event loop in `event_loop.rs`
3. Before each model dispatch, check budget:
   ```rust
   match guardrail.check(estimated_cost) {
       BudgetAction::Ok => { /* proceed */ }
       BudgetAction::Warn(msg) => { tracing::warn!("{}", msg); /* proceed */ }
       BudgetAction::RouteToCheaper => { /* switch to fallback model */ }
       BudgetAction::Block(msg) => { return Err(anyhow!("Budget exceeded: {}", msg)); }
       // ...
   }
   ```
4. After each model call, update the guardrail with actual cost:
   ```rust
   guardrail.record_spend(actual_cost);
   ```
5. Set sensible defaults: per-turn $0.50, per-session $10.00, per-plan $100.00.

## Design Guidance

The guardrail should be optional -- users who don't configure `[budget]` should not be
blocked. The `Block` action should produce a clear error message showing cumulative spend
and the configured limit. Consider adding a `--budget-override` CLI flag for one-off
increases.

## Write Scope

- `crates/roko-learn/src/budget.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Set `budget.max_session_usd = 0.01` in roko.toml
- [ ] `roko run "write a long essay"` stops with budget exceeded message
- [ ] Without budget config, no blocking occurs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Set `budget.max_session_usd = 0.01` in roko.toml
- `roko run "write a long essay"` stops with budget exceeded message
- Without budget config, no blocking occurs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
