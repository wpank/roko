# STAB_08: Fix dual episode writes in `roko run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-08`](../ISSUE-TRACKER.md#stab-08)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.08
- Priority: **P0**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

In `run.rs`, at line 1301, there is a direct `append_episode_log()` call. Then at line 2680,
`runtime.record_completed_run(completed)` also writes episodes through `LearningRuntime`.
Both are behind `#[cfg(feature = "legacy-orchestrate")]`.

The dual write produces duplicate records in different files (`.roko/episodes.jsonl` at root
vs `.roko/learn/episodes.jsonl` under the learn directory).

## Exact Changes

1. Remove the direct `append_episode_log()` call at line 1301 (keep the `record_completed_run`
   call which goes through `LearningRuntime`).
2. Verify that `LearningRuntime::record_completed_run()` writes to the canonical
   `.roko/learn/episodes.jsonl` path.
3. Check all episode readers (`roko status`, `roko learn episodes`, TUI episodes tab) to
   ensure they read from the learn path, not the root path.
4. If backward compatibility is needed, add a one-time migration that moves entries from
   the root path to the learn path.
5. Since both calls are behind `#[cfg(feature = "legacy-orchestrate")]`, verify if the
   non-legacy V2 path also has dual writes. If not, this fix is only needed for the
   legacy feature flag.

## Design Guidance

Single writer principle: `LearningRuntime` should be the sole episode writer across all
paths. Any other episode write should go through it.

## Write Scope

- `crates/roko-cli/src/run.rs`

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

- [ ] `roko run "hello"` produces exactly one new episode entry (not two)
- [ ] Episode entry appears in `.roko/learn/episodes.jsonl` (canonical path)
- [ ] `roko learn episodes` reads from the correct path

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run "hello"` produces exactly one new episode entry (not two)
- Episode entry appears in `.roko/learn/episodes.jsonl` (canonical path)
- `roko learn episodes` reads from the correct path
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
