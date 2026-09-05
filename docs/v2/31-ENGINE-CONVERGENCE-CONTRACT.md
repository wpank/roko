# 31 -- Engine Convergence Contract

> **Version**: 2.0
> **Date**: 2026-09-05
> **Scope**: Executable contract defining the post-convergence engine architecture. Graph is the sole engine; WorkflowEngine is retired (#276); Runner-v2 is retained as `--engine legacy` only. This document is the single unambiguous ownership and boundary reference for the converged engine.
> **Implementation status:** IMPLEMENTED -- Engine convergence is complete. Graph is the default and sole production engine. WorkflowEngine has been deleted (#276); its serializable contract types are preserved in `roko-runtime::workflow_contract`. Runner-v2 remains available as `--engine legacy` (alias `runner-v2`) for one deprecation cycle. `RuntimeServices` provides the shared service facade across all 7 `RuntimeProfile` variants. `ProductionPlanTopology` builds the canonical per-task subgraph, `GuaranteedFinallyController` provides guaranteed cleanup, and the 12-row `FeedbackSettler` drives completion sinks.
> **Backlog**: #242

---

## Purpose

This contract defined the parity requirements for replacing Runner-v2 and `WorkflowEngine` with a unified `GraphEngine`. **That migration is now complete.** This contract:

1. Records the frozen boundary types that flow between orchestration stages.
2. Provides three golden fixtures that load through both plan loading and graph conversion.
3. Classifies every production stage as Workflow or Activity.
4. Records the cutover invariants that held during migration.
5. Documents the post-convergence architecture.

### Post-convergence summary

- **Graph** is the sole production engine (`PlanEngine::Graph`, the `#[default]` variant).
- **Runner-v2** is retained as `--engine legacy` (alias `runner-v2`) for one deprecation cycle.
- **WorkflowEngine** has been deleted (#276). Its serializable types (`WorkflowConfig`, `WorkflowRunConfig`, `WorkflowRunReport`, `CommitOutcome`, `Phase`) are preserved in `roko-runtime::workflow_contract` for downstream consumers.
- **RuntimeServices** provides the shared service facade. 7 `RuntimeProfile` variants (`FullPlan`, `GraphPlan`, `Workflow`, `DirectLight`, `AgentServer`, `ChatLight`, `AuthoredGraph`) select which of the 6 `ServiceBundleId` bundles (Dispatch, Prompt, Feedback, Extensions, Observation, Guards) are required, optional, or forbidden.
- **ProductionPlanTopology** (`roko-graph::topology`) builds the canonical 11-node per-task subgraph: TaskContext + 6 enrichers (knowledge, episodes, playbook, modulation, safety, experiment) + Compose + TaskExecutor + Gate + SuccessBoundary.
- **GuaranteedFinallyController** (`roko-graph::finally`) wraps graph execution with absolute cleanup guarantees (terminal receipt, lease release, agent stop, snapshot flush) regardless of success, failure, panic, or cancellation.
- **FeedbackSettler** (`roko-execution::feedback::settler`) drives 12 ordered completion sinks with exactly-once idempotency per receipt: attempt_receipt, actual_cost, structured_audit (critical), then episode, efficiency, routing, error_pattern, playbook, knowledge, daimon, conductor, projection (optional).
- **CellResources** (`roko-graph::cell`) injects shared service handles (e.g., `SharedGateEvaluator`) into `CellContext` for each Cell execution.

---

## Stale Claim Corrections

The following claims from earlier audit documents are **incorrect** as of 2026-09-03:

| Stale Claim | Correction |
|---|---|
| "TaskDispatcher is not wired" | `GraphTaskDispatcher` and `SharedAgentFactory` are injected by `cmd_plan_run_engine` with plugin/MCP support, actual-cost plan budget, graph checkpoints, extensions, and telemetry. |
| "Graph has no persistence" | `GraphSnapshot` serializes per-node status and Activity outputs. `ActivityRecorder`/`ActivityReplayer` provide JSONL-based durable replay. Hot Graph tick state persists across restarts via `BudgetCheckpoint` and `HotGraphCheckpointManifest`. |
| "GraphEngine::validate() does not invoke validate_edges()" | `validate_for_start()` calls `graph.validate_edges(&self.registry)` and rejects any `EdgeValidationError`. |

---

## Frozen Boundary Table

These version-1 boundary types are owned by their respective downstream packets. This contract copies them verbatim; it does not redesign them.

| Boundary | Owner | Required Fields |
|---|---|---|
| Task context Signal | #265 `ComposeRequest` | run_id, plan_id, task_id, role, scoped dependency context, task context |
| Composed prompt Signal | #265 `ComposedPrompt` | scope IDs (run/plan/task), text, estimated tokens, included section IDs, dropped section IDs, warnings |
| Dispatch outcome | #247 `TaskDispatchOutcome` | output Signals, status, provider, model, input_tokens, output_tokens, cost_micro_usd, changed files, session_id, agent_id, terminal attempt receipt |
| Gate verdict | #250 `ProductionGateVerdictV1` | request fingerprint, workspace fingerprint, ordered rungs, terminal state, mostly_passing, duration_ms, adaptive snapshot |
| Terminal feedback | #253 `TaskAttemptReceiptV1` | Attempt metadata plus ordered 12-sink settlement receipts |
| Merge request/outcome | #254 `CompletionDeliveryRequest`, `CompletionDeliveryReceiptV1` | Plan ID, branch, files changed, priority, merge status, error |

---

## Golden Expected Schema

Every fixture `expected.json` must conform exactly to this schema. Missing fields fail serde deserialization. Unknown fields are denied (`#[serde(deny_unknown_fields)]`).

```json
{
  "schema_version": 1,
  "fixture_id": "string",
  "graph_fingerprint": "string (BLAKE3 hex)",
  "request_fingerprints": ["string"],
  "prompt_fingerprints": ["string"],
  "tasks": [
    {
      "task_id": "string",
      "dependencies": ["string"],
      "status": "completed | failed | skipped | cancelled",
      "attempts": 1,
      "provider": "string",
      "model": "string",
      "role": "string",
      "effort": "string (tier)",
      "workspace_fingerprint": "string",
      "input_tokens": 0,
      "output_tokens": 0,
      "cost_micro_usd": 0
    }
  ],
  "events": [
    {
      "sequence": 0,
      "event_type": "string",
      "source": "string",
      "payload": {}
    }
  ],
  "receipts": [
    {
      "idempotency_key": "string",
      "owner": "string",
      "state": "string",
      "evidence_fingerprint": "string"
    }
  ],
  "final": {
    "plan_status": "completed | failed | cancelled",
    "completed_task_ids": ["string"],
    "skipped_task_ids": ["string"],
    "failed_task_ids": ["string"],
    "total_input_tokens": 0,
    "total_output_tokens": 0,
    "total_cost_micro_usd": 0,
    "merge_state": "merged | skipped | not_attempted",
    "publication_state": "published | skipped | not_attempted",
    "terminal_event_id": "string"
  }
}
```

### Schema rules

- `tasks` is sorted by `task_id` (lexicographic).
- `events` retains identity (source, sequence) but timestamps are removed for deterministic comparison.
- `receipts` is sorted by `idempotency_key`.
- `final` contains exactly the listed fields; no optional nesting.

---

## Phase / Symbol / Receipt Table

These are the current production phase anchors. Downstream Graph receipts use the fixed `receipt_label` even after legacy symbols retire.

| Phase | Current Entry | Current Action | Current Exits | Receipt Label |
|---|---|---|---|---|
| `AutoFixing` | `PhaseKind::Gating` + `ExecutorEvent::GateFailed` below cap | `ExecutorAction::SpawnAgent { role: AutoFixer, task: "fix" }` | `AutoFixDone` -> `Gating`, `Skip` -> `Skipped`, cap -> `Failed` | `phase.autofix` |
| `Verifying` | `GatePassed` or `VerifyRegenDone` | `ExecutorAction::RunVerify` | `VerifyPassed` -> `Reviewing`, `VerifyFailed` -> `RegeneratingVerify`, `Skip` | `phase.verify` |
| `Reviewing` | `VerifyPassed` | `SpawnAgent { role: Auditor, task: "review" }` | `ReviewApproved` -> `DocRevision`, `ReviewRejected` -> `Implementing`, `Skip` | `phase.review` |
| `DocRevision` | `ReviewApproved` | `SpawnAgent { role: Scribe, task: "docs" }` | `DocRevisionDone` -> `Merging`, `Skip` | `phase.docs` |
| `Merging` | `DocRevisionDone` or `OperatorMerge` | `ExecutorAction::MergeBranch` | `MergeSucceeded` -> `Complete`, `MergeFailed` -> `Failed`, `Skip` | `phase.merge` |

### Express mode

Express mode records an auto-completed receipt for disabled review/docs phases. The receipt `state` is `"auto_completed"` with `evidence_fingerprint` set to the plan's graph fingerprint.

### Warm-review

Warm-review may prepare an agent but cannot settle `phase.review` before the phase entry event arrives. The review agent's output is held until the phase machine transitions to `Reviewing`.

### Mostly-passing

Mostly-passing remains gate evidence that selects targeted autofix. It never changes a failed verdict to success. A mostly-passing gate result triggers `AutoFixing` only for the specific failed rungs, not a full re-implementation.

---

## Stage Classification: Workflow vs Activity

Every production stage is classified for replay and idempotency:

| Stage | Class | Replay Rule | Idempotency |
|---|---|---|---|
| Plan loading and DAG construction | Workflow | Re-derive from `tasks.toml` on resume | Deterministic given same input |
| Graph conversion (`plan_to_graph`) | Workflow | Re-derive from plan data | Deterministic |
| Graph fingerprinting | Workflow | Re-derive from graph definition | BLAKE3 stable given sorted nodes/edges |
| Topological sorting and wave planning | Workflow | Re-derive from graph structure | Deterministic |
| Budget reservation and tracking | Workflow | Re-derive from policy + consumed totals | Token/cost counters are monotonic |
| Compose/prompt assembly | Workflow | Re-derive from task context + sections | Deterministic given same context + section set |
| Provider dispatch (LLM call) | Activity | Record output; replay substitutes | Non-deterministic; JSONL recorded |
| Tool execution | Activity | Record output; replay substitutes | Non-deterministic; JSONL recorded |
| Gate execution (compile, test, clippy) | Activity | Record verdict; replay substitutes | Non-deterministic (depends on workspace state) |
| Verify command execution | Activity | Record verdict; replay substitutes | Non-deterministic |
| Review agent | Activity | Record decision; replay substitutes | Non-deterministic |
| Doc revision agent | Activity | Record output; replay substitutes | Non-deterministic |
| Merge/publish | Activity | Record outcome; replay substitutes | Non-deterministic (depends on git state) |
| Snapshot persistence | Workflow | Always written fresh | Idempotent overwrite |
| Episode logging | Workflow | Append-only, deduplicated by episode ID | Idempotent insert |
| Feedback settlement | Workflow | Settle once per idempotency key | At-most-once per receipt |
| Adaptive threshold update | Workflow | Derived from gate verdicts | Monotonic EMA update |
| Telemetry emission | Workflow | Fire-and-forget; loss is acceptable | Best-effort delivery |

---

## Capability Matrix (post-convergence)

Each runtime variant is classified for its current support after engine convergence.

| Capability | Graph (default) | Authored Graph | Legacy (Runner-v2) | Direct Dispatch | Agent Server | Chat |
|---|---|---|---|---|---|---|
| Task DAG execution | live | live | live (deprecated) | out-of-scope | out-of-scope | out-of-scope |
| Worktree ownership | live | absent | live (deprecated) | absent | absent | absent |
| Graph-native gates | live | absent | absent | absent | absent | absent |
| Rich streaming events | live | absent | live (deprecated) | absent | partial | absent |
| Terminal learning feedback | live | absent | live (deprecated) | absent | absent | absent |
| Structural replan | live | absent | live (deprecated) | absent | absent | absent |
| Merge/publish pipeline | live | absent | live (deprecated) | absent | absent | absent |
| Interactive control (pause/resume/cancel) | live | absent | live (deprecated) | absent | absent | absent |
| Budget enforcement | live | live | live (deprecated) | absent | absent | absent |
| Snapshot/resume | live | live | live (deprecated) | absent | absent | absent |
| Activity recording/replay | live | live | partial (deprecated) | absent | absent | absent |
| Graph fingerprinting | live | live | absent | absent | absent | absent |
| Edge type-schema validation | live | live | absent | absent | absent | absent |
| Hot Graph (tick-driven) | absent | live | absent | absent | absent | absent |
| Telemetry Lens routing | live | live | absent | absent | absent | absent |
| Plugin/MCP cell dispatch | live | live | absent | absent | absent | absent |
| ProductionPlanTopology | live | absent | absent | absent | absent | absent |
| GuaranteedFinallyController | live | absent | absent | absent | absent | absent |
| 12-row FeedbackSettler | live | absent | absent | absent | absent | absent |
| CellResources injection | live | live | absent | absent | absent | absent |
| Single-prompt compose-gate | live (via template) | absent | absent | live | absent | absent |
| One-shot LLM dispatch | live (via template) | absent | absent | live | live | live |
| Agent chat REPL | absent | absent | absent | absent | absent | live |

**WorkflowEngine column removed**: WorkflowEngine was deleted by #276. Its single-prompt compose-gate and one-shot dispatch capabilities are now served by Graph templates (Workflow profile) and direct dispatch respectively.

### Direct dispatch classification

Direct one-shot dispatch (e.g., `roko run "<prompt>"`, agent sidecar `/message`) is a non-orchestrating `RuntimeServices`-backed service outside engine-retirement scope. Graph templates may consume it as a leaf Cell, but #283 need not wrap every one-shot request in a single-node Graph.

---

## Cutover Invariants

These invariants held during the engine cutover and continue to hold in the converged state.

1. **No duplicate provider calls**: A task-attempt in the new engine must produce exactly one provider dispatch. Shadow runs must not re-invoke providers that were already called by the primary engine.

2. **No duplicate episodes**: Each task-attempt produces at most one episode record. The episode ID is derived from `(run_id, plan_id, task_id, attempt_index)` and is deduplicated on insert.

3. **No duplicate feedback settlement**: Each `TaskAttemptReceiptV1` settles its 12 sinks exactly once per idempotency key. The idempotency key is `(run_id, plan_id, task_id, attempt_index, sink_name)`.

4. **No duplicate merges**: A plan's merge request is enqueued at most once per terminal completion. Re-enqueue after resume must check the merge queue for an existing entry with the same plan ID.

5. **No duplicate PR updates**: GitHub PR comments and status checks are keyed by `(plan_id, task_id, phase)` and are idempotent updates, not appends.

6. **No duplicate terminal events**: The terminal event (plan completed/failed/cancelled) is emitted exactly once. Resume from a snapshot that already has a terminal event must not emit another.

7. **Resume must not replay completed Activities**: A resumed execution must recognize previously completed Activity nodes from the snapshot/checkpoint and skip them. Only Workflow nodes are re-derived.

8. **Shadow runs must be side-effect-free**: A shadow Graph execution running alongside the legacy engine must not write to the signal log, episode log, feedback sinks, merge queue, or GitHub. It may only compare its computed outcomes against the primary engine's actual outcomes. (This invariant applied during the migration period; with convergence complete, shadow runs are no longer needed.)

---

## Three Frozen Fixtures

### 1. `diamond_success`

**Topology**: A -> {B, C} -> D (diamond DAG)

All four tasks succeed. Tasks B and C execute in parallel after A completes. Task D executes after both B and C complete.

**Expected**: All tasks `completed`, zero failed/skipped, merge state `not_attempted`, all events in sequence.

### 2. `gate_replan_cap`

**Topology**: Linear chain with gate failure.

One task succeeds, its gate fails, one structural replan (#252) produces a revised task, then the replan cap is exhausted.

**Expected**: Original task `completed`, gate verdict `failed` with `mostly_passing: false`, replan task `completed`, second gate failure hits cap, plan status `failed`, merge state `not_attempted`.

### 3. `cancel_resume_budget`

**Topology**: A -> B -> C (linear chain with budget constraint).

Task A completes. Task B is cancelled mid-flight. Resume does not replay A. Task C is blocked at its budget boundary (cost would exceed plan budget).

**Expected**: A `completed`, B `cancelled`, C `skipped` (budget exceeded), plan status `failed`, resume checkpoint contains A's Activity output but not B's.

---

## Fixture File Inventory

Each fixture directory under `crates/roko-graph/tests/fixtures/engine_convergence/` contains:

| File | Purpose |
|---|---|
| `tasks.toml` | Plan task definitions loadable by `TasksFile::parse_str` and convertible via `plan_to_graph` |
| `graph.toml` | Equivalent authored graph definition loadable by `loader::load_from_str` |
| `expected.json` | Golden normalized outcomes conforming to the frozen schema above |
| `activities.jsonl` | Recorded Activity inputs (provider outputs) for deterministic replay |

No fixture invokes a live provider, git operation, feedback sink, or publication port. All Activity outputs are pre-recorded in `activities.jsonl`.

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 2.0 | 2026-09-05 | Post-convergence update. Graph is the sole engine; WorkflowEngine retired (#276); Runner-v2 retained as `--engine legacy`. Updated capability matrix, implementation status, cutover invariants, and post-convergence architecture summary documenting RuntimeServices (7 profiles), ProductionPlanTopology, GuaranteedFinallyController, 12-row FeedbackSettler, and CellResources injection. |
| 1.0 | 2026-09-03 | Initial contract. Frozen boundary table, golden schema, three fixtures, capability matrix, cutover invariants. Backlog #242. |
