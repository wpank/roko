# CONF_25: Fix Dual Episode Writes in `roko run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-25`](../ISSUE-TRACKER.md#conf-25)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.25
- Priority: **P2**
- Effort: Small
- Depends on: `CONF_10` (source 16.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko run` writes episodes twice:
- Direct `append_episode_log()` call at `run.rs:1301`
- `LearningRuntime::record_completed_run()` at `run.rs:2680`

This produces duplicate records in different files or double entries in the same file.

## Exact Changes

1. Remove the direct `append_episode_log()` call at line 1301.
2. Let `LearningRuntime` (via `FeedbackService`) be the single episode writer.
3. Verify that `FeedbackService` writes to the canonical path.
4. If two paths differ (`.roko/episodes.jsonl` vs `.roko/learn/episodes.jsonl`),
   pick one canonical location and update all readers.

## Write Scope

- `crates/roko-cli/src/run.rs`

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

- [ ] Running `roko run "hello"` produces exactly 1 episode record, not 2.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Running `roko run "hello"` produces exactly 1 episode record, not 2.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
