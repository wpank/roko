# 144 — Daimon Affect Delta Persistence and Episode Metadata

**Priority**: P2 — Daimon affect state is loaded and passed to the CascadeRouter but affect changes triggered by task/gate outcomes are not persisted back; the affect feedback loop is open and affect state cannot be correlated with learning outcomes.
**Size**: S (1 day)
**Crates**: `crates/roko-daimon/src/`, `crates/roko-cli/src/runner/event_loop.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §F-5 (suggested 128), `tmp/backlog/_mori-old-gaps.md` MO-24

---

## Background

The daimon (affect engine) models the system's current "emotional" state: frustration builds after repeated failures, relief follows a gate pass after a long struggle, curiosity peaks when novel patterns appear. These states modulate the cascade router: a frustrated state routes to more conservative models; a curious state routes to more exploratory ones.

Current wiring: `DaimonState` is loaded from `.roko/daimon/affect.json` at runner startup and passed into the cascade router for the first task dispatch. After that:
1. Affect deltas triggered by task/gate outcomes are NOT applied back to `DaimonState`.
2. The updated affect state is NOT persisted to `.roko/daimon/affect.json`.
3. Affect state at the time of each agent dispatch is NOT included in episode metadata.

This means affect influence is only felt on the very first task of each run. All subsequent routing decisions use the same initial affect state, regardless of what happened during the run.

## Current State

- `crates/roko-daimon/src/` — `DaimonState` with load/save and affect update methods.
- `crates/roko-cli/src/runner/event_loop.rs` — loads `DaimonState` at startup; passes to cascade router in `dispatch_agent_with()`.
- No call to `daimon_state.update()` after task/gate events.
- No call to `daimon_state.save()` during or after runner execution.
- Episode metadata struct — no `affect_state_at_dispatch` field.

## Implementation Plan

1. **Update affect state on task outcomes**: In the event handler:
   - `TaskCompleted` (gate pass): call `daimon_state.apply_success_delta(cost_usd, elapsed_secs)`.
   - `TaskFailed` (gate fail): call `daimon_state.apply_failure_delta(failure_kind, retry_count)`.
   - `GateCompleted { passed: true }` after a long struggle: call `daimon_state.apply_relief_delta()`.

2. **Persist after each update**: After each `daimon_state.apply_*` call, call `daimon_state.save(&affect_path)`. Use an async write with error-logging on failure (not fatal).

3. **Add affect state to episode metadata**: In the `EpisodeLogger` write path, add:
   ```rust
   pub struct EpisodeRecord {
       // existing fields ...
       pub affect_at_dispatch: Option<SerializedAffectState>,
   }
   ```
   Capture `daimon_state.snapshot()` at the moment of agent dispatch and store it.

4. **Add affect state to efficiency events**: Similarly, add `affect_at_dispatch` to `AgentEfficiencyEvent` for correlation analysis.

5. **Verify cascade router uses updated state**: Confirm that the cascade router receives the updated `DaimonState` on each dispatch, not just the initial loaded state. If the runner passes the initial state by value, change to passing a reference that is updated in place.

6. **Add `[learning.daimon]` config**: Allow `affect_persistence = false` to disable persistence for testing environments where a clean affect state is required.

## Acceptance Criteria

1. After a gate pass, affect state changes (e.g., frustration decreases) and the change persists to `.roko/daimon/affect.json`.
2. After a gate fail, affect state changes (e.g., frustration increases) and persists.
3. Episode records include `affect_at_dispatch` with the affect state at the time of dispatch.
4. The second task dispatch uses the affect state updated by the first task's outcome, not the initial loaded state.
5. `roko knowledge dream run` or any learning consumer can correlate episode outcomes with affect state.

## Verification Checklist

- [ ] Run a task that fails the gate; check `.roko/daimon/affect.json` for an increased frustration value.
- [ ] Run a task that passes the gate after a failure; check affect for a decreased frustration value.
- [ ] Inspect `.roko/episodes.jsonl`; verify entries have `affect_at_dispatch` field.
- [ ] Run two tasks; verify the second task's dispatch uses the affect state updated by the first task.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Call `daimon_state.apply_*` on task/gate outcomes; persist after each update |
| `crates/roko-daimon/src/` | Verify `apply_success_delta`, `apply_failure_delta`, `snapshot()`, `save()` exist |
| `crates/roko-learn/src/` | Add `affect_at_dispatch` to `EpisodeRecord` and `AgentEfficiencyEvent` |
