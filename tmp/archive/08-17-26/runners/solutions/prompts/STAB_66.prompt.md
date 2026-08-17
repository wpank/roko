# STAB_66: Wire VerdictPublisher to all gate dispatch paths

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-66`](../ISSUE-TRACKER.md#stab-66)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.66
- Priority: **P2**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_66 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

VerdictPublisher is optional and rarely provided. Gate verdicts are not broadcast to TUI.

## Exact Changes

1. Provide VerdictPublisher in each gate dispatch path.
2. Wire to DashboardEvent emitter (TUI).
3. Wire to SSE channel (serve).

## Write Scope

- `crates/roko-gate/src/verdict_publisher.rs`

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

- [ ] During `roko plan run` with dashboard, gate progress appears in real time

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_66 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- During `roko plan run` with dashboard, gate progress appears in real time
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_66 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
