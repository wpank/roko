# LERN_02: Emit ModelCall Events from `roko chat` Turns

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-02`](../ISSUE-TRACKER.md#lern-02)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.2
- Priority: **P0**
- Effort: 3 hours
- Depends on: `LERN_01` (source 7.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`TurnResult` (at `chat_session.rs:260`) contains `input_tokens: u64`, `output_tokens: u64`, `cost_usd: f64`, `duration: Duration`, and `cancelled: bool`. The model slug is available from `self.model` (set during `ChatAgentSession::new()`).

After each turn completes (in `send_turn()` at line 887, `send_turn_api()` at line 489, `send_turn_oneshot()` at line 957), a `TurnResult` is returned. This is the emission point.

`FeedbackService` implements `FeedbackSink` (async trait at `foundation.rs:250`). `record()` is async. Chat sessions are async.

## Exact Changes

1. After each successful `TurnResult` is produced, emit `FeedbackEvent::ModelCall` with:
   - `run_id`: `Some(self.session_run_id.clone())`
   - `model`: `Some(self.model.clone())`
   - `role`: `"chat".to_string()`
   - `input_tokens`: from `turn_result.input_tokens`
   - `output_tokens`: from `turn_result.output_tokens`
   - `cost_usd`: from `turn_result.cost_usd`
   - `latency_ms`: `turn_result.duration.as_millis() as u64`
   - `success`: `!turn_result.cancelled`
   - `prompt_section_ids`: `vec![]`
   - `knowledge_ids`: `vec![]`
2. Call `self.feedback.as_ref().unwrap().record(event).await` (or `let _ = ...` to avoid panicking on feedback errors).
3. Add the emission after all three send paths (`send_turn_api`, `send_turn_oneshot`, `send_turn_streaming`). Use a helper method `emit_model_call(&self, turn: &TurnResult)` to avoid duplication.

## Write Scope

- `crates/roko-cli/src/chat_session.rs`

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

- [ ] Run `roko chat`, send one message, exit
- [ ] `.roko/learn/efficiency.jsonl` contains a `model_call` record with `role: "chat"`, non-zero tokens

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run `roko chat`, send one message, exit
- `.roko/learn/efficiency.jsonl` contains a `model_call` record with `role: "chat"`, non-zero tokens
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
