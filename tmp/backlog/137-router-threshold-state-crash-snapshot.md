# 137 — Router + Threshold State in Crash Snapshot

**Priority**: P1 — A crash mid-run resets cascade router observations and adaptive threshold updates accumulated during that run, forcing the next run to start from a less-informed state than the crashed run had reached.
**Size**: XS (2-4 hours)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/types.rs`
**Depends on**: #135 (adaptive gate thresholds in runner-v2 must be wired before they can be snapshotted)
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §E-1 (suggested 121)

---

## Background

`RunStateSnapshot` is the crash-safe state written to `.roko/state/run-state.json` on each runner tick. It records task completion status, current plan, and enough context to resume without repeating completed work.

Two important runtime states are absent from the snapshot:
1. **CascadeRouter state**: The router accumulates observations (model-rung pass/fail pairs) during a run. These improve routing decisions mid-run. On crash, all in-run observations are lost.
2. **AdaptiveThresholds state**: After #135 wires threshold updates, the thresholds change during a run based on gate results. On crash, the in-run threshold evolution is lost.

Both of these are small serializable structs. Adding them to `RunStateSnapshot` and restoring them on resume takes approximately 30 minutes of coding plus the serialization derives.

## Current State

- `RunStateSnapshot` in `crates/roko-cli/src/runner/types.rs` — does not include `CascadeRouter` or `AdaptiveThresholds` state.
- `CascadeRouter` — serializable (derives `Serialize`/`Deserialize`).
- `AdaptiveThresholds` — serializable after #135 adds the derives (if not already present).
- Resume path in `runner/resume.rs` — reads `RunStateSnapshot` and restores task state; does not restore router or threshold state.

## Implementation Plan

1. **Add fields to `RunStateSnapshot`**:
   ```rust
   pub struct RunStateSnapshot {
       // existing fields ...
       pub cascade_router_state: Option<SerializedRouterState>,
       pub adaptive_thresholds: Option<SerializedThresholds>,
   }
   ```

2. **Serialize router state on each tick**: In `event_loop.rs`, when writing the snapshot:
   ```rust
   snapshot.cascade_router_state = Some(cascade_router.to_serialized());
   snapshot.adaptive_thresholds = Some(adaptive_thresholds.to_serialized());
   ```

3. **Restore on resume**: In `runner/resume.rs`, after reading the snapshot:
   ```rust
   if let Some(router_state) = snapshot.cascade_router_state {
       cascade_router.restore_from(router_state);
   }
   if let Some(thresholds) = snapshot.adaptive_thresholds {
       adaptive_thresholds.restore_from(thresholds);
   }
   ```

4. **Handle missing fields**: When resuming from an older snapshot that lacks these fields, use `Option::None` as the default and start with fresh state (same as today's behaviour). This ensures backward compatibility.

5. **Verify serialization roundtrip**: Add a unit test that serializes `CascadeRouter` state and `AdaptiveThresholds`, writes them to `RunStateSnapshot`, and verifies they restore correctly.

## Acceptance Criteria

1. After adding the fields, `RunStateSnapshot` serializes and deserializes without error.
2. On crash and resume, the cascade router retains observations from before the crash.
3. On crash and resume, adaptive thresholds retain updates from before the crash.
4. Resuming from an old snapshot (without these fields) works without error.
5. Unit test for serialization roundtrip passes.

## Verification Checklist

- [ ] Run a plan with 5 tasks; crash after task 3; resume; verify the router state from tasks 1-3 is used for task 4's dispatch.
- [ ] Unit test: `RunStateSnapshot` with `cascade_router_state = Some(...)` serializes and deserializes to identical state.
- [ ] Remove `cascade_router_state` from a snapshot JSON file; verify resume works without panic.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/types.rs` | Add `cascade_router_state` and `adaptive_thresholds` to `RunStateSnapshot` |
| `crates/roko-cli/src/runner/event_loop.rs` | Populate new fields when writing snapshot |
| `crates/roko-cli/src/runner/resume.rs` | Restore router and threshold state from snapshot |
