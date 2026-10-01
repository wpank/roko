# Engine Convergence

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 16.
> Documents the three-phase convergence from Runner-v2 through WorkflowEngine
> to the sole Graph engine.

---

## Overview

The execution engine went through three phases of convergence. Understanding
this history is important for interpreting code comments, git blame, and the
remaining `--engine legacy` flag that exists only for backward compatibility
during the final deprecation cycle.

---

## Timeline

### Phase 1: Runner-v2 (original)

The original execution engine was a monolithic event loop in `roko-cli`.
It directly managed plan phases, agent dispatch, gate invocation, snapshot
persistence, and the merge queue from a single `PlanRunner` struct with
30+ fields.

**Characteristics:**

- All orchestration logic in one file (`orchestrate.rs`)
- `ParallelExecutor` state machine emitting `ExecutorAction` values
- Runtime harness (`PlanRunner`) dispatching actions to real subsystems
- Tight coupling between scheduling, dispatch, and state persistence
- Tested via end-to-end tests with real filesystem operations

**Strengths:** simple mental model, everything in one place, easy to trace
control flow through a single file.

**Weaknesses:** difficult to test individual stages in isolation, hard to
share service construction with other execution surfaces (serve, ACP,
agent sidecar), monolithic snapshot format that captured entire plan state
rather than per-node state, no way to resume at enrichment-stage
granularity.

### Phase 2: Graph engine introduced (#260)

PR #260 made `PlanEngine::Graph` the default execution engine. Plans were
converted to `Graph` values via `plan_to_graph()` and executed by the
`GraphEngine`. Runner-v2 remained available as `--engine legacy`.

**Key changes:**

- `plan_to_graph()` converter translating `tasks.toml` into a `Graph`
- `GraphEngine` with topological sorting, bounded parallel waves, and
  conditional routing
- `ActivityRecorder` and `ActivityReplayer` for deterministic replay
- `GraphSnapshotV2` with BLAKE3 fingerprinting for drift detection
- Cell-based execution model where each node type is a `Cell` implementation
- `CellRegistry` mapping `cell_type` strings to `Cell` implementations
- `ProductionPlanTopology` building 11-node subgraphs per task

**Coexistence:** both engines consumed the same `RuntimeServices` value
(#243), ensuring provider health, rate limiters, cost tables, prompt
caches, and process supervisors were shared regardless of engine choice.
This was critical -- without shared services, the two engines would have
maintained separate health registries and rate limiters, causing incorrect
rate limiting and divergent health views.

### Phase 3: WorkflowEngine retired (#276)

PR #276 deleted the intermediate `WorkflowEngine` entirely. This was a
separate abstraction that had been introduced between Runner-v2 and the
Graph engine but never gained traction. Its responsibilities were fully
absorbed by the Graph engine.

**After #276:**

- `GraphEngine` is the sole execution engine
- Runner-v2 retained as `--engine legacy` for one release cycle
- `ProductionPlanTopology` builds the canonical 11-node-per-task subgraph
- A `GuaranteedFinallyController` was drafted but never compiled; it was deleted on
  2026-10-01 (gap-ff6e83)

### Phase 4: Runner-v2 removal (current)

Runner-v2 is being removed. The `--engine legacy` flag and `FullPlan`
profile exist only for backward compatibility during this final transition.
All new execution uses `GraphPlan` exclusively.

---

## What Changed Architecturally

| Aspect | Runner-v2 | Graph Engine |
|---|---|---|
| Execution model | Event loop with tick/apply | DAG of Cells in topological waves |
| Enrichment | Controller-side, opaque | In-graph nodes (6 parallel enrichers) |
| Gate invocation | Controller-side | In-graph `GateCell` nodes |
| Prompt composition | Controller-side | In-graph `ComposeCell` |
| Resume granularity | Per-plan phase | Per-node status + per-Activity replay |
| Snapshot format | `ExecutorSnapshot` (JSON) | `GraphSnapshotV2` (JSON + BLAKE3) |
| Parallelism | `JoinSet` + semaphore | Topological waves + semaphore |
| Conditional routing | Not supported | `EdgeCondition` on edges |
| Budget tracking | `plan_costs` HashMap | Atomic microdollar tracker |
| Process isolation | Manual tracking | `ProcessSupervisor` |
| Observability | Action/Event logs | Per-node lifecycle events |

---

## Why Convergence Was Necessary

The Runner-v2 architecture had several structural problems that motivated
the transition:

1. **Testing difficulty.** Enrichment, composition, and gating were opaque
   controller-side operations. Testing them required end-to-end runs with
   real filesystems. The Graph engine expresses these as cells that can be
   tested individually with mock inputs.

2. **Service duplication.** Without a shared builder, each execution
   surface (CLI, serve, ACP, sidecar) independently constructed providers,
   rate limiters, and caches. `RuntimeServicesBuilder` (#243) solved this.

3. **Coarse resume.** Runner-v2 snapshots captured per-plan phase, not
   per-node status. A crash during enrichment required re-running the
   entire enrichment phase. The Graph engine tracks per-node status and
   replays only failed/interrupted nodes.

4. **No conditional routing.** Runner-v2 could not skip enrichment stages
   based on cached results. The Graph engine's `EdgeCondition` enables
   conditional routing, allowing enricher nodes to be skipped when their
   outputs are already available.

---

## RuntimeServices as the Bridge

The `RuntimeServicesBuilder` (#243) was the key enabling abstraction for
convergence. By extracting service construction into a profile-driven
builder, both engines could consume identical provider health registries,
rate limiters, cost tables, prompt caches, and process supervisors.

Without `RuntimeServices`, each engine would have independently constructed
its own service instances, leading to:

- Duplicate provider health tracking (health status divergence between
  engines would cause confusing behavior)
- Separate rate limiters (double the actual API rate, risking provider
  throttling)
- Inconsistent cost accounting (each engine would report different costs
  for the same run)
- Separate prompt caches (double memory usage)

The builder ensures exactly one construction path regardless of engine
choice. Now that convergence is complete, it continues to serve all seven
runtime profiles from a single code path.

---

## Migration Path for Existing Code

Code that references Runner-v2 concepts should be updated:

| Runner-v2 | Graph Engine | Notes |
|---|---|---|
| `ParallelExecutor` | `GraphEngine` | Different execution model |
| `PlanState` | `NodeStatus` per node | More granular |
| `ExecutorAction` | Cell execution | Cells are the action vocabulary |
| `ExecutorEvent` | Node completion | Status propagation replaces events |
| `ExecutorSnapshot` | `GraphSnapshotV2` | Different format, BLAKE3 fingerprint |
| `PlanRunner::run()` | `drive_controller()` | Different entry point |
| `tick()` loop | `execute()` on engine | One-shot vs. tick-based |
| `PlanStateMachine` | Flow lifecycle | Controller-managed transitions |

---

## Remaining Runner-v2 Code

The following Runner-v2 artifacts still exist for the deprecation cycle:

- `RuntimeProfile::FullPlan` -- profile for Runner-v2 plan execution
- `--engine legacy` CLI flag -- selects Runner-v2 instead of Graph
- Runner-v2 snapshot path (`.roko/state/state-snapshot.json`)
- `PlanStateMachine` in `roko-core` -- phase transition logic
- Legacy snapshot deserialization compat in `ExecutorSnapshot::from_json()`

These will be removed once the deprecation cycle completes. New features
should not target Runner-v2.

---

## Verification

```bash
# Confirm Graph is the default engine
cargo run -p roko-cli -- plan run plans/<dir>
# This uses GraphPlan profile and GraphEngine

# Confirm legacy flag still works (during deprecation)
cargo run -p roko-cli -- plan run plans/<dir> --engine legacy
# This uses FullPlan profile and Runner-v2
```
