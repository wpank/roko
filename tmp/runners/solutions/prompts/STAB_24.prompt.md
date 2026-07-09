# STAB_24: Wire runner v2 episode logging

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-24`](../ISSUE-TRACKER.md#stab-24)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.24
- Priority: **P1**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Runner v2 does not write episodes on task completion. Episodes are the primary learning
signal consumed by `PromptAssemblyService`, error similarity matching, and `roko learn`.

## Exact Changes

1. On task completion in `event_loop.rs`, construct an `Episode`:
   ```rust
   let episode = Episode {
       task_id: task_id.clone(),
       model: model_name.clone(),
       success: gate_passed,
       input_tokens, output_tokens,
       gate_verdicts: verdicts.clone(),
       duration_ms,
       timestamp: Utc::now(),
       // ...
   };
   ```
2. Write via `EpisodeSink` or `LearningRuntime::record_completed_run()`.
3. Include gate results, token counts, cost, and timing.
4. Flush immediately after write.

## Design Guidance

Use `LearningRuntime::record_completed_run()` as the single writer (consistent with Task 1.08).

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

- [ ] `roko plan run` on a 3-task plan produces 3 new entries in `.roko/learn/episodes.jsonl`
- [ ] Each entry has model, success, tokens, gate verdicts

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run` on a 3-task plan produces 3 new entries in `.roko/learn/episodes.jsonl`
- Each entry has model, success, tokens, gate verdicts
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
