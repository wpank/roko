# LERN_03: Emit WorkflowComplete on Chat Session End

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-03`](../ISSUE-TRACKER.md#lern-03)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.3
- Priority: **P0**
- Effort: 2 hours
- Depends on: `LERN_02` (source 7.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Chat sessions end in multiple ways: user types `/exit`, Ctrl-C, or the REPL loop exhausts. The session struct tracks total cost and token counts across turns (or can accumulate them). `FeedbackEvent::WorkflowComplete` (at `foundation.rs:233-248`) has `event_type`, `run_id`, `model`, `success`, `total_input_tokens`, `total_output_tokens`, `total_cost_usd`, `total_latency_ms`, `gate_results`.

## Exact Changes

1. Add running accumulators to `ChatAgentSession`: `total_input_tokens: u64`, `total_output_tokens: u64`, `total_cost_usd: f64`, `total_latency_ms: u64`, `turn_count: u32`. Update after each turn.
2. On session end (wherever the REPL loop exits), emit `FeedbackEvent::WorkflowComplete` with:
   - `event_type`: `"chat_session"`
   - `run_id`: session UUID
   - `model`: last model used
   - `success`: true (session completed normally)
   - Accumulated totals
   - `gate_results`: empty vec (no gates in chat)
3. Call `feedback.flush()` (sync) or `feedback.flush_async().await` before the session drops.
4. Consider implementing this in a `Drop`-adjacent cleanup method since `FeedbackService::drop()` already flushes, but explicit is better.

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

- [ ] After a multi-turn chat session, `.roko/learn/efficiency.jsonl` shows one WorkflowComplete plus N ModelCall records
- [ ] Accumulated totals match sum of individual turn records

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After a multi-turn chat session, `.roko/learn/efficiency.jsonl` shows one WorkflowComplete plus N ModelCall records
- Accumulated totals match sum of individual turn records
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
