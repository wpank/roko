# 117 — Plan-Level Wave Computation (Cross-Plan DAG with Kahn's Algorithm)

**Priority**: P2 — Prerequisite for wave-based TUI visualization (#125), queue manifest milestones (#116), and critical-path ETA; without it, multi-plan parallelism has no structured grouping.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/runner/`, `crates/roko-cli/src/commands/plan.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_checklist-gaps.md` §1.2, `tmp/backlog/_mori-old-gaps.md` MO-06

---

## Background

Within a single plan, roko already computes a task DAG using Kahn's algorithm in `runner/task_dag.rs`. The same algorithm applies at the plan level: given a set of plans where each plan may declare `depends_on_plan: Vec<String>`, we can compute which plans form "wave 0" (no dependencies), "wave 1" (depends only on wave 0 plans), and so on. Plans within the same wave can execute in parallel; sequential waves enforce ordering.

Mori's `orchestrator/dag.rs` had `PlanDag::compute_waves()` doing exactly this. Roko's runner loads plans and has the dependency data in each plan's TOML metadata but does not group them into waves. Without waves, operators cannot predict parallelism, the TUI cannot show wave progress, and the queue manifest has no grouping foundation.

Wave computation is also a prerequisite for file-overlap analysis (detecting when two parallel plans touch the same crate), critical path ETA (sum of the longest sequential wave chain), and the F2 plan tree widget showing wave headers.

## Current State

- `crates/roko-cli/src/runner/task_dag.rs` — Kahn's algorithm exists for task-level DAG computation; plan-level equivalent is absent.
- `tasks.toml` schema includes `depends_on_plan: Vec<String>` — the edge data already exists.
- `crates/roko-cli/src/runner/event_loop.rs` — loads plans but does not compute wave index per plan.
- `roko plan list` — shows plans in a table without wave grouping.
- No `wave_index` field is attached to any plan struct at runtime.

## Implementation Plan

1. **Create `PlanDag` in `crates/roko-cli/src/runner/plan_dag.rs`**:
   ```rust
   pub struct PlanDag {
       pub plans: Vec<PlanNode>,   // ordered by wave_index asc
       pub waves: Vec<Vec<String>>, // wave_index → Vec<plan_id>
       pub critical_path: Vec<String>, // longest dependency chain
       pub critical_path_minutes: u64,
   }

   impl PlanDag {
       pub fn compute(plans: &[PlanManifest]) -> Result<Self, DagError>
   }
   ```

2. **Implement Kahn's algorithm at plan level**: Port the logic from `task_dag.rs`. Input: `Vec<PlanManifest>` with `id` and `depends_on_plan`. Output: per-plan `wave_index: u32`, sorted wave groupings, cycle detection returning `DagError::Cycle`.

3. **Attach `wave_index` to runtime plan state**: Update the plan-tracking struct in `event_loop.rs` to carry `wave_index`. When persisting runner state, include wave assignments.

4. **Update `plan_loader::load_plans()`**: After loading all plans, call `PlanDag::compute()` and attach wave indices to the loaded plan manifests. Return a `PlanDag` alongside the plan list.

5. **Wave-aware execution in event loop**: Currently plans are dispatched when their task-level prerequisites are met. Add a wave-level gate: a plan in wave N can only start after all plans in wave N-1 are in a terminal state (completed or failed-and-skipped).

6. **`roko plan list --waves` flag**: Print plans grouped by wave with a `Wave N:` header per group. Show parallelism width (number of plans per wave).

7. **Cycle detection error message**: When `DagError::Cycle` is returned, identify the cycle members and print a human-readable error: "Cycle detected: plan-A → plan-B → plan-A".

8. **File overlap warning**: After computing waves, detect pairs of plans in the same wave that share the same `crates_touched` entries (from tasks.toml metadata). Emit a warning but do not block execution.

## Acceptance Criteria

1. `PlanDag::compute()` correctly assigns wave indices for a set of plans with known dependencies.
2. Cycle detection returns an error with the cycle member IDs listed.
3. `roko plan list --waves` groups plans by wave with a header.
4. Wave 0 plans start at runner startup; wave 1 plans start only after all wave 0 plans complete.
5. Plans with no `depends_on_plan` all land in wave 0.
6. File overlap warning is emitted when two wave-parallel plans share a crate.
7. Unit tests cover: linear chain, diamond, independent plans (all wave 0), cycle detection.

## Verification Checklist

- [ ] Unit test: three plans A→B→C produces waves [A], [B], [C].
- [ ] Unit test: A and B have no deps; C depends on both → waves [A,B], [C].
- [ ] Unit test: A→B→A produces `DagError::Cycle`.
- [ ] `roko plan list --waves` output has `Wave 0:` and `Wave 1:` sections.
- [ ] Integration test: run a two-wave plan set; verify wave 1 starts only after wave 0 completes.
- [ ] File overlap warning appears when two wave-parallel plans share a crate name.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/plan_dag.rs` | New file: `PlanDag`, Kahn's algorithm |
| `crates/roko-cli/src/runner/mod.rs` | Export `plan_dag` module |
| `crates/roko-cli/src/runner/event_loop.rs` | Use `PlanDag` for wave-aware dispatch |
| `crates/roko-cli/src/commands/plan.rs` | Add `--waves` flag to `roko plan list` |
