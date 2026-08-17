# STAB_56: Wire dream consolidation trigger

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-56`](../ISSUE-TRACKER.md#stab-56)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.56
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_56 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Dream consolidation is built but has no runtime trigger. `DreamTriggerSink` writes events
that nothing reads. Knowledge consolidation only happens via manual `roko knowledge dream run`.

## Exact Changes

1. Add dream loop to `roko serve` background tasks.
2. Configure via `roko.toml` (`[dreams]` section): cron interval or plan-completion trigger.
3. Alternatively: add post-run hook in `roko plan run`.
4. Report dream status in `roko status`.

## Write Scope

- `crates/roko-dreams/src/runner.rs`
- `crates/roko-serve/src/lib.rs`

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

- [ ] After plan run, dream cycle runs automatically (if configured)
- [ ] `roko status` shows last dream timestamp

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_56 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After plan run, dream cycle runs automatically (if configured)
- `roko status` shows last dream timestamp
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_56 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
