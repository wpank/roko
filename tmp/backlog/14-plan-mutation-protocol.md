# Plan Mutation Protocol

**Origin**: `tmp/architecture-archive/19-visual-composition.md` — Plan mutation protocol (lines 61–370)
**Status**: Backlog
**Priority**: P2 — unlocks conversation-as-editor UX; not blocking core self-hosting
**Size**: M (2–3 days)

---

## Problem statement

Plans are currently edited by directly writing TOML (`tasks.toml`). Agents that
need to adjust a plan — adding a task they discovered mid-execution, removing a
redundant step, inserting a checkpoint — have no structured way to express those
changes. They either emit raw TOML and hope the runtime reloads it, or write
freeform text that a human must interpret and apply.

This collapses two distinct concerns: _intent_ (what change the agent wants) and
_representation_ (how that change is stored). It makes undo/redo impossible, it
produces no audit trail of who changed what and when, and it cannot be safely
validated before application (a syntactically valid TOML edit can still introduce
a DAG cycle or a duplicate task ID).

The architecture spec (`19-visual-composition.md`) defines a `PlanMutation` enum
with nine typed variants as the solution. The enum is not yet implemented anywhere
in the workspace — agents still edit TOML directly.

---

## Proposed solution

Add a `PlanMutation` enum to `crates/roko-core/src/dispatch_plan.rs` (or a
co-located `plan_mutation.rs` module) with the nine variants from the spec:

```rust
pub enum PlanMutation {
    AddTask    { task: TaskSpec, after: Option<TaskId> },
    RemoveTask { id: TaskId },
    UpdateTask { id: TaskId, patch: TaskPatch },
    AddDependency    { from: TaskId, to: TaskId },
    RemoveDependency { from: TaskId, to: TaskId },
    Reorder    { task_ids: Vec<TaskId> },
    SetParallel { task_ids: Vec<TaskId> },
    AddCheckpoint { after: TaskId, name: String },
    UpdatePlanMeta  { patch: PlanMetaPatch },
}
```

A `PlanMutationBatch` bundles one or more mutations with metadata (author,
timestamp, session ID). A `PlanMutationLog` appends each applied batch to
`.roko/state/plan-mutations.jsonl` for the audit trail.

A `apply_mutations(plan: &mut Plan, batch: &PlanMutationBatch) -> MutationResult`
function validates then applies mutations in order. Validation gates:

1. `AddTask` with a duplicate `id` → rejected.
2. `RemoveTask` for a non-existent `id` → rejected.
3. `AddDependency` that introduces a cycle → rejected (topological sort check).
4. `Reorder` that references unknown IDs → rejected.
5. Rejected mutations return in a `rejected` array with reasons; valid mutations
   in the same batch still apply.

The runner's `event_loop.rs` calls `apply_mutations` whenever an agent emits a
`PlanMutation` tool call (new tool definition in `roko-std`), immediately
re-serialising the updated plan to `tasks.toml`. The mutation log entry is
appended before the TOML write so failures are recoverable.

Undo is implemented as a reverse-mutation mapping:
- `AddTask` ↔ `RemoveTask`
- `AddDependency` ↔ `RemoveDependency`
- `UpdateTask` stores a `before` snapshot for rollback

---

## Implementation location

| File | Change |
|---|---|
| `crates/roko-core/src/plan_mutation.rs` (new) | `PlanMutation`, `PlanMutationBatch`, `PlanMutationLog`, `MutationResult`, `apply_mutations` |
| `crates/roko-core/src/lib.rs` | `pub mod plan_mutation` |
| `crates/roko-std/src/tool/builtin/mod.rs` | New `plan_mutate` tool definition (agents call this instead of writing TOML directly) |
| `crates/roko-cli/src/runner/event_loop.rs` | Handle `plan_mutate` tool calls: validate + apply + persist + log |
| `crates/roko-serve/src/routes/plans.rs` | `POST /plans/{id}/mutate` — HTTP endpoint for dashboard use |

---

## Acceptance criteria

1. `apply_mutations` rejects `AddTask` with a duplicate ID and returns the
   rejection in `MutationResult::rejected`; valid mutations in the same batch
   still apply cleanly.
2. `apply_mutations` rejects `AddDependency` that would introduce a DAG cycle;
   the cycle is described in the rejection reason.
3. Every applied batch is appended to `.roko/state/plan-mutations.jsonl` with
   author, timestamp, session ID, and the full mutation list before the TOML
   write is attempted.
4. The runner handles a `plan_mutate` tool call during task execution: it
   validates, applies, re-serialises `tasks.toml`, and logs — without
   interrupting the running task.
5. `cargo test -p roko-core --lib plan_mutation` covers at minimum: add/remove
   roundtrip, cycle rejection, batch partial-apply, log append.
6. `POST /plans/{id}/mutate` returns `200` with the applied + rejected split,
   or `422` when the entire batch is invalid.

---

## References

- `tmp/architecture-archive/19-visual-composition.md` — Plan mutation protocol
  (lines 61–370), full `PlanMutation` enum (lines 71–114), validation rules
  (lines 332–338), supporting types `TaskSpec` / `TaskPatch` / `PlanMetaPatch`
  (lines 119–184)
- `tmp/architecture-archive/19-visual-composition.md` line 1115 —
  `PlanMutationApplied` event type (SSE / WebSocket broadcast contract)
- `crates/roko-core/src/dispatch_plan.rs` — existing plan dispatch types;
  `plan_mutation.rs` should be a sibling module
- `crates/roko-cli/src/runner/event_loop.rs` — runner dispatch / tool-call
  handling; mutation application hooks go here
- `crates/roko-std/src/tool/builtin/mod.rs` — where to register the new
  `plan_mutate` built-in tool
- `crates/roko-serve/src/routes/plans.rs` — existing plan HTTP routes; the
  `/mutate` endpoint extends this file
