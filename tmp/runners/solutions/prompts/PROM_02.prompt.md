# PROM_02: Add `tier_scaled_budget()` to budget.rs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-02`](../ISSUE-TRACKER.md#prom-02)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.2
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Add a function that proportionally scales a `PromptBudget` to fit
within a `ContextTier`'s token budget. Uses the existing `total_budget()`
helper at line 128.

## Exact Changes

1. Import `ContextTier` from `crate::context_provider`
2. Add `pub fn tier_scaled_budget(base: PromptBudget, tier: ContextTier) -> PromptBudget`
3. Compute `base_total = total_budget(&base)` (existing function, line 128)
4. Compute `tier_total = tier.default_token_budget() * 4` (tokens-to-chars heuristic)
5. If `tier_total >= base_total`: return `base` unchanged
6. Compute `scale = tier_total as f64 / base_total as f64`
7. Scale each field: `(field as f64 * scale) as usize`
8. Add unit tests

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `tier_scaled_budget(budget_for(AgentRole::Implementer), ContextTier::Surgical)` produces total <= 16000 chars (~4K tokens)
- [ ] `tier_scaled_budget(budget_for(AgentRole::Implementer), ContextTier::Full)` produces total <= 96000 chars (~24K tokens)
- [ ] No field goes negative

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `tier_scaled_budget(budget_for(AgentRole::Implementer), ContextTier::Surgical)` produces total <= 16000 chars (~4K tokens)
- `tier_scaled_budget(budget_for(AgentRole::Implementer), ContextTier::Full)` produces total <= 96000 chars (~24K tokens)
- No field goes negative
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
