# DISP_32: Emit Episode Records from One-Shot Paths

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-32`](../ISSUE-TRACKER.md#disp-32)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.32
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_32 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The runner v2 path (`crates/roko-cli/src/runner/event_loop.rs`) writes episodes to `.roko/episodes.jsonl` and efficiency events to `.roko/learn/efficiency.jsonl`. But the one-shot paths (`roko <prompt>` via `dispatch_v2.rs`, `roko chat` via `chat_inline.rs`) write neither.

`ModelCallService` records `FeedbackEvent::ModelCall` via `FeedbackSink` (wired in `dispatch_v2.rs:88-92`). But `FeedbackService` writes to `.roko/learn/feedback.jsonl`, not to the episode log. The episode format expected by the learning subsystem is different.

## Exact Changes

1. In `dispatch_v2.rs`, after `ModelCallService::call()` returns, construct and append an episode record:
   ```rust
   let episode = Episode {
       timestamp: Utc::now(),
       run_id: format!("dispatch-v2:{}", uuid::Uuid::new_v4()),
       model: response.model.clone(),
       role: "inline".to_string(),
       input_tokens: response.usage.input_tokens,
       output_tokens: response.usage.output_tokens,
       cost_usd: response.usage.cost_usd,
       latency_ms: response.latency_ms,
       success: true,
       entry_point: "roko_inline".to_string(),
   };
   persist::append_jsonl(&episodes_path, &episode)?;
   ```
2. Similarly in `chat_inline.rs`, emit an episode after each turn
3. Use the same episode format as `runner/event_loop.rs` for consistency
4. Emit efficiency events alongside episodes (wall time, token efficiency)

## Design Guidance

Episodes are the canonical learning signal. Without them, the CascadeRouter, prompt experiments, and efficiency analysis have no data from interactive use. The format must match what `roko-learn` expects -- check `roko_learn::episode_logger::Episode` for the canonical shape.

## Write Scope

- `crates/roko-cli/src/dispatch_v2.rs`
- `crates/roko-cli/src/chat_inline.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `cargo run -p roko-cli -- run "echo hello"` produces an entry in `.roko/episodes.jsonl`
- [ ] `roko chat` produces episodes for each turn
- [ ] Episode format matches `Episode` struct serialization

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_32 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko chat` produces episodes for each turn
- Episode format matches `Episode` struct serialization
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_32 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
