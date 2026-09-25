# Cell Composition Patterns

> Pipeline, fan-out, conditional routing, and the concrete Graph topologies
> used in production. Every computation in Roko is a Graph of Cells.

**Sources**: `crates/roko-graph/src/types.rs`, `crates/roko-graph/src/cells/`

---

## 1. Principle: No Inheritance, No Middleware

Cells compose by wiring them into directed acyclic Graphs. This is the
only composition mechanism. There are no inheritance hierarchies, no
middleware stacks, no plugin chains. A `Graph` is a
`petgraph::DiGraph<Node, Edge>` where each `Node` references a
`cell_type` name resolved through the `CellRegistry`.

---

## 2. Graph Primitives

### Node

```rust
pub struct Node {
    pub id: NodeId,                    // unique within this graph
    pub cell_type: String,            // CellRegistry lookup key
    pub config: toml::Value,          // passed to the factory
    pub inputs: Vec<String>,          // named inputs from upstream
    pub outputs: Vec<String>,         // named outputs for downstream
    pub execution_class: ExecutionClass, // Workflow or Activity
}
```

`ExecutionClass` determines snapshot behavior:
- `Workflow` -- deterministic (routing, scoring, composition); re-derived
  on replay, not recorded in snapshots.
- `Activity` (default) -- non-deterministic (LLM calls, tool execution);
  outputs are recorded so replay reproduces results without re-invoking
  external systems.

### Edge

```rust
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub condition: Option<EdgeCondition>,
}
```

### EdgeCondition

```rust
pub enum EdgeCondition {
    Success,                       // source succeeded
    Failure,                       // source failed
    OutputEquals { key, value },   // named output matches value
    Always,                        // unconditional dependency
}
```

`None` condition means unconditional (same as `Always`). The engine
evaluates conditions after each node execution and only follows edges
whose conditions are met.

---

## 3. Pattern: Linear Pipeline

The simplest pattern. Each Cell feeds its output as input to the next.

```
A -> B -> C -> D
```

**Example**: Plan execution pipeline

```
TaskContext -> Enrichers -> PlanCompose -> TaskExecutor -> PlanGate
```

| Cell | Registry key | Role |
|---|---|---|
| `TaskContextCell` | `plan.task-context` | Assemble task metadata and predecessor state |
| (Enrichers) | (multiple) | Fan-in of 6 context enrichers |
| `PlanComposeCell` | `plan.compose` | Combine enriched context into a single prompt |
| `TaskExecutorCell` | `plan.task-executor` | Dispatch to LLM provider via `TaskDispatcher` |
| `PlanGateCell` | `plan.gate` | Run compile/lint/test rungs |

This is `ProductionPlanTopology` in `roko-graph`.

---

## 4. Pattern: Fan-In

Multiple upstream Cells feed into a single downstream Cell. The
downstream Cell receives the union of all upstream outputs as its input.

```
A ──┐
B ──┼──> D
C ──┘
```

**Example**: PlanComposeCell receives inputs from 6 enrichers plus the
task context cell:

```
TaskContext ───────────┐
EpisodeEnricher ──────┐│
PlaybookEnricher ────┐││
KnowledgeEnricher ──┐│││
CostEnricher ──────┐││││
AffectEnricher ───┐│││││
                  ├┤│││└──> PlanCompose
                  │└┘││
                  └──┘│
                     └┘
```

Each enricher produces context Signals that PlanCompose merges under a
token budget.

---

## 5. Pattern: Fan-Out

One upstream Cell feeds multiple downstream Cells that execute in
parallel within the same wave.

```
       ┌──> B
A ─────┤
       └──> C
```

The engine bounds parallel execution within a wave via
`GraphPolicy::max_concurrent_nodes` (default 4).

---

## 6. Pattern: Conditional Routing

Edges with `EdgeCondition` create branching execution paths. Different
downstream Cells execute depending on the upstream result.

```
           Success ──> B
A ─────┤
           Failure ──> C
```

**Example**: Corrigibility pipeline -- each cell stops the pipeline on
veto via conditional `Success` edges:

```
Deference ──Success──> Switch ──Success──> Truth ──Success──> Impact ──Success──> Task
     |                   |                   |                   |
     └──Failure──>       └──Failure──>       └──Failure──>       └──Failure──>
         (halt)               (halt)              (halt)              (halt)
```

If any cell returns a veto (Failure), execution stops. Only `Success`
edges propagate forward.

### OutputEquals Routing

```
         key=model,value=claude ──> ClaudeDispatch
Router ──┤
         key=model,value=gpt ────> GptDispatch
```

`OutputEquals { key, value }` checks a named output field against an
expected value. This enables data-driven routing where the upstream Cell
writes a classification into its output.

---

## 7. Pattern: Safety Pipeline (Corrigibility)

The five-head corrigibility verification Graph. Each Cell is a
`ProtocolId::Verify` implementation that checks a specific aspect of
safety, in strict priority order:

```
Deference -> Switch -> Truth -> Impact -> Task
```

| Cell | Registry key | Checks |
|---|---|---|
| `VerifyDeferenceCell` | `security.verify.deference` | Highest priority: deference to operator |
| `VerifySwitchCell` | `security.verify.switch` | Kill-switch and override compliance |
| `VerifyTruthCell` | `security.verify.truth` | Truthfulness and honesty |
| `VerifyImpactCell` | `security.verify.impact` | Impact assessment and harm prevention |
| `VerifyTaskCell` | `security.verify.task` | Task-specific constraints |

All edges are `Success`-conditional. A veto at any stage stops the
pipeline. The ordering enforces that deference is always checked first,
regardless of task-specific policies.

---

## 8. Pattern: Immune Pipeline

Five-stage cognitive immune system, also using `Success` edges:

```
Perception -> Assessment -> Containment -> Validation -> Escalation
```

| Cell | Registry key | Stage |
|---|---|---|
| `ImmunePerceptionCell` | `security.immune.perception` | Detect potential threats |
| `ImmuneAssessmentCell` | `security.immune.assessment` | Assess severity |
| `ImmuneContainmentCell` | `security.immune.containment` | Contain the threat |
| `ImmuneValidationCell` | `security.immune.validation` | Validate containment |
| `ImmuneEscalationCell` | `security.immune.escalation` | Escalate if needed |

Versioned state (`ImmuneCellState`) rejects skipped or reordered stages,
enforcing the strict sequential invariant.

---

## 9. Pattern: Cognitive Loop (Hot Graph)

Seven typed Cells forming a cyclic execution pattern with a T0
short-circuit:

```
Sense -> Assess -> Compose -> Act -> Verify -> Persist -> React
  ^                                                         |
  |--- T0 short-circuit (skip middle, go to React) --------|
```

| Cell | Registry key | Protocol | Role |
|---|---|---|---|
| `SenseCell` | `signal-reader` | Observe | Read new Signals/Pulses |
| `AssessCell` | `relevance-scorer` | Score | Score and rank material |
| `CognitiveComposeCell` | `cognitive-composer` | Compose | Assemble prompt under budget |
| `ActCell` | `agent-dispatcher` | -- | Dispatch to LLM provider |
| `VerifyCell` | `verification-gate` | Verify | Post-action verification |
| `PersistCell` | `result-persister` | Store | Write results to durable store |
| `ReactCell` | `policy-reactor` | React | Maintenance, events, force-full-tick requests |

The T0 short-circuit is an `OutputEquals` edge from SenseCell: when no
new Signals or Pulses are detected, the middle five cells are skipped
and React is invoked directly for maintenance.

---

## 10. Edge Validation

`Graph::validate_edges(registry)` checks type compatibility between
connected nodes using `CellDescriptor` introspection. No Cell
construction occurs -- validation uses the side-effect-free descriptors.

Rules:
1. Missing registry entries produce errors.
2. Untyped cells (`None` schema) are always valid.
3. Typed cells must have compatible schemas via
   `TypeSchema::is_compatible_with`.
4. All errors are collected (no fail-fast).

---

## 11. Graph Execution Policy

```rust
pub struct GraphPolicy {
    pub max_concurrent_nodes: usize,   // default 4
    pub timeout: Duration,
    pub failure_strategy: FailureStrategy,
}

pub enum FailureStrategy {
    FailFast,          // stop on first node failure
    ContinueOnError,   // run remaining nodes
}
```

The engine executes nodes in topological order, grouping independent
nodes into waves. Within each wave, up to `max_concurrent_nodes` cells
run in parallel. Wave context (`wave_index`, `total_waves`) is injected
into the Graph CellContext.

---

## 12. Verification Commands

```bash
# List all graph cell registrations
grep -rn 'register\|cell_type' crates/roko-graph/src/cells/ --include='*.rs' | head -30

# Run graph composition tests
cargo test -p roko-graph -- --nocapture

# Validate edge types
cargo test -p roko-graph validate_edges -- --nocapture
```

---

## 13. Source Files

| File | Contents |
|---|---|
| `crates/roko-graph/src/types.rs` | Graph, Node, Edge, EdgeCondition, GraphPolicy |
| `crates/roko-graph/src/cells/corrigibility.rs` | 5 corrigibility cells |
| `crates/roko-graph/src/cells/immune.rs` | 5 immune cells |
| `crates/roko-graph/src/cells/cognitive.rs` | 7 cognitive loop cells |
| `crates/roko-graph/src/cells/plan_compose.rs` | PlanComposeCell (fan-in) |
| `crates/roko-graph/src/cells/plan_gate.rs` | PlanGateCell |
| `crates/roko-graph/src/cells/task_executor.rs` | TaskExecutorCell |
| `crates/roko-graph/src/cells/task_context.rs` | TaskContextCell |
