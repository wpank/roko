# Graph Engine Runner-v2 Parity

**Priority**: P2 — Graph works for dispatch but lacks lifecycle features
**Size**: XL (3-4w)
**Crates**: `crates/roko-graph/`, `crates/roko-cli/`

---

## Problem

The Graph engine (`crates/roko-graph/src/engine.rs`, ~2100 lines) provides bounded
topological-wave execution with real provider dispatch, cost tracking, snapshot-resume,
7 cognitive Cells, 5 Verify Cells, and the immune decision Graph. It is invoked via
`roko plan run --engine graph`.

However, it lacks seven Runner-v2 lifecycle features required for production parity.
Without these, Graph execution cannot gate, replan, isolate, merge, approve, fully
persist, or cleanly cancel — making it unsuitable as a drop-in replacement for Runner-v2.

---

## What already exists

| Feature | Runner-v2 | Graph | Gap severity |
|---|---|---|---|
| Gate pipeline (19 rungs) | `runner/gate_dispatch.rs` | None | **High** — no verification |
| Replan on gate failure | `build_gate_failure_plan_revision()` | None | **High** — no retry |
| Approval workflow | Channel-based human review | Rejects `--approval` at startup | **Medium** |
| Worktree isolation | `WorktreeManager` per-attempt | None (shared workspace) | **High** — no isolation |
| Merge queue lifecycle | Full CI wait + regression + publish | Enqueue only | **Medium** |
| State persistence | Atomic snapshots + run ledger | Activity replay only | **Medium** |
| Cancellation propagation | Process group teardown | Node-level only | **Low** |
| Per-turn cost halt | `turn_exceeds_budget()` | Reserve-only | **Low** |

---

## What to do

### Phase 1: Gates (highest value)

- **Step 1a.** After each Activity completes, call `gate_dispatch::run_gate_once()` with
  the task's gate configuration and the Activity output.
- **Step 1b.** Record `GateVerdict` in the Graph's per-node state and persist it in
  the Activity log.
- **Step 1c.** On gate failure, mark the node as `Failed` with the verdict. If the
  failure action is `NeedsReplan`, proceed to Phase 2.

### Phase 2: Replan on gate failure

- **Step 2a.** When a gate returns `GateFailureAction::NeedsReplan`, call
  `build_gate_failure_plan_revision()` to generate revised task steps.
- **Step 2b.** Update the Graph's in-memory node definition with the revised task.
- **Step 2c.** Re-execute the revised node (up to `gate_failure_replan_cap` attempts).
- **Step 2d.** Persist revision state in the Activity log for resume.

### Phase 3: Worktree isolation

- **Step 3a.** Before dispatching each Activity, acquire a worktree via `WorktreeManager`.
- **Step 3b.** Execute the Activity in the isolated worktree.
- **Step 3c.** On success, stage the worktree for merge. On failure, clean up.
- **Step 3d.** Respect disk-aware admission (check free space before checkout).

### Phase 4: Merge queue integration

- **Step 4a.** After gate success, enqueue the worktree merge via `MergeQueue`.
- **Step 4b.** Wait for merge completion (CI + local regression) before marking the
  node as done.
- **Step 4c.** On merge failure, mark the node as failed with merge diagnostics.

### Phase 5: Full state persistence

- **Step 5a.** Extend `GraphSnapshot` to include gate results, revision state, merge
  queue state, and plan metadata.
- **Step 5b.** Write snapshots atomically after each node completion (not just Activity).
- **Step 5c.** On resume, restore gate/revision/merge state alongside Activity replay.

### Phase 6: Approval channel

- **Step 6a.** Design an approval channel (stdin prompt or HTTP endpoint) for human
  review before Activity dispatch.
- **Step 6b.** When `--approval` is set, pause after plan conversion and display the
  Graph for review. Resume only on approval.

### Phase 7: Process-level cancellation

- **Step 7a.** On `FlowHandle::cancel()`, propagate cancellation to in-flight agent
  processes (process group kill).
- **Step 7b.** Clean up worktrees for cancelled nodes.
- **Step 7c.** Write a terminal snapshot before exit.

---

## Acceptance criteria

- [ ] Gate pipeline runs after each Activity and persists verdicts
- [ ] Gate failure triggers replan with configurable attempt cap
- [ ] Activities execute in isolated worktrees (not shared workspace)
- [ ] Successful activities merge through MergeQueue with CI gating
- [ ] Full lifecycle state survives crash/restart (not just Activities)
- [ ] `--approval` pauses for human review before dispatch
- [ ] Cancellation kills in-flight agents and cleans worktrees
- [ ] All existing Graph tests pass (`cargo test -p roko-graph`)
- [ ] `roko plan run --engine graph` produces equivalent outcomes to `--engine runner-v2`
  for a representative plan set

---

**Origin**: GAPS.md "Graph Engine incomplete -- PARTIAL" (2026-08-17)
