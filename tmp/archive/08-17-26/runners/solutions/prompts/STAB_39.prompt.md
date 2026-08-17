# STAB_39: Add `thinking_tokens` to UsageObservation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-39`](../ISSUE-TRACKER.md#stab-39)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.39
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_39 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`UsageObservation` tracks input/output/cache tokens but not thinking/reasoning tokens.
Models with thinking (Claude extended thinking, OpenAI o3/o4-mini) produce internal tokens
that cost money but are invisible.

## Exact Changes

1. Add `thinking_tokens: Option<u64>` to `UsageObservation`.
2. Update Claude CLI stream parser to extract reasoning token counts.
3. Update OpenAI-compat parser for `reasoning_tokens` field.
4. Update `CostTable` for thinking-specific pricing.
5. Surface in usage reports.

## Write Scope

- `crates/roko-agent/src/usage.rs`
- `crates/roko-agent/src/provider/`

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

- [ ] Run with `--effort high` -- episode shows non-zero `thinking_tokens`
- [ ] Cost accounting includes thinking token costs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_39 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run with `--effort high` -- episode shows non-zero `thinking_tokens`
- Cost accounting includes thinking token costs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_39 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
