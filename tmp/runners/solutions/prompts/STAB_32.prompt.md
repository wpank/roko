# STAB_32: Fix model showing "-" in TUI for runner v2

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-32`](../ISSUE-TRACKER.md#stab-32)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.32
- Priority: **P1**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_32 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Runner v2 passes empty string for model in TUI events. The dashboard shows "-" instead of
the model name.

## Exact Changes

1. When dispatching an agent, include the resolved model name in the dispatch event.
2. When the agent responds with usage, include model in the progress event.
3. Populate from the dispatch context (model selection result), not from agent output.

## Design Guidance

The model name should be set at dispatch time (before agent starts) and carried through
all events for that task. Do not rely on agent output parsing for the model name.

## Write Scope

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

- [ ] `roko plan run` with TUI shows model name (e.g., "claude-sonnet-4") instead of "-"

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_32 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run` with TUI shows model name (e.g., "claude-sonnet-4") instead of "-"
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_32 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
