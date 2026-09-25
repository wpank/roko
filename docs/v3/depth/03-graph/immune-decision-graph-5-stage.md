# Five-Stage Immune Decision Graph

> Every host-visible tool result and canonical provider primary output traverses
> a fixed, linear, fail-closed immune pipeline implemented as five Graph Cells.
> This file documents the pipeline structure, state machine, each stage, the
> runtime graph wrapper, and the safety invariants.

---

## Source File

`crates/roko-graph/src/cells/immune.rs`

---

## Pipeline Topology

```
  [Perception] --> [Assessment] --> [Containment] --> [Validation] --> [Escalation]
```

Five typed, independently executable Graph Cells connected by unconditional
edges. The pipeline is strictly sequential (`max_concurrent_nodes = 1`).

---

## Stage Inventory

| # | Stage | Cell Type | Protocol | Input State | Output State |
|---|---|---|---|---|---|
| 1 | Perception | `security.immune.perception` | `Observe` | `Request` | `Perceived` |
| 2 | Assessment | `security.immune.assessment` | `Score` | `Perceived` | `Assessed` |
| 3 | Containment | `security.immune.containment` | `React` | `Assessed` | `Contained` |
| 4 | Validation | `security.immune.validation` | `Verify` | `Contained` | `Validated` |
| 5 | Escalation | `security.immune.escalation` | `Route` | `Validated` | `Complete` |

All five stages are `ExecutionClass::Workflow` (deterministic) and produce
identical results to the pure-transform `ImmunePipeline` (verified by test
parity).

---

## ImmuneCellState

The versioned state carried between stages is a tagged enum:

```rust
#[serde(tag = "stage", content = "state", rename_all = "snake_case")]
pub enum ImmuneCellState {
    Request {
        target: ContentHash,
        anomaly: AnomalyScore,
        affected_signals: Vec<ContentHash>,
    },
    Perceived {
        perception: ImmunePerception,
        affected_signals: Vec<ContentHash>,
    },
    Assessed {
        assessment: ImmuneAssessment,
        affected_signals: Vec<ContentHash>,
    },
    Contained(ImmuneContainment),
    Validated(ImmuneValidation),
    Complete(ImmunePipelineResult),
}
```

Each stage accepts exactly one versioned state variant and rejects any other.
This is enforced by pattern-matching:

```rust
// Example: ImmuneValidationCell
match state {
    ImmuneCellState::Contained(containment) => {
        ImmuneCellState::Validated(pipeline.validate(containment))
    }
    other => Err("ImmuneValidation expected contained state, got {other}")
}
```

Feeding a `Request` state to `ImmuneValidationCell` fails with "expected
contained state." This prevents stage skipping even if a caller attempts
to reuse a later Cell outside its canonical predecessor chain.

---

## Fail-Closed Semantics

**Safety invariants:**

1. Each stage accepts exactly one versioned `ImmuneCellState` variant.
   Out-of-order, malformed, skipped, duplicated, or reordered state is
   rejected.

2. The pipeline runs with `max_concurrent_nodes = 1` (strictly sequential).

3. The graph labels declare:
   - `security.fail_closed = true`
   - `security.effect_free = true`

4. Each cell requires exactly one input Signal. Multiple or zero inputs
   return `RokoError::Invalid`.

5. The pure-transform `ImmunePipeline` (from `roko-core`) produces identical
   results to the runtime Graph execution. This is verified by the
   `runtime_graph_executes_all_five_stages_with_pure_transform_parity` test.

---

## Theoretical Foundation

The five-stage pipeline is inspired by the Artificial Immune System (AIS)
literature, particularly:

- **de Castro, L. N. & Timmis, J. (2002).** *Artificial Immune Systems: A New
  Computational Intelligence Approach.* Springer. The AIS model structures
  immune response as perception (antigen recognition), assessment (affinity
  evaluation), response (clonal selection/containment), validation (memory
  cell verification), and escalation (adaptive immune activation).

The five stages map to AIS concepts:

| Roko Stage | AIS Concept | Function |
|---|---|---|
| Perception | Antigen recognition | Identify the anomalous Signal |
| Assessment | Affinity evaluation | Score threat severity |
| Containment | Clonal selection / response | Quarantine or isolate |
| Validation | Memory cell verification | Check collateral safety |
| Escalation | Adaptive immune activation | Decide whether to escalate to operator |

---

## Stage Details

### 1. Perception (`ImmunePerceptionCell`)

Accepts a `Request` containing:
- `target: ContentHash` -- the Signal under review.
- `anomaly: AnomalyScore` -- immutable evidence from the boundary detector.
- `affected_signals: Vec<ContentHash>` -- proposed containment scope.

Produces a `Perceived` state with an `ImmunePerception` record.
The `affected_signals` are carried through without mutation.

### 2. Assessment (`ImmuneAssessmentCell`)

Scores the perceived threat. Maps the `AnomalyScore` dimensions into a
deterministic `ThreatSeverity` classification:

| Score Range | Severity |
|---|---|
| `score >= critical_threshold` (default 0.95) | Critical |
| `score >= quarantine_threshold` (default 0.8) | High |
| `score >= 0.5` | Medium |
| `score < 0.5` | Low |

Produces an `Assessed` state with an `ImmuneAssessment`.

### 3. Containment (`ImmuneContainmentCell`)

Decides the containment action based on assessment severity and affected scope:

| Severity | Action |
|---|---|
| Critical | `ResponseAction::IsolateAgent` |
| High | `ResponseAction::IsolateAgent` |
| Medium with affected signals | `QuarantineDecision::Quarantine` |
| Low | `QuarantineDecision::Pass` |

Produces a `Contained` state with `ImmuneContainment` including:
- `decision: QuarantineDecision` (Pass or Quarantine).
- `action: Option<ResponseAction>`.

### 4. Validation (`ImmuneValidationCell`)

Validates that containment is safe. Checks collateral scope:
- If `affected_signals` is non-empty and a Quarantine decision was made,
  `collateral_safe = false`.
- Otherwise `collateral_safe = true`.

Produces a `Validated` state with `ImmuneValidation`.

### 5. Escalation (`ImmuneEscalationCell`)

Decides whether operator escalation is required:
- `escalation_required = true` when:
  - Severity is High or Critical, OR
  - Collateral is not safe.

Produces a `Complete` state with `ImmunePipelineResult` (the final output).
When a result trace is attached, the completion is captured for the graph
wrapper's return value.

---

## Runtime Graph Wrapper: `ImmunePipelineGraph`

```rust
pub struct ImmunePipelineGraph {
    pipeline: ImmunePipeline,
}

impl ImmunePipelineGraph {
    pub fn new(quarantine_threshold: f64, critical_threshold: f64) -> Self;
    pub async fn screen(
        &self,
        target: ContentHash,
        anomaly: AnomalyScore,
        affected_signals: Vec<ContentHash>,
    ) -> Result<ImmuneGraphOutput, GraphError>;
}
```

`screen()` is the runtime entry point:

1. Constructs the five-node Graph via `immune_pipeline_graph()`.
2. Creates a traced registry where the EscalationCell captures its output.
3. Injects the screening request as a root Signal.
4. Executes via `GraphEngine::new(graph, registry).execute()`.
5. Returns `ImmuneGraphOutput` with both the pipeline result and per-stage
   execution evidence.

### ImmuneGraphOutput

```rust
pub struct ImmuneGraphOutput {
    pub result: ImmunePipelineResult,   // validated containment and escalation
    pub graph:  GraphOutput,             // per-stage execution evidence
}
```

---

## Cell Registration

`register_immune_cells(registry)` registers all five cell factories with
`CellDescriptor` metadata:

- All five cells share the same input/output `TypeSchema`:
  `TypeSchema::JsonSchema("roko.security.immune_cell_state.v1")`.
- All cells are version `(1, 0, 0)`.
- Edge validation can check type compatibility without constructing cells.

The `default_registry()` pre-registers all five immune cells alongside the
seven cognitive cells and other built-in cells.

---

## Cell Implementation: Macro

All five immune cells are generated by the `define_immune_cell!` macro, which:

1. Creates a struct with `pipeline: ImmunePipeline`, `result_trace: Option`,
   and input/output schemas.
2. Implements `Cell` with the correct `cell_id`, `cell_name`, `cell_version`,
   `protocols`, and schemas.
3. In `execute()`:
   a. Decodes the single input Signal into `ImmuneCellState`.
   b. Pattern-matches on the expected variant.
   c. Calls the corresponding `ImmunePipeline` method.
   d. If the result is `Complete` and a trace is attached, captures it.
   e. Encodes the next state into a new Signal with `immune_stage` tag.

---

## Graph Construction

`immune_pipeline_graph()` builds the canonical five-node linear graph:

```rust
let mut graph = Graph::new(GraphMetadata {
    name: "immune-pipeline".to_string(),
    description: Some("Five-stage cognitive immune safety pipeline"),
    labels: HashMap::from([
        ("security.fail_closed", "true"),
        ("security.effect_free", "true"),
    ]),
});
graph.policy = GraphPolicy {
    max_concurrent_nodes: 1,  // strictly sequential
    ..Default::default()
};
// Add 5 nodes, 4 edges (linear chain)
```

All nodes are `ExecutionClass::Workflow`. Each node declares `inputs: ["state"]`
and `outputs: ["state"]`.

---

## Verification

```bash
cargo test -p roko-graph --lib cells::immune::tests
```

Test coverage:
- Default registry hosts all five immune cells.
- Runtime graph executes all five stages with pure-transform parity.
- Collateral scope is detected before escalation.
- Out-of-order immune stage fails closed ("expected contained state").
- Malformed immune input fails closed (non-JSON body).
