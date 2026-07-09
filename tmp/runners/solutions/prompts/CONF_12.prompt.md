# CONF_12: Add FeedbackService to `roko chat` Path

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-12`](../ISSUE-TRACKER.md#conf-12)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.12
- Priority: **P2**
- Effort: Small
- Depends on: `CONF_10` (source 16.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko chat` records zero learning signals. No episodes, no routing observations, no
cost tracking. `FeedbackService` and `FeedbackEvent` are not referenced anywhere in
`chat_session.rs` (confirmed via grep). Chat is the most-used interactive entry point.

## Exact Changes

1. If 16.10 routes chat through `ServiceFactory::build()`, the `FeedbackService` is
   constructed automatically. Verify this is wired.
2. Emit `FeedbackEvent::WorkflowComplete` when the chat session ends (on `/quit` or
   Ctrl-D) so the session's total cost is recorded.
3. Emit per-turn cost observations for CascadeRouter learning.

## Write Scope

- `crates/roko-cli/src/chat_session.rs`
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

- [ ] After a 3-turn chat session, `.roko/learn/cascade-router.json` has 3 new observations.
- [ ] `.roko/learn/costs.jsonl` has cost records for the session.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After a 3-turn chat session, `.roko/learn/cascade-router.json` has 3 new observations.
- `.roko/learn/costs.jsonl` has cost records for the session.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
