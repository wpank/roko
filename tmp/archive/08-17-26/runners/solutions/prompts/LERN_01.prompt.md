# LERN_01: Wire FeedbackService to `roko chat` Session Setup

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-01`](../ISSUE-TRACKER.md#lern-01)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.1
- Priority: **P0**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ChatAgentSession` (at `chat_session.rs:307`) is the interactive REPL session. It manages a `ClaudeCliAgent`, sends turns via `send_turn()` / `send_turn_api()` / `send_turn_streaming()`, and receives `TurnResult` (at line 260) with `input_tokens`, `output_tokens`, `cost_usd`, `duration`. It has no `FeedbackService` field.

`FeedbackService::from_roko_dir_with_episodes()` (at `feedback_service.rs:140`) creates a service with an `EpisodeLogger` attached. It accepts a `.roko` path and auto-creates the `learn/` subdirectory. The `FeedbackEvent::ModelCall` variant (at `roko-core/src/foundation.rs:200-225`) has `run_id`, `model`, `role`, `input_tokens`, `output_tokens`, `cost_usd`, `latency_ms`, `success` fields.

`chat_session.rs` currently imports nothing from `roko_learn`.

## Exact Changes

1. Add a `feedback: Option<Arc<FeedbackService>>` field to `ChatAgentSession` struct at line 307.
2. In `ChatAgentSession::new()` (line 341), resolve the `.roko` directory from the workdir (same pattern as `dispatch_v2.rs:62`). Create `FeedbackService::from_roko_dir_with_episodes(&roko_dir)` and store as `Some(Arc::new(svc))`.
3. Generate a `run_id: String` (UUID) per chat session, store on the struct. This groups all turns in one session.
4. Verify that `roko-learn` is in `roko-cli/Cargo.toml` dependencies (it is -- used by `run.rs` already).
5. No behavior change to chat flow yet -- just the service is created and held.

## Write Scope

- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-cli/Cargo.toml`

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

- [ ] `ChatAgentSession` struct has `feedback` field
- [ ] `FeedbackService` is created in `new()` without panicking

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `ChatAgentSession` struct has `feedback` field
- `FeedbackService` is created in `new()` without panicking
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
