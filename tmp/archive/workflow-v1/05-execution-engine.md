# PRD-05 — Execution Engine

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Crate**: `roko-workflow` (engine submodule)
**Prerequisites**: PRD-00, PRD-02, PRD-04

---

## 0. Scope

This document defines the runtime that turns a loaded `Workflow` + `WorkflowInput` into a sequence of Module invocations, Artifact productions, lifecycle events, and a final Output. The engine handles state-graph traversal with conditional edges, loops, fan-out / fan-in, sub-workflow composition, human-in-loop, retries, cancellation, budget enforcement, resumability, and episode logging.

The engine is the single execution path for everything: Workflows, Profiles (visual-gate2), and any future composition primitive. There is no second runtime.

---

## 1. Engine Inputs

```rust
pub struct EngineInput {
    pub workspace:  WorkspaceRef,
    pub workflow:   ResolvedWorkflow,         // pinned versions
    pub input:      Value,                    // matches workflow.input.schema
    pub macros:     MacroBindings,            // resolved macros
    pub slots:      SlotBindings,             // resolved slots
    pub trigger:    Option<TriggerRef>,       // who fired this
    pub policy:     RunPolicy,                // overrides workflow.policy if set
    pub resume_from: Option<RunSnapshot>,     // for `--resume`
}

pub struct RunPolicy {
    pub budget_usd:        Option<f64>,
    pub deadline:          Option<Duration>,
    pub on_module_failure: FailureStrategy,
    pub max_retries:       u32,
    pub human_input_default: HumanInputDefault,
    pub parallelism_cap:   u32,
    pub checkpoint_interval: Duration,
}
```

---

## 2. State Graph Semantics

### 2.1 Node Kinds

Defined in PRD-02 §2.2. Engine semantics per kind:

| Node | Behavior |
|---|---|
| `Module` | Resolve module by ref + version. Build `ModuleInput` (project upstream output via edge mapping; layer macros). Acquire capabilities. Invoke `Module::run`. Capture output, evidence, artifacts, findings, metrics. Emit `NodeStarted` / `NodeCompleted` / `NodeFailed` events. |
| `SubWorkflow` | Recursively invoke engine on a child workflow. Child runs in its own RunId; events bubble up to parent run with parent_run_id breadcrumb. Sub-workflow output projects through edge mappings into parent's downstream nodes. |
| `Branch` | Evaluate `condition` Expr against current run state. Walk only edges whose `condition` evaluates true. |
| `FanOut` | Iterate `over` expression (must yield array). Spawn one child execution per element, capped by `max_parallelism`. Children execute downstream subgraph until next `FanIn`. |
| `FanIn` | Wait for all parallel branches launched by matching `FanOut`. Apply `MergeStrategy` (concat / first-success / all-or-fail / vote). Continue with merged state. |
| `Loop` | Repeat `body` subgraph. Evaluate `until` Expr at each iteration. Bounded by `max_iterations`. Emit `LoopIteration` events. |
| `HumanInput` | Pause run. Emit `HumanInputRequested` event. Wait for `HumanInputReceived` event matching the run+node. Apply timeout if set. |
| `Wait` | Block until `WaitCondition` is satisfied (artifact appears, event received, time elapsed, sub-workflow completes). |
| `Slot` | At run start, slot resolution replaces this node with the user-bound Module / sub-Workflow / inline graph. Engine never sees a raw `Slot` at runtime. |
| `Noop` | Pass-through. Used as a synchronization point. |

### 2.2 Edge Conditions

Edges may carry a `condition: Expr`. Engine evaluates conditions in source-node-completion order. Multiple matching edges fan out (parallel by default). Zero matching edges from a non-exit node is a runtime error unless the node is a designated exit.

### 2.3 Expression Language

The Expr language is small, total, and deterministic. Used for edge conditions, loop predicates, fan-out source expressions, macro transforms.

```
expr   := value | binop | unop | call | path | index
value  := bool | int | float | string | null
binop  := "==" | "!=" | "<" | "<=" | ">" | ">="
        | "AND" | "OR" | "+" | "-" | "*" | "/"
        | "in" | "matches"
unop   := "NOT" | "-"
path   := identifier ("." identifier)*
index  := path "[" expr "]"
call   := identifier "(" expr ("," expr)* ")"
```

Built-in functions: `len`, `first`, `last`, `flatten`, `unique`, `sort`, `lower`, `upper`, `now`, `severity_max`, `severity_at_least`, `count_where`, `any`, `all`.

Variables in scope:
- `input` — workflow input
- `output` — last completed node's output
- `<node-id>` — any prior completed node's output by id
- `macros` — resolved macros
- `slots` — resolved slot fillings
- `run` — run-level metadata (id, started_at, elapsed)
- `audit` — sugar for the `audit` node when present (convention)

Expr evaluation has a 100ms timeout per call; long evaluation is a bug, not a feature.

---

## 3. Failure Strategies

```rust
pub enum FailureStrategy {
    Fail,                                     // any module failure fails the run
    Retry        { max: u32, backoff: Backoff },
    RetryWithEscalation,                      // retry; on retry, escalate model tier
    Decompose,                                // ask the workflow author to split the failing module
    Skip         { mark: bool },              // continue past failure; mark output as skipped
    Compensate   { compensator: ModuleRef },  // run a cleanup module then continue
    Replan,                                   // engine asks a planner module to revise the rest of the graph
    HumanResolve,                             // pause for human decision
}

pub enum Backoff {
    Constant     { ms: u64 },
    Exponential  { base_ms: u64, factor: f64, max_ms: u64, jitter: bool },
}
```

`RetryWithEscalation` is the default for LLM-based modules: first attempt with the configured model; on retry, escalate to the next tier in the cascade router (e.g., haiku → sonnet → opus).

`Replan` is the powerful one: when a module fails in a way that isn't retriable (logical impasse, schema violation, contradictory output), the engine invokes a designated `planner` module with the current state graph, the failure, and a request to revise. The revised subgraph replaces the failing portion. This is how the system self-heals long-running pipelines.

---

## 4. Resumability

Every run produces a snapshot at every node completion (configurable via `RunPolicy::checkpoint_interval` to throttle for very fast modules). Snapshots persist to `<workspace>/.roko/runs/<run-id>/snapshot.json`.

```rust
pub struct RunSnapshot {
    pub run_id:        RunId,
    pub workflow:      ResolvedWorkflowRef,
    pub input:         Value,
    pub macros:        MacroBindings,
    pub slots:         SlotBindings,
    pub completed:     Vec<NodeCompletion>,    // per-node output, evidence, artifacts
    pub in_flight:     Vec<NodeId>,            // running at snapshot time
    pub queued:        Vec<NodeId>,            // ready-to-run
    pub blocked:       Vec<BlockedNode>,       // awaiting human input / wait conditions
    pub artifacts:     Vec<ArtifactRef>,       // produced so far
    pub events_offset: u64,                    // cursor into run-events.jsonl
    pub policy:        RunPolicy,
    pub trigger:       Option<TriggerRef>,
    pub started_at:    DateTime<Utc>,
    pub last_checkpoint_at: DateTime<Utc>,
}
```

`roko run <workflow> --resume <run-id>` (or dashboard "Resume" button) reloads the snapshot and continues from the queued nodes. In-flight nodes at snapshot time are restarted from scratch (Modules must be idempotent or carry their own checkpointing).

A failed run can be retried from the failing node onward (`--retry-from <node-id>`) without re-running upstream nodes.

---

## 5. Cancellation

A run may be cancelled at any time:
- **External** — `roko run cancel <run-id>` or dashboard cancel button.
- **Budget** — `BudgetExceeded` triggers cancellation.
- **Deadline** — `Deadline` triggers cancellation.
- **Workspace lock** — workspace enters maintenance mode.
- **Trigger replacement** — `cancel-running` concurrency policy on a re-firing trigger.

The engine propagates cancellation via a `CancellationToken` shared into every Module's `ModuleContext`. Modules check periodically and return `ModuleError::Cancelled`. The engine then runs any registered compensators, persists final state as `Cancelled`, and exits.

---

## 6. Budget Enforcement

```rust
pub struct BudgetTracker {
    pub usd_limit:     Option<f64>,
    pub usd_spent:     f64,
    pub warn_at_pct:   f32,                   // emit BudgetWarn at this fraction
    pub strategy:      BudgetExceedStrategy,
}

pub enum BudgetExceedStrategy {
    Cancel,                                   // fail run immediately
    SkipOptional,                             // skip remaining "optional" nodes (e.g., enrich)
    Downgrade,                                // re-route remaining LLM calls to cheaper tier
    HumanInput,                               // pause and ask human whether to continue
}
```

Modules that incur cost call `ctx.budget.charge(cost)` after each chargeable operation. The tracker reduces from the limit. At thresholds, events fire; at exhaustion, the strategy executes.

---

## 7. Human-in-Loop

`HumanInput` nodes are first-class. When the engine reaches one, it:

1. Persists current state.
2. Emits `HumanInputRequested` with prompt, schema, and timeout.
3. The dashboard, TUI, or CLI surfaces the prompt to the user.
4. The user provides input via dashboard form, TUI prompt, or `roko run respond <run-id> --node <node-id> --input <json>`.
5. Engine receives `HumanInputReceived`, validates against schema, resumes.

Timeout behavior is configurable:
- `Cancel` — timeout aborts the run.
- `Default <value>` — timeout uses a default value and continues.
- `Skip` — timeout skips the node.
- `Escalate` — timeout pings on a different channel (Slack, email).

Human-input nodes are persisted across daemon restarts: a daemon that comes up after a crash sees pending human-input requests and continues serving them.

---

## 8. Concurrency & Parallelism

The engine is fully async (`tokio`). Nodes execute concurrently when:
- Multiple edges from a `Branch` evaluate true.
- A `FanOut` spawns child branches.
- The state graph has natural parallelism (multiple independent subtrees).

Parallelism is bounded by:
- `RunPolicy::parallelism_cap` — global cap per run.
- `FanOut::max_parallelism` — per-fan-out cap.
- Per-Module concurrency cap (declared by Module).
- Workspace-level `max_concurrent_runs` (across all triggers, all workflows).

The engine uses a central scheduler (tokio task pool) so total in-flight Modules across all runs is bounded.

---

## 9. Episode Logging

Every Module run produces an `Episode` written to `<workspace>/.roko/episodes.jsonl`:

```rust
pub struct Episode {
    pub episode_id:   EpisodeId,
    pub run_id:       RunId,
    pub workflow:     WorkflowRef,
    pub node_id:      NodeId,
    pub module:       ModuleRef,
    pub input:        Value,                   // truncated if large
    pub output:       Value,
    pub model:        Option<ModelRef>,
    pub temperature:  Option<f32>,
    pub tokens_in:    Option<u32>,
    pub tokens_out:   Option<u32>,
    pub usd_cost:     f64,
    pub wall_ms:      u64,
    pub retries:      u32,
    pub findings:     Vec<Finding>,
    pub success:      bool,
    pub timestamp:    DateTime<Utc>,
    pub hdc_fingerprint: HdcVector,            // for resonance
}
```

Episodes feed the existing `roko-learn` infrastructure: cascade router updates, prompt experiments, efficiency tracking. The new dimension: per-Workflow learning. The cascade router selects models *per Module per Workflow*, so the synthesizer in `doc-ingest` can have a different cost/quality trade-off than the synthesizer in `prd-draft`.

---

## 10. Cascade Router Integration

`ctx.model_router` resolves model selection at Module-call time:

```rust
let model = ctx.model_router
    .select(role: "strategist",
            module: self.name(),
            workflow: ctx.workflow.name,
            difficulty_hint: input.difficulty(),
            budget_remaining: ctx.budget.remaining())
    .await?;
```

The router consults:
1. Workspace-level model defaults from `workspace.toml`.
2. Workflow-level overrides from macros.
3. Per-(role, module, workflow) success-rate history from episodes.
4. Cost/quality Pareto from the bandit state.

This is how "more workflows → more synergy" expresses itself in routing: every Workflow's runs improve model selection across all related Workflows.

---

## 11. Run Storage Layout

```
<workspace>/.roko/runs/<run-id>/
├── snapshot.json              # latest snapshot
├── snapshot.<seq>.json        # historical snapshots (retention configurable)
├── input.json
├── output.json                # populated on completion
├── events.jsonl               # full event stream
├── artifacts/                 # produced artifacts (or symlinks to artifact store)
│   └── art_<id>
├── episodes/                  # episodes for this run
│   └── ep_<id>.json
└── manifest.json              # status, started/ended, error if any
```

Retention is per-workspace policy (default: keep last 1000 runs, GC older).

---

## 12. Cost / Time Estimation

Before execution, the engine produces a cost-time estimate by walking the state graph and summing `Module::estimate_cost`. The estimate informs:
- Pre-run UI confirmation (dashboard shows expected cost).
- Budget validation (warn if estimate > limit).
- ETA display in TUI / dashboard during execution.

Estimates are stored alongside the run; deviations between estimate and actual feed back into Module estimators (a learning loop).

---

## 13. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Linear workflow runs end-to-end with all events emitted in order. | Integration test on a 3-module sequential workflow. |
| Conditional edge: condition evaluation routes correctly. | Test fixture with branching condition. |
| FanOut/FanIn: parallel branches with merged output via `concat` strategy. | Test on 3-way parallel research. |
| Loop terminates at `until` predicate satisfaction; respects `max_iterations`. | Two tests: predicate met early, predicate never met (cap kicks in). |
| Sub-workflow: parent + child events visible in unified timeline. | Nested workflow test. |
| HumanInput: run pauses, resumes after `respond` invocation. | Async integration test. |
| Resume: kill engine mid-run; resume from snapshot; result identical to non-killed run. | Property test on idempotent workflow. |
| Cancellation: external cancel propagates to in-flight modules within 5s. | Slow module + cancel; module reports cancelled. |
| Budget enforcement: `Cancel` strategy aborts on overage; `Downgrade` re-routes. | Two tests, one per strategy. |
| Episodes written for every Module run, with correct cost / token / model fields. | Verify count matches node count. |
| Cascade router queries succeed and reflect prior episodes. | Bandit state snapshot test. |

---

## 14. Open Questions

- Should the engine support speculative execution (start downstream nodes before upstream is done, cancel if upstream output invalidates)? Powerful but complex; defer.
- Should there be a "shadow run" capability (run a candidate workflow alongside production, compare outputs, never persist shadow artifacts)? Useful for A/B; specify in marketplace PRD if shipped.
- Resource limits per Module (CPU, memory, file handles)? Likely yes via cgroups for native; tokio for cooperative; WASM has fuel; scripts get OS-level limits via launcher. Specify in v1.1.
- Multi-machine engine for very large fan-outs? Out of scope for v1.
