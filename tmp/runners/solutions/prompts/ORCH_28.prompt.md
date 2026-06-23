# ORCH_28: Update Runner v2 to Use WorkflowEngine for Task Dispatch

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-28`](../ISSUE-TRACKER.md#orch-28)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.28
- Priority: **P1**
- Effort: 8 hours
- Depends on: `ORCH_01` (source 2.1), `ORCH_05` (source 2.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Runner v2's event loop at `crates/roko-cli/src/runner/event_loop.rs` (3,035 LOC) has its own dispatch path: it constructs a `Dispatcher`, resolves agent runtimes via `resolve_agent_runtime()`, and dispatches agents via `spawn_agent_result_bridge()`. This is separate from EffectDriver's `spawn_agent()`.

Once WorkflowEngine supports parallel task dispatch (Tasks 2.1-2.5), Runner v2 should delegate to WorkflowEngine for agent dispatch rather than maintaining its own dispatch path. This eliminates the AP-4DISP anti-pattern (four dispatch implementations).

Runner v2 adds significant operational features not in WorkflowEngine:
- Line-by-line streaming output parsing (`agent_stream.rs`)
- Real-time TUI updates via `StateHub`
- Episode and efficiency event recording
- Dream consolidation on plan completion

These need to be preserved by wiring them into WorkflowEngine's event system.

## Exact Changes

1. Identify the dispatch call sites in `event_loop.rs` (search for `spawn_agent`, `resolve_agent_runtime`, `Dispatcher`).
2. Replace direct agent dispatch with calls to `EffectDriver::spawn_agent()` or `spawn_agent_in_worktree()`.
3. Wire Runner v2's `StateHub` updates into WorkflowEngine's `RuntimeEvent` emissions.
4. Preserve streaming output parsing by implementing the `ModelCaller` trait with streaming support.
5. Update `max_concurrent_tasks` in `ExecutorConfig` construction to use the configured value instead of hardcoded `1`.
6. Preserve episode/efficiency event recording by implementing the `EpisodeRecorder` trait.
7. Keep dream consolidation trigger by subscribing to WorkflowEngine's completion events.

## Design Guidance

This is the highest-risk task because Runner v2 is the active CLI execution path. The migration should be incremental: start by having Runner v2 use EffectDriver for dispatch while keeping its own event loop. Full migration to WorkflowEngine's `run_plan()` is a later step. Test each step with `roko plan run <dir>` on a real plan.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/runner/mod.rs`

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

- [ ] `roko plan run <dir>` uses EffectDriver for agent dispatch
- [ ] TUI updates still work (StateHub receives events)
- [ ] Episode recording still produces `.roko/episodes.jsonl` entries
- [ ] Streaming output parsing still works
- [ ] `max_concurrent_tasks` respects config (not hardcoded to 1)
- [ ] All existing Runner v2 functionality preserved

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run <dir>` uses EffectDriver for agent dispatch
- TUI updates still work (StateHub receives events)
- Episode recording still produces `.roko/episodes.jsonl` entries
- Streaming output parsing still works
- `max_concurrent_tasks` respects config (not hardcoded to 1)
- All existing Runner v2 functionality preserved
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
