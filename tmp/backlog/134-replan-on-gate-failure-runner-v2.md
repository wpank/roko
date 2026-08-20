# 134 — Replan-on-Gate-Failure in Runner-v2

**Priority**: P1 — Without replan, runner-v2 can only retry with backoff or mark a task fatal; Mori-level self-correction requires generating a structurally different plan when gate failures indicate the original approach is wrong, not just failing the same approach again.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/types.rs`
**Depends on**: Backlog #14 (Plan Mutation Protocol — the `PlanMutation` enum provides the vocabulary)
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §D-2 (suggested 119), `tmp/backlog/_mori-old-gaps.md` MO-22

---

## Background

Runner-v2 handles gate failures with a `RetryAction` enum that supports `Retry` (retry with backoff) and `MarkFatal` (stop after N retries). This covers the simple case where an agent made a mistake that can be corrected by another attempt with the same approach.

Mori also had `RetryAction::Replan`: when gate failures suggest the task decomposition is wrong (not just the implementation), a replan generates a revised task structure. For example, if three consecutive compile gate failures suggest the task split was incorrect (two tasks need to be a single atomic task), a replan merges them. This is structural self-correction, not just retrying.

The `NeedsReplan` failure kind already exists in the codebase but produces no replan record and no DAG mutation. The `PlanMutation` enum (from backlog #14) provides the vocabulary for expressing plan changes. This item wires them together into a live replan path in runner-v2.

Legacy `orchestrate.rs` had this: a replan ledger tracking what replans were attempted, a strategy selector choosing which mutation type to try, and a DAG mutation that modified task structure. Runner-v2 needs a functional equivalent.

## Current State

- `crates/roko-cli/src/runner/types.rs` — `RetryAction` enum has `Retry` and `MarkFatal`; `NeedsReplan` variant may exist but produces no action.
- `crates/roko-cli/src/runner/event_loop.rs` — gate failure handler dispatches `RetryAction`; no replan path.
- Backlog #14 (Plan Mutation Protocol) — `PlanMutation` enum specifies how to express plan changes.
- `orchestrate.rs` — replan ledger exists as reference (frozen after #132).

## Implementation Plan

1. **Add `RetryAction::Replan` variant**:
   ```rust
   Replan {
       strategy: ReplanStrategy,  // MergeWithNext, SplitTask, InsertPreconditionTask, ChangeApproach
       reason: String,            // from gate failure context
       attempt: u32,
   }
   ```

2. **Replan decision logic**: In the gate failure handler, after N consecutive `Retry` attempts (configurable, default 2), escalate to `Replan` if the failure mode suggests structural issues:
   - Same error on every retry → `ChangeApproach`
   - Conflicting changes with adjacent task → `MergeWithNext`
   - Missing context that prior task would provide → `InsertPreconditionTask`

3. **Replan ledger**: Write to `.roko/state/replan-ledger.json` on each replan attempt:
   ```json
   {"task_id": "...", "plan_id": "...", "strategy": "ChangeApproach", "attempt": 1, "gate_failure": "...", "timestamp": "..."}
   ```
   Persist and restore on crash/resume.

4. **Replan generation**: Call `build_gate_failure_plan_revision(task, gate_failure_context, strategy)` (which already exists in runner-v2 from the GAPS.md description) to generate a revised task definition.

5. **DAG mutation**: Apply the `PlanMutation` to the `TaskDag` in memory using backlog #14's mutation protocol. For `MergeWithNext`, replace two task nodes with one combined node. For `InsertPreconditionTask`, add a new node before the failing task.

6. **Max replans cap**: Per-plan limit of 3 replans (configurable). After the cap, fall through to `MarkFatal`. Log cap-hit as a structured event.

7. **Deduplication**: Track replan attempts by strategy × task_id in the ledger. Do not attempt the same strategy for the same task twice.

## Acceptance Criteria

1. After 2 consecutive retry failures, `RetryAction::Replan` is chosen.
2. A replan record is written to `.roko/state/replan-ledger.json`.
3. The revised task replaces the original in the `TaskDag`.
4. The plan proceeds with the revised task (agent dispatch uses the new task definition).
5. After 3 replan attempts on the same task, `MarkFatal` is chosen.
6. The same strategy is not attempted twice for the same task.
7. Crash-resume preserves the replan ledger (replans are not repeated after resume).

## Verification Checklist

- [ ] Create a task designed to fail the same way on every attempt; verify `Replan` is triggered after 2 retries.
- [ ] Verify `replan-ledger.json` has a new entry after a replan.
- [ ] After replan, verify `TaskDag` is modified (e.g., task title or definition changed).
- [ ] After 3 replan failures, verify the task is marked `Fatal` and the runner continues.
- [ ] Crash and resume; verify the replan ledger is restored and replans are not repeated.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/types.rs` | Add `RetryAction::Replan` variant and `ReplanStrategy` enum |
| `crates/roko-cli/src/runner/event_loop.rs` | Wire replan decision logic, ledger writes, DAG mutation |
| `crates/roko-cli/src/runner/` | Replan ledger persistence module |
