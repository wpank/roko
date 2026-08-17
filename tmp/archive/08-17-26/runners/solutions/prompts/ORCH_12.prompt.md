# ORCH_12: Implement EpisodeRecorder for WorkflowEngine

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-12`](../ISSUE-TRACKER.md#orch-12)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.12
- Priority: **P1**
- Effort: 5 hours
- Depends on: `ORCH_10` (source 2.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`EpisodeLogger` exists at `crates/roko-learn/src/episode_logger.rs` and records agent turns + gate results to `.roko/episodes.jsonl`. orchestrate.rs calls it throughout the dispatch/gate loop. Runner v2 also records episodes via its event loop.

WorkflowEngine's `FeedbackSink` trait records `FeedbackEvent::ModelCall` and `FeedbackEvent::GateResult` but these are generic feedback events, not structured episodes. The episode format includes `Episode { plan_id, task_id, agent_role, model, turns, gates, outcome, ... }`.

This task wraps `EpisodeLogger` in an `EpisodeRecorder` trait implementation and wires it into EffectDriver.

## Exact Changes

1. Create a `LearnEpisodeRecorder` adapter that wraps `EpisodeLogger` and implements `EpisodeRecorder`.
2. `record_turn()` creates an entry in the current episode's turns list.
3. `record_gate()` adds a gate verdict to the current episode.
4. `finalize()` writes the completed episode to `.roko/episodes.jsonl`.
5. Wire into EffectDriver: after each `spawn_agent()` call, call `episode_recorder.record_turn()`. After each `run_gates()` call, call `episode_recorder.record_gate()`. At workflow completion, call `finalize()`.

## Design Guidance

The adapter should be thread-safe (multiple concurrent agents recording turns). Use a `DashMap` or `tokio::sync::Mutex<HashMap>` keyed by run_id to track in-flight episodes.

## Write Scope

- `crates/roko-learn/src/episode_logger.rs`
- `crates/roko-runtime/src/effect_driver.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `LearnEpisodeRecorder` implements `EpisodeRecorder` trait
- [ ] Episodes are written to `.roko/episodes.jsonl` on workflow completion
- [ ] Each episode contains turns (agent calls) and gates (gate results)
- [ ] Concurrent episodes (from parallel tasks) do not interleave

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `LearnEpisodeRecorder` implements `EpisodeRecorder` trait
- Episodes are written to `.roko/episodes.jsonl` on workflow completion
- Each episode contains turns (agent calls) and gates (gate results)
- Concurrent episodes (from parallel tasks) do not interleave
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
