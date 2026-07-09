# STAB_12: Wire feedback recording to `roko chat`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-12`](../ISSUE-TRACKER.md#stab-12)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.12
- Priority: **P1**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`chat_session.rs` makes model calls but records no episodes, no routing observations, no
cost tracking. Grep for `FeedbackService` in `chat_session.rs` returns zero matches.
Every chat session is a lost learning opportunity.

## Exact Changes

1. In chat session setup, instantiate `FeedbackService`:
   ```rust
   let feedback = FeedbackService::from_roko_dir_with_episodes(&roko_dir)?;
   ```
2. After each model response, emit a feedback event:
   ```rust
   feedback.record_model_call(ModelCallRecord {
       model: model_name.clone(),
       input_tokens, output_tokens,
       latency_ms, success: true,
       provider: provider_name.clone(),
   })?;
   ```
3. On session end (`/quit` or Ctrl-D), emit a session completion event:
   ```rust
   feedback.record_session_complete(SessionRecord {
       session_id, total_turns, total_cost,
       total_input_tokens, total_output_tokens,
   })?;
   ```
4. Optionally: attach CascadeRouter to observe model performance from chat.

## Design Guidance

Keep feedback recording lightweight -- it should not add perceptible latency to chat. Use
async write-behind if needed. The feedback sink should be the same one used by `roko run`
for consistency.

## Write Scope

- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-learn/src/feedback_service.rs`

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

- [ ] Start `roko chat`, send one message, quit
- [ ] `.roko/learn/efficiency.jsonl` has a new entry with non-zero tokens
- [ ] `.roko/episodes.jsonl` (or learn equivalent) has a new entry

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Start `roko chat`, send one message, quit
- `.roko/learn/efficiency.jsonl` has a new entry with non-zero tokens
- `.roko/episodes.jsonl` (or learn equivalent) has a new entry
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
