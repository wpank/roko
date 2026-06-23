# STAB_67: Add gate budget tracking for LLM judge calls

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-67`](../ISSUE-TRACKER.md#stab-67)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.67
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_67 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

LLM judge gate calls have no cost tracking. Each call is an LLM API call but no episode or
cost is recorded.

## Exact Changes

1. Record episode per judge invocation.
2. Track cumulative gate cost separately.
3. Cap judge invocations per task (default: 3).
4. Include gate cost in run summary.

## Write Scope

- `crates/roko-gate/src/`

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

- [ ] LLM judge call produces cost entry
- [ ] Total cost includes judge costs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_67 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- LLM judge call produces cost entry
- Total cost includes judge costs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_67 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
