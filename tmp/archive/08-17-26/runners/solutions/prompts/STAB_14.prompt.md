# STAB_14: Forward streaming events to chat TUI

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-14`](../ISSUE-TRACKER.md#stab-14)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.14
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The audit mentioned `while let Some(_event) = event_rx.recv().await {}` draining events.
Grep for this exact pattern returns zero matches, suggesting the code may have been
refactored since the audit. The streaming infrastructure may already work.

## Exact Changes

1. Search `chat_inline.rs` for the event receive loop. Look for patterns like:
   - `while let Some(event) = rx.recv().await`
   - `event_rx.recv()`
   - Any channel receiver that processes agent events
2. If events are still being drained without processing:
   - Map `AgentStreamEvent::Text(text)` -> append to ratatui viewport buffer
   - Map `AgentStreamEvent::ToolCall { name, args }` -> show tool name in status bar
   - Map `AgentStreamEvent::Complete { usage }` -> show cost/token stats
3. If the event processing is already working:
   - Verify that streaming text appears character-by-character during response
   - Mark this task as resolved with a note about the audit being stale
4. Ensure the viewport auto-scrolls as new text arrives.
5. Test with a real agent call to confirm streaming works.

## Design Guidance

The ratatui viewport should use a ring buffer for streaming text to avoid unbounded memory
growth. A reasonable cap is 64KB of visible text, with older content scrollable. Tool call
display should be ephemeral (shown for 2 seconds in the status bar, then hidden).

## Write Scope

- `crates/roko-cli/src/chat_inline.rs`

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

- [ ] `roko chat` displays streaming text progressively during agent response
- [ ] Tool calls are visible during execution (name at minimum)
- [ ] Cost/token stats appear after response completes
- [ ] No spinner-only period followed by text dump

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko chat` displays streaming text progressively during agent response
- Tool calls are visible during execution (name at minimum)
- Cost/token stats appear after response completes
- No spinner-only period followed by text dump
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
