# RNNR_11: Build cumulative context section generator

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-11`](../ISSUE-TRACKER.md#rnnr-11)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.11
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Generate a "What Changed Before You" section that shows each task
what files were modified by prior tasks in the plan.

## Exact Changes

1. Define `CompletedTaskSummary`:
   ```rust
   pub struct CompletedTaskSummary {
       pub task_id: String,
       pub files_changed: Vec<(String, i32, i32)>,  // path, lines_added, lines_removed
       pub brief_description: String,
   }
   ```
2. Add `pub fn cumulative_context(completed: &[CompletedTaskSummary], token_budget: usize) -> String`
3. Format as markdown:
   ```
   ## What Changed Before You
   Files modified by prior tasks in this plan:
   - `src/gate/compile.rs` (+45 -12): Added run_compile_gate, modified gate_pipeline
   - `src/lib.rs` (+3 -0): Added `pub mod gate;`
   ```
4. When total tokens exceed `token_budget` (default 4000), truncate oldest
   task summaries first, keeping most recent changes visible
5. Use `TokenCounter` from the compose crate for byte counting

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Cumulative section generated from completed task data
- [ ] Token budget respected (never exceeds default)
- [ ] Oldest entries truncated first when budget exceeded
- [ ] Empty section returned when no prior tasks

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Cumulative section generated from completed task data
- Token budget respected (never exceeds default)
- Oldest entries truncated first when budget exceeded
- Empty section returned when no prior tasks
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
