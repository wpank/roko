# LERN_22: Wire Forensic Replay CLI Command

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-22`](../ISSUE-TRACKER.md#lern-22)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.22
- Priority: **P3**
- Effort: 3 hours
- Depends on: `LERN_07` (source 7.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ForensicReplay` (at `forensic_replay.rs:50`) reconstructs decision context from episodes: `from_episodes(task_id, all_episodes)` (line 75), `summary()` (line 216), `turn_count()`, `failed_gate_count()`. `replay()` async function (line 250) provides the full replay.

There is no CLI command to invoke forensic replay. `roko learn` currently has: All, Route, Experiments, Efficiency, Episodes, Tune.

## Exact Changes

1. Add `Replay { episode_id: String, workdir: Option<PathBuf> }` variant to the `LearnCmd` enum in `commands/learn.rs`.
2. In `dispatch_learn()`, add the `LearnCmd::Replay` match arm:
   - Load episodes from `.roko/learn/episodes.jsonl`
   - Call `ForensicReplay::from_episodes(&episode_id, &episodes)`
   - Display: model selected, gate verdicts, cost, duration, turn count
   - If CascadeRouter state is loadable, show "with current router state, this task would use {model}" comparison
3. Register the new subcommand in clap arg parsing.

## Write Scope

- `crates/roko-cli/src/commands/learn.rs`
- `crates/roko-cli/src/commands/mod.rs`

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

- [ ] Run a task, then `roko learn replay <episode-id>` shows decision context
- [ ] Counterfactual model selection displayed when router state exists
- [ ] Missing episode ID produces a clear error message

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run a task, then `roko learn replay <episode-id>` shows decision context
- Counterfactual model selection displayed when router state exists
- Missing episode ID produces a clear error message
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
