# LERN_06: Deduplicate Episode Writes in `roko run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-06`](../ISSUE-TRACKER.md#lern-06)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.6
- Priority: **P1**
- Effort: 2 hours
- Depends on: `LERN_05` (source 7.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`run.rs` writes episodes twice:
1. Direct call at line 1301: `append_episode_log(...)` -> `.roko/episodes.jsonl`
2. Via `LearningRuntime::record_completed_run()` at line 2680 -> `.roko/learn/episodes.jsonl`

The `append_episode_log` function is defined at line 2545 of `run.rs`. `LearningRuntime` internally calls `EpisodeLogger::append()` inside `record_completed_run()`.

## Exact Changes

1. Remove the direct `append_episode_log()` call at line 1301 and the function definition at line 2545.
2. Verify `LearningRuntime::record_completed_run()` writes all required episode fields that `append_episode_log` was writing (agent_id, role, model, tokens, gate verdicts, cost, HDC fingerprint).
3. If any fields are missing from `CompletedRunInput::from_episode()` (at `runtime_feedback.rs:863`), add them.
4. Update any code that reads from `.roko/episodes.jsonl` (root path) to read from `.roko/learn/episodes.jsonl` instead, or ensure `LearningRuntime` writes to the root path.
5. Remove dead imports related to the removed function.

## Write Scope

- `crates/roko-cli/src/run.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Run `roko run`, count episodes -- exactly one per execution
- [ ] Episode has all fields: gate verdicts, cost, tokens, model, HDC fingerprint
- [ ] No duplicate entries across episode log files

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run `roko run`, count episodes -- exactly one per execution
- Episode has all fields: gate verdicts, cost, tokens, model, HDC fingerprint
- No duplicate entries across episode log files
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
