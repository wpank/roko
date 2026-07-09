# PROM_15: Wire Conversation Compaction into Chat Loop

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-15`](../ISSUE-TRACKER.md#prom-15)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.15
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: After each assistant response, check if conversation history
should be compacted using `compact_history()`.

## Exact Changes

1. Import `roko_compose::compaction::{compact_history, CompactionPolicy, ChatMessage}`
2. Define a default policy:
   ```rust
   CompactionPolicy {
       trigger_threshold: 0.70,
       anchor_roles: vec!["system".into()],
       preserve_last_n_turns: 8,
       summary_budget_tokens: 128,
   }
   ```
3. Convert between `ChatAgentSession::api_history` format and `ChatMessage` format
4. After each assistant response, check `should_compact(&messages, &policy)`:
   - Estimate if compactable region > 70% of total context
5. If true, call `compact_history()`:
   - Use a dedicated Haiku call or the current chat agent as summarizer
   - Replace compacted messages with the summary message
6. Continue the session with compacted history

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Chat session with 30+ turns triggers compaction
- [ ] After compaction, system messages and recent 8 turns are preserved verbatim
- [ ] Gate results and tool outcomes from compacted region are carried forward
- [ ] Chat continues working normally after compaction

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Chat session with 30+ turns triggers compaction
- After compaction, system messages and recent 8 turns are preserved verbatim
- Gate results and tool outcomes from compacted region are carried forward
- Chat continues working normally after compaction
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
