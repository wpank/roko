# PERF_04: LearningRuntime Single-Open

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-04`](../ISSUE-TRACKER.md#perf-04)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.4
- Priority: **??**
- Effort: ?
- Depends on: `PERF_03` (source 10.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Thread `LearningRuntime` through to `append_episode_log()` instead of
re-opening it. Saves ~70ms per run (3 file reads + JSON parse + distillation
spawn).

## Exact Changes

1. In the main dispatch path, open `LearningRuntime::open_under()` once
2. Change `append_episode_log()` signature to accept `lr: &mut LearningRuntime`
3. Remove the `LearningRuntime::open_under()` call inside `append_episode_log()`
   at line 2663-2670
4. Thread the same instance from the dispatch caller (around line 1301) to
   `append_episode_log()`
5. Call `lr.flush()` / drop at run end to ensure persistence

## Write Scope

- `crates/roko-cli/src/run.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/10-PERFORMANCE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `RUST_LOG=roko_learn=debug roko run "echo hello"` shows exactly ONE "opening
- [ ] `.roko/episodes.jsonl` still receives entries
- [ ] `.roko/learn/cascade-router.json` still updates

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `RUST_LOG=roko_learn=debug roko run "echo hello"` shows exactly ONE "opening
- `.roko/episodes.jsonl` still receives entries
- `.roko/learn/cascade-router.json` still updates
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
