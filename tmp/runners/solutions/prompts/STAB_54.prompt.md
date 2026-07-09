# STAB_54: Make tool loop max iterations configurable

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-54`](../ISSUE-TRACKER.md#stab-54)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.54
- Priority: **P2**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_54 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Cerebras uses 50 iterations, OpenAI-compat uses 30. Not configurable.

## Exact Changes

1. Add `max_tool_iterations` to `ModelProfile` or `ProviderConfig`.
2. Read from config in each adapter.
3. Default: 30 for API, 50 for Cerebras.

## Write Scope

- `crates/roko-agent/src/provider/cerebras.rs`
- `crates/roko-agent/src/provider/openai_compat.rs`

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

- [ ] `max_tool_iterations = 10` in config limits agent to 10 iterations

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_54 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `max_tool_iterations = 10` in config limits agent to 10 iterations
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_54 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
