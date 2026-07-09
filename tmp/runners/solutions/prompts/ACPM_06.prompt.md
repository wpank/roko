# ACPM_06: Multi-Turn Context Carry-Forward

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-06`](../ISSUE-TRACKER.md#acpm-06)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.6
- Priority: **P1**
- Effort: 4 hours
- Depends on: `ACPM_04` (source 9.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`AcpSession` has `history: Vec<ConversationTurn>` (defined in session.rs) that tracks conversation but does not track which files the agent modified. Subsequent turns have no continuity about what was changed unless the user re-mentions files.

## Exact Changes

1. Add `pub touched_files: Vec<TouchedFile>` to `AcpSession`:
   ```rust
   pub struct TouchedFile {
       pub path: String,
       pub turn_index: usize,
       pub change_type: String, // "edited", "created", "deleted"
   }
   ```
2. After each prompt completes in `bridge_events.rs`, extract file paths from tool call updates (`ToolCallKind::Edit`, `Create`, `Delete`) and append to `touched_files`.
3. Deduplicate the list by path, cap at 20 files (remove oldest when over limit).
4. In the next prompt's context assembly (Task 9.4 integration), add touched files as `ContextItem { source_name: "touched_file", priority: 150, score: recency_score, evictable: true }` where `recency_score = 1.0 - (turns_ago * 0.15)`.
5. Score touched files by recency: most recently touched = highest score.

## Write Scope

- `crates/roko-acp/src/session.rs`
- `crates/roko-acp/src/bridge_events.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] ACP session: first prompt edits `src/lib.rs`, second prompt receives `src/lib.rs` content in context without @-mention
- [ ] Files from 3+ turns ago are evicted when budget is tight
- [ ] `touched_files` does not grow beyond 20 entries

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- ACP session: first prompt edits `src/lib.rs`, second prompt receives `src/lib.rs` content in context without @-mention
- Files from 3+ turns ago are evicted when budget is tight
- `touched_files` does not grow beyond 20 entries
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
