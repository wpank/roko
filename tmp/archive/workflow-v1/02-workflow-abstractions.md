# PRD-02 — Workflow Abstractions

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Crate**: `roko-workflow` (new, kernel)
**Prerequisites**: PRD-00, PRD-01

---

## 0. Scope

This document defines the kernel of the workflow subsystem: the trait definitions, type system, and contracts that every other PRD in this set builds on. Everything lives in `crates/roko-workflow/`. Dependencies are minimal: `roko-core` (for `Engram`, `Score`, `Verdict`, `Context`, `ContentHash`), `roko-fs` (for substrate IO), `serde`, `tokio`. No higher-level crate depends on this except via these public types.

The five primitives:

1. **`Module`** — smallest unit of work; takes typed input, produces typed output, declares capabilities and required evidence.
2. **`Workflow`** — composition of Modules wired into a state graph; carries Macros and Slots.
3. **`Artifact`** — content-addressed, versioned, lineage-tracked output.
4. **`Macro`** — a promoted Workflow parameter exposed to consumers.
5. **`Slot`** — a typed empty position in a Workflow consumers fill in.

`Trigger` is a peer primitive but is fully specified in PRD-03; this PRD defines only the trigger-binding interface.

---

## 1. The Module Trait

### 1.1 Design Motivation

A Module is the smallest piece of orchestrated work — small enough that 12 of them composed produce real value, large enough that a single LLM call or a single shell command counts as one. Modules are explicitly typed at their inputs and outputs so the engine can validate compositions before execution and so the visual editor (PRD-11) can render typed cables between them.

Three properties:
1. **Typed I/O** — Module declares input/output types; the engine refuses to wire mismatched types unless an adapter exists.
2. **Declared capabilities** — Module declares what it needs (`fs.read`, `net`, `llm`, `shell`, `chain.write`); the workspace grants or denies; failures are clean.
3. **Pure-by-default** — Modules are encouraged to be deterministic given inputs; non-determinism (LLM calls, network) is explicit and observable.

### 1.2 The Trait

```rust
/// A unit of work composable into Workflows.
///
/// Modules are the lego pieces. Implementations can be Rust (trait impls in-tree
/// or in plugin crates), WASM (sandboxed user extensions), scripts (bash/python/
/// node with declared capabilities), or pure TOML (compositions of other Modules).
#[async_trait]
pub trait Module: Send + Sync {
    /// Stable identifier across versions. `kebab-case`.
    fn name(&self) -> &str;

    /// Semver of this Module implementation.
    fn version(&self) -> &Version;

    /// Human-readable description for catalogs and search.
    fn description(&self) -> &str;

    /// Tags for filtering and discovery.
    fn tags(&self) -> &[&str] { &[] }

    /// Type tag identifying the input shape.
    fn input_schema(&self) -> &TypeSchema;

    /// Type tag identifying the output shape.
    fn output_schema(&self) -> &TypeSchema;

    /// Capabilities this module requires to run. Engine fails closed when
    /// the workspace has not granted a required capability.
    fn capabilities(&self) -> &[Capability];

    /// Optional: evidence kinds required (per visual-gate2 PRD-02).
    /// If non-empty, the engine ensures these are present in the EvidenceBag
    /// before invoking the module.
    fn required_evidence(&self) -> &[EvidenceKind] { &[] }

    /// Optional: declared cost estimator. Used for budget enforcement and
    /// ETA display. Returns expected USD + expected wall-clock seconds.
    fn estimate_cost(&self, input: &ModuleInput) -> CostEstimate {
        CostEstimate::unknown()
    }

    /// Run the module. The engine handles retry, cancellation, episode logging,
    /// and capability checking around this call.
    async fn run(
        &self,
        input: ModuleInput,
        ctx: &ModuleContext,
    ) -> Result<ModuleOutput, ModuleError>;
}
```

### 1.3 ModuleInput / ModuleOutput

Inputs and outputs are typed payloads carrying both the shape and the evidence/lineage. `ModuleInput::payload` is a serializable user-facing value; `ModuleInput::evidence` carries upstream evidence the module may consult.

```rust
pub struct ModuleInput {
    pub payload:  Value,                 // serde_json::Value, schema-validated
    pub evidence: EvidenceBag,           // shared evidence from upstream
    pub macros:   MacroBindings,         // resolved Workflow-level macro values
    pub context:  ModuleInputContext,    // run id, workflow id, parent module, etc
}

pub struct ModuleOutput {
    pub payload:    Value,               // schema-validated against output_schema
    pub evidence:   EvidenceBag,         // new evidence this module produced
    pub artifacts:  Vec<Artifact>,       // any persistable artifacts
    pub findings:   Vec<Finding>,        // per visual-gate2 PRD-01
    pub metrics:    ModuleMetrics,       // tokens, cost, wall time, retries
    pub next_state: Option<StateHint>,   // optional hint to engine for routing
}
```

### 1.4 ModuleContext (the runtime hands this to every Module)

```rust
pub struct ModuleContext {
    pub workspace:     WorkspaceRef,
    pub run_id:        RunId,
    pub workflow:      WorkflowRef,
    pub event_bus:     EventBusHandle,
    pub artifact_store: ArtifactStoreHandle,
    pub knowledge:     KnowledgeStoreHandle,
    pub model_router:  ModelRouterHandle,    // for selecting models per role
    pub shell:         ShellHandle,           // capability-gated
    pub net:           NetHandle,             // capability-gated
    pub fs:            FsHandle,              // capability-gated
    pub llm:           LlmHandle,             // capability-gated
    pub cancel:        CancellationToken,
    pub deadline:      Option<Instant>,
    pub budget:        BudgetTracker,
    pub episode:       EpisodeRecorder,
    pub trace:         TraceSpan,
}
```

The handles inside `ModuleContext` are gated by the capabilities the Module declared. Calling `ctx.net.fetch(...)` from a Module that did not declare `Capability::Net` panics in debug, errors in release.

---

## 2. The Workflow Type

A Workflow is data, not a trait. It is a TOML-defined, serializable, persistable composition of Modules. The engine (PRD-05) interprets it.

### 2.1 The Type

```rust
pub struct Workflow {
    pub identity:    WorkflowIdentity,
    pub description: String,
    pub tags:        Vec<String>,
    pub macros:      Vec<MacroDef>,
    pub slots:       Vec<SlotDef>,
    pub graph:       StateGraph,
    pub policy:      WorkflowPolicy,
    pub schema:      WorkflowSchema,
}

pub struct WorkflowIdentity {
    pub name:      String,                  // kebab-case, unique
    pub version:   Version,                 // semver
    pub publisher: Option<String>,          // marketplace handle
    pub forked_from: Option<ArtifactRef>,   // lineage
}

pub struct WorkflowSchema {
    pub input:  TypeSchema,                  // workflow-level input
    pub output: TypeSchema,                  // workflow-level output
}
```

### 2.2 StateGraph

The execution topology. Detailed in PRD-05 §2; summarized here for completeness.

```rust
pub struct StateGraph {
    pub nodes: Vec<StateNode>,
    pub edges: Vec<StateEdge>,
    pub entry: NodeId,
    pub exits: Vec<NodeId>,                  // multiple terminal states allowed
}

pub enum StateNode {
    Module    { id: NodeId, module: ModuleRef, params: Value },
    SubWorkflow { id: NodeId, workflow: WorkflowRef, params: Value },
    Branch    { id: NodeId, condition: Expr },        // conditional fan-out
    FanOut    { id: NodeId, over: Expr, max_parallelism: usize },
    FanIn     { id: NodeId, strategy: MergeStrategy },
    Loop      { id: NodeId, body: NodeId, until: Expr, max_iterations: u32 },
    HumanInput { id: NodeId, prompt: String, schema: TypeSchema, timeout: Option<Duration> },
    Wait      { id: NodeId, until: WaitCondition },   // for trigger-driven joins
    Slot      { id: NodeId, slot_ref: SlotRef },      // resolved at run start
    Noop      { id: NodeId },
}

pub struct StateEdge {
    pub from:      NodeId,
    pub to:        NodeId,
    pub condition: Option<Expr>,           // None = unconditional
    pub maps:      Vec<Mapping>,           // input projection from upstream output
}
```

### 2.3 Composition: Sub-Workflows

A `StateNode::SubWorkflow` references another Workflow by `name@version` and runs it as a node. The parent's state graph waits for the sub-workflow to terminate. Sub-workflow inputs/outputs are mapped via `Mapping` declarations on the incoming and outgoing edges. This is how workflows compose — it is not a wrapper, it is the same engine recursing.

### 2.4 Visual-Gate2 Integration

A `Profile` from visual-gate2 is realized as a Workflow with:
- Entry node: a `FanOut` over EvidenceCollectors (parallel evidence gathering).
- Body nodes: `Module` nodes for each Criterion.
- Exit node: a `FanIn` aggregating CriterionResults into a `Verdict`.

The engine treats Profiles indistinguishably from any other Workflow. Audit / quality / visual-gate use cases all use this composition. PRD-06 catalogs them.

---

## 3. Artifact

Every persistable Module / Workflow output is an Artifact: content-addressed, versioned, lineage-tracked.

```rust
pub struct Artifact {
    pub id:          ArtifactId,        // sha256(content)[..16]  base32
    pub kind:        ArtifactKind,      // file, blob, json, markdown, ...
    pub name:        Option<String>,    // human-friendly
    pub mime:        String,
    pub size:        u64,
    pub created_at:  DateTime<Utc>,
    pub workflow:    Option<WorkflowRef>,
    pub module:      Option<ModuleRef>,
    pub run:         Option<RunId>,
    pub source:      Vec<ArtifactRef>,  // upstream artifacts (lineage)
    pub provenance:  Provenance,
    pub storage:     StorageLocator,
    pub tags:        Vec<String>,
}

pub enum ArtifactKind {
    File,
    Markdown,
    Toml,
    Json,
    Diff,
    Image,
    AudioBytes,
    PdfBytes,
    Binary,
    DesignTokens,
    StructuredFinding,
    Custom(String),
}

pub struct Provenance {
    /// For ingested artifacts: which source file / line range / commit.
    pub source_files: Vec<SourceFileRange>,
    /// For LLM-generated: model id, prompt hash, temperature, seed.
    pub generation:   Option<GenerationProvenance>,
    /// For web-fetched: URL, fetch timestamp, http status, content hash.
    pub web_fetch:    Option<WebFetchProvenance>,
    /// Citations the artifact claims to have used.
    pub citations:    Vec<Citation>,
}

pub struct SourceFileRange {
    pub path:        PathBuf,
    pub start_line:  u32,
    pub end_line:    u32,
    pub content_hash: ContentHash,
}
```

Artifacts are written to the workspace ArtifactStore (see `roko-fs`) and indexed by content hash for deduplication. Lineage is queryable: `roko artifact lineage <id>` walks `source[]` recursively.

---

## 4. Macro (Promoted Parameter)

A Macro is a Workflow-level parameter exposed to consumers without requiring them to inspect internals. The DAW Rack Macro analog. A consumer sees a small set of high-level knobs; the Workflow author chose which internal Module parameters those knobs map to.

```rust
pub struct MacroDef {
    pub name:        String,
    pub label:       String,                  // shown in UI
    pub description: String,
    pub kind:        MacroKind,
    pub default:     Value,
    pub bindings:    Vec<MacroBinding>,       // which internal params it sets
}

pub enum MacroKind {
    Boolean,
    Enum     { variants: Vec<String> },
    Integer  { min: i64, max: i64, step: i64 },
    Float    { min: f64, max: f64, step: f64 },
    Text     { pattern: Option<String> },
    Money    { currency: String, max: f64 },
    ModelRef,
    AgentRef,
    SlotRef,                                  // the macro IS the slot's filling
}

pub struct MacroBinding {
    pub target_node:   NodeId,
    pub target_param:  String,                // dotted path
    pub transform:     Option<Expr>,          // optional value-transformation
}
```

A single macro can fan out across multiple internal nodes. Setting `macro.strictness = "high"` might bind to `auditor.threshold = 0.9`, `synthesizer.temperature = 0.3`, and `reviewer.iterations = 3` simultaneously.

---

## 5. Slot (Typed Empty Position)

A Slot is an explicit hole in a Workflow consumers must fill before running. Slots are the composability hinge: a `research-pipeline` Workflow has slots for "Researcher" and "Verifier" — consumers plug in any Module/sub-Workflow whose output type matches the slot's input type, without forking the parent.

```rust
pub struct SlotDef {
    pub name:           String,
    pub label:          String,
    pub description:    String,
    pub accepts:        SlotKind,           // what kind of thing fits
    pub input_schema:   TypeSchema,
    pub output_schema:  TypeSchema,
    pub default_filling: Option<SlotFilling>,
    pub required:       bool,
}

pub enum SlotKind {
    AnyModule,
    AnyWorkflow,
    AnyEvaluation,                          // visual-gate2 Profile
    SpecificTag { tag: String },
    Capability  { capability: Capability }, // any Module with this capability
}

pub enum SlotFilling {
    Module    { module: ModuleRef, params: Value },
    Workflow  { workflow: WorkflowRef, params: Value },
    Inline    { graph: StateGraph },        // ad-hoc fill
}
```

Slots and Macros are orthogonal: a Macro tunes parameters of pre-wired Modules; a Slot lets the consumer plug in the actual Module.

---

## 6. Type System

Workflows pass typed values between nodes. The type system needs to be expressive enough to declare doc / image / code / metrics / structured findings, but simple enough to validate at config-load time.

### 6.1 TypeSchema

JSON-Schema-compatible with workflow-specific extensions:

```rust
pub enum TypeSchema {
    Primitive(PrimitiveType),
    Object  { fields: BTreeMap<String, TypeSchema>, required: Vec<String> },
    Array   { items: Box<TypeSchema>, min: Option<u32>, max: Option<u32> },
    Enum    { variants: Vec<String> },
    Union   { variants: Vec<TypeSchema> },
    Ref     { name: String, version: Option<Version> },  // named registered type
    Artifact { kind: Option<ArtifactKind> },
    Evidence { kinds: Vec<EvidenceKind> },               // visual-gate2 evidence
    Tagged  { tag: String, inner: Box<TypeSchema> },     // newtype semantics
}
```

Named types are registered in the workspace + user level type registry, similar to module/workflow registries. Built-in types ship for common shapes (`MarkdownDoc`, `Citation`, `WebPage`, `Diff`, `TestResult`, etc).

### 6.2 Adapters

When two nodes need to be wired but their types don't exactly match, the engine looks up an adapter Module. Adapters are first-class Modules tagged `kind = "adapter"`. The visual editor (PRD-11) auto-inserts adapters where unambiguous and prompts when ambiguous.

---

## 7. Capabilities

Capabilities form the security model. A Module declares the capabilities it needs; the workspace grants capabilities; the engine intersects.

```rust
pub enum Capability {
    FsRead   { paths: Option<Vec<PathPattern>> },
    FsWrite  { paths: Option<Vec<PathPattern>> },
    Net      { domains: Option<Vec<String>> },
    Shell    { commands: Option<Vec<String>> },
    Llm      { providers: Option<Vec<String>> },
    Chain    { read: bool, write: bool, networks: Option<Vec<String>> },
    Secrets  { keys: Option<Vec<String>> },
    Workspace { read: bool, write: bool },
    KnowledgeRead,
    KnowledgeWrite,
    Process  { kind: ProcessKind },
    Custom   { name: String, params: Value },
}
```

Capability intersection happens at three layers:
- **Module declaration** — what it needs.
- **Workflow allow-list** — what its embedding workflow permits (a workflow can deny a capability its modules need; the workflow then can't use that module).
- **Workspace grant** — what the user has authorized at the workspace level.

A Module may run only when all three layers permit. Marketplace artifacts disclose their full capability tree on install (PRD-12).

---

## 8. Lifecycle Events

Every Module run, Workflow run, edge traversal, and artifact production emits a typed event onto the workspace event bus. Triggers (PRD-03) can subscribe to these. The dashboard streams them. The episode logger persists them.

```rust
pub enum WorkflowEvent {
    RunStarted    { run: RunId, workflow: WorkflowRef, trigger: TriggerRef, input: Value },
    RunCompleted  { run: RunId, output: Value, duration: Duration, cost: Cost },
    RunFailed     { run: RunId, error: WorkflowError },
    RunCancelled  { run: RunId, reason: String },
    NodeStarted   { run: RunId, node: NodeId, module: Option<ModuleRef> },
    NodeCompleted { run: RunId, node: NodeId, output: Value, duration: Duration },
    NodeFailed    { run: RunId, node: NodeId, error: ModuleError, will_retry: bool },
    EdgeTraversed { run: RunId, from: NodeId, to: NodeId, condition: Option<String> },
    ArtifactProduced { run: RunId, node: NodeId, artifact: ArtifactId },
    HumanInputRequested { run: RunId, node: NodeId, prompt: String, schema: TypeSchema },
    HumanInputReceived  { run: RunId, node: NodeId, value: Value },
    BudgetWarn    { run: RunId, used: Cost, limit: Cost },
    BudgetExceeded{ run: RunId, used: Cost, limit: Cost },
    LoopIteration { run: RunId, node: NodeId, iteration: u32 },
}
```

These events are the foundation of the dashboard's real-time view (PRD-10), the TUI's run inspector (PRD-09), the trigger system's chaining (PRD-03), and the learning system's per-run telemetry.

---

## 9. References (Identity Across Contexts)

```rust
pub struct ModuleRef    { pub name: String, pub version: VersionReq }
pub struct WorkflowRef  { pub name: String, pub version: VersionReq }
pub struct ArtifactRef  { pub id: ArtifactId, pub workspace: Option<WorkspaceId> }
pub struct WorkspaceRef { pub id: WorkspaceId, pub name: String }
pub struct RunId(pub Ulid);
pub struct NodeId(pub String);            // local within a graph
pub struct ArtifactId(pub [u8; 16]);
```

Refs use semver requirements (`^1.0`, `~2.3`, `=1.4.7`) for resolution. The registry resolves at workflow-load time and pins the resolved version to the run for reproducibility.

---

## 10. Errors

```rust
pub enum WorkflowError {
    SchemaMismatch     { expected: TypeSchema, got: TypeSchema, at: NodeId },
    CapabilityDenied   { module: ModuleRef, capability: Capability },
    ModuleError        { node: NodeId, source: ModuleError },
    BudgetExceeded     { used: Cost, limit: Cost },
    Cancelled,
    HumanInputTimeout  { node: NodeId, timeout: Duration },
    Cycle              { path: Vec<NodeId> },
    UnresolvedSlot     { slot: String },
    UnresolvedMacro    { macro_name: String },
    AdapterNotFound    { from: TypeSchema, to: TypeSchema, at: NodeId },
    Internal           { source: BoxError },
}

pub enum ModuleError {
    InvalidInput       { reason: String },
    Timeout            { elapsed: Duration },
    Capability         { needed: Capability },
    External           { source: BoxError },        // network, shell, llm
    LogicError         { reason: String },
    Cancelled,
}
```

Errors carry enough structure for the engine to decide retry-vs-escalate (PRD-05) and for the dashboard to render rich error displays (PRD-10).

---

## 11. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `Module` trait, `Workflow` type, `Artifact`, `Macro`, `Slot` exist as public types in `roko-workflow`. | `cargo doc -p roko-workflow` lists them; `cargo check` clean. |
| Workflow load-time validation rejects type mismatches without an adapter. | Unit test: wire `ModuleA: () -> String` to `ModuleB: i32 -> ()` → schema error. |
| Capability intersection: Module + Workflow + Workspace grant must all permit. | Test matrix; module fails closed if any layer denies. |
| Sub-workflow invocation: workflow A calls workflow B; B's events surface in A's run timeline. | Integration test; events captured. |
| Macros bind to multiple internal params; setting one macro updates all. | Unit test on `MacroBinding::resolve`. |
| Slots accept any compatible Module/Workflow; load fails if required slot is empty. | Negative test on workflow load with unresolved slot. |
| Artifact lineage queryable: walking `source[]` returns every upstream artifact. | Integration test; lineage walk on multi-step workflow. |
| Visual-gate2 Profile compiles to a Workflow and runs through the same engine. | Adapter test; existing `LlmJudgeGate` profile produces a `Verdict`. |

---

## 12. Open Questions

- Should Modules be addressable as strongly-typed Rust types (with macros generating impl) or as dynamic trait objects only? Leaning toward both: in-tree Modules are typed; plugin/WASM/script Modules are erased to a JSON-schema interface.
- Should there be a "private" Module visibility that hides from the marketplace and visual editor? Probably yes.
- How do we handle very large artifacts (>100MB) — store inline in the artifact store, or push to S3/Tigris and store a reference? Probably the latter, with a configurable size threshold.
- Should the `StateGraph` permit cycles or only the explicit `Loop` node? Decision: only `Loop`. All other cycles are errors. This keeps the engine analyzable.
