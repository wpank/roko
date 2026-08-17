# CONF_15: Wire Streaming Events to TUI in `chat_inline.rs`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-15`](../ISSUE-TRACKER.md#conf-15)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.15
- Priority: **P2**
- Effort: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The chat inline handler discards streaming events. Grep for
`while let Some(_event) = event_rx.recv().await` at `chat_inline.rs` returned no
results (the pattern may have been partially fixed), but the audit (AP-7) documents
that streaming events are drained without rendering. The TUI shows a spinner until
the entire response is complete.

## Exact Changes

1. Verify current state of event handling in chat_inline.rs.
2. If events are still discarded: replace `_event` with actual event mapping to the
   inline renderer or `DashboardEvent`.
3. For `ClaudeStreamEvent::Assistant`: render text tokens incrementally in the viewport.
4. For `ClaudeStreamEvent::Tool`: show tool call name and progress indicator.
5. For `ClaudeStreamEvent::Result`: finalize the response display.

## Write Scope

- `crates/roko-cli/src/chat_inline.rs`

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

- [ ] Running `roko chat` and sending a message shows tokens appearing incrementally,

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Running `roko chat` and sending a message shows tokens appearing incrementally,
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
