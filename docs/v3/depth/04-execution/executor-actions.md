# Executor Actions

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 8.
> Preserves and updates content from v1 `01-orchestration/05-executor-actions.md`.

---

## Overview

In the Graph engine, execution is driven by Cell dispatch rather than
explicit action types. However, the conceptual vocabulary of "what the
executor can do" remains relevant for understanding the system. Each Cell
type in the production topology corresponds to an executor action.

This document maps the v1 `ExecutorAction` vocabulary to the current Cell
execution model and documents the runtime effects of each action.

**Source:** `crates/roko-graph/src/engine.rs`,
`crates/roko-graph/src/topology.rs`

---

## Cell-Based Action Vocabulary

The Graph engine's action vocabulary is its `CellRegistry`. Each cell type
is a reified action:

| Cell type | v1 equivalent | Effect |
|---|---|---|
| `plan.context` | (part of DispatchPlan) | Emit task context Signal |
| `plan.knowledge` | (part of SpawnAgent/Strategist) | Query neuro knowledge store |
| `plan.episodes` | (part of SpawnAgent/Strategist) | Query recent episodes |
| `plan.playbook` | (part of SpawnAgent/Strategist) | Query matching playbooks |
| `plan.modulation` | (part of SpawnAgent/Strategist) | Daimon affect modulation |
| `plan.safety` | (part of SpawnAgent/Strategist) | Safety contract checks |
| `plan.experiment` | (part of SpawnAgent/Strategist) | A/B experiment assignment |
| `plan.compose` | (part of SpawnAgent) | Assemble system prompt |
| `task-executor` | SpawnAgent(Implementer) | Build prompt, launch LLM, collect response |
| `plan.gate` | RunGate | Invoke compile/test/clippy pipeline |
| `plan.success-boundary` | CompletePlan | Mark task complete, emit downstream Signal |

### Enrichment (6 parallel cells)

The six enricher cells run in parallel within a single topological wave.
Each queries a different subsystem and produces a Signal that feeds into
the compose cell:

- **KnowledgeCell**: queries the neuro knowledge store for entries relevant
  to the task's domain, files, and description. Uses HDC fingerprinting
  for semantic similarity matching.
- **EpisodesCell**: queries recent episodes for patterns related to the
  task. Filters by plan context and model history.
- **PlaybookCell**: queries playbook store for matching when/then entries.
  Returns the top matches ranked by relevance score.
- **ModulationCell**: queries the Daimon affect engine for arousal,
  confidence, and dispatch modulation parameters. Higher arousal can
  influence model selection through the cascade router.
- **SafetyCell**: checks safety contracts and tool policy for the
  requested agent role. Returns the intersected allow/deny lists and any
  pre-dispatch safety conditions.
- **ExperimentCell**: assigns any active A/B experiments and returns the
  assigned variant. Experiment assignments are durable -- the same task
  always gets the same variant within a plan run.

All six enricher cells are classified as Workflow (deterministic), so their
outputs are never recorded. On resume, they re-execute from their inputs.

### Composition

**ComposeCell** receives seven inputs (six enrichment Signals + task
context) and assembles the complete system prompt via
`RoleSystemPromptSpec`. The composed prompt is a single Signal carrying
the assembled prompt text, model selection, tool policy, and MCP
configuration.

The composition uses the 9-layer prompt builder:

1. Core identity and capabilities
2. Role-specific instructions
3. Plan context (PRD, task description)
4. Learned context (skills, playbooks, knowledge)
5. Feedback context (gate failures, review feedback)
6. Operating constraints (budget, timeout, tool restrictions)
7. Experiment overrides (A/B variant sections)
8. Safety conditions (pre-dispatch checks)
9. Modulation parameters (affect-adjusted model hints)

### Execution

**TaskExecutorCell** (cell type `task-executor`) is the primary action
cell. It:

1. Extracts the composed prompt from its input Signal.
2. Resolves the provider via `DispatchFactory`.
3. Selects the model via `CascadeRouter` (LinUCB bandit with anomaly
   detection), incorporating iteration count, prior gate failure, crate
   familiarity, and affect confidence.
4. Builds an `AgentRunConfig` with role-specific parameters.
5. Launches the agent via the configured provider adapter.
6. Records the agent process in `ProcessSupervisor`.
7. Collects the response and emits it as an output Signal.
8. Logs an efficiency event and episode entry on completion.

### Gate

**GateCell** (cell type `plan.gate`) invokes the gate pipeline:

| Rung | Gate | What it checks |
|---|---|---|
| 0 | `CompileGate` | `cargo build --workspace` |
| 1 | `TestGate` | `cargo test --workspace` |
| 2 | `ClippyGate` | `cargo clippy --workspace --no-deps -- -D warnings` |
| 3-6 | Higher-rung oracles | Diff analysis, semantic verification, task-specific checks |

Gate results are emitted as Signals carrying the verdict, summary, and
duration. The controller uses gate results to decide whether to continue
to verification or enter the auto-fix loop.

### Success Boundary

**SuccessBoundaryCell** (cell type `plan.success-boundary`) marks a task as
complete and emits a downstream Signal that unblocks dependent tasks. In
the production topology, inter-task dependencies connect the predecessor's
success boundary to the dependent's task context.

---

## Controller-Side Actions

Several actions are not expressed as graph nodes. They are performed by the
controller (`drive_controller`) outside the DAG:

| Action | When | Effect |
|---|---|---|
| Worktree creation | Before graph execution | `WorktreeManager::ensure_for_plan()` |
| Verification | After gate pass | Run task-level verify commands from tasks.toml |
| Code review | After verification | Launch auditor agent |
| Documentation | After review approval | Launch scribe agent |
| Merge | After documentation | Enqueue in `MergeQueue` |
| Snapshot | Periodically | Write `GraphSnapshotV2` to disk |
| Cleanup | On terminal | `GuaranteedFinallyController` cleanup |

These actions exist outside the graph because they operate on the
execution context (worktrees, merge queue, snapshots) rather than on task
data.

---

## Comparison: v1 Actions vs. Current Cells

| v1 ExecutorAction | Current equivalent | Scope change |
|---|---|---|
| `DispatchPlan { plan_id }` | Controller creates worktree + graph | Controller-side |
| `SpawnAgent { plan_id, role, task }` | `task-executor` cell | In-graph |
| `RunGate { plan_id, rung }` | `plan.gate` cell | In-graph |
| `RunVerify { plan_id }` | Controller verify step | Controller-side |
| `MergeBranch { plan_id }` | Controller merge step | Controller-side |
| `FailPlan { plan_id, reason }` | `GuaranteedFinallyController` | Controller-side |
| `CompletePlan { plan_id }` | `TerminalReceipt` | Controller-side |
| `PausePlan { plan_id }` | Cancellation token | Controller-side |
| `ResumePlan { plan_id }` | Snapshot restore | Controller-side |
| `Reorder { plan_id, position }` | Cross-plan DAG recompute | Controller-side |

The key architectural shift is that enrichment, composition, and gating
are now in-graph nodes rather than opaque controller actions. This makes
them observable (each stage emits events), replayable (Activity outputs
are recorded), and individually resumable (per-node status tracking).

---

## Agent Roles

Each agent-invoking cell implicitly selects a role. The role determines
model selection, tool policy, and prompt assembly:

| Role | Cell/Action | Prompt layers | Tool policy |
|---|---|---|---|
| Implementer | `task-executor` | All 9 layers | Role-specific allowlist |
| AutoFixer | `task-executor` (retry) | Identity + role + gate failure | Restricted to fix tools |
| Strategist | Controller enrichment | Identity + role + plan context | Read-only tools |
| Auditor | Controller review | Identity + role + diff | Read-only tools |
| Scribe | Controller docs | Identity + role + changed files | Doc tools only |

Role selection drives model selection through the cascade router
(higher-capability models for complex roles), tool policy through the
agent contract (deny-wins intersection), and prompt assembly through
`RoleSystemPromptSpec` (role-specific instructions and constraints).
