# ACPM_31: File Change Notifications to Editor

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-31`](../ISSUE-TRACKER.md#acpm-31)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.31
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_31 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`FileChangeNotification` and `FileChangeType` already exist in `crates/roko-acp/src/types.rs`. The runner's `detect_file_changes()` function (line 103) already parses `git diff --name-status`. These just need to be emitted as ACP session updates.

## Exact Changes

1. After each agent completes in the pipeline runner, call `detect_file_changes()`.
2. For each changed file, emit `CognitiveEvent::ToolCallComplete` with the file change as content, or add a new `SessionUpdate::FileChange` variant if not already present.
3. Batch notifications to avoid flooding: cap at 50 per agent completion.
4. Include the change type (Created, Modified, Deleted, Renamed) in each notification.

## Write Scope

- `crates/roko-acp/src/runner.rs`

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

- [ ] ACP client receives file change notifications after agent edits
- [ ] Notifications include correct change type
- [ ] Large changesets are capped at 50 notifications

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_31 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- ACP client receives file change notifications after agent edits
- Notifications include correct change type
- Large changesets are capped at 50 notifications
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_31 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
