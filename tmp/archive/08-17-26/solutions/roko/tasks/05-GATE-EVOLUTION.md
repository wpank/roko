# Gate Evolution: Unified Evaluation Framework -- Task Breakdown

> Evolve the existing `roko-gate` 7-rung pipeline into a unified evaluation
> framework with typed evidence, composable criteria, LLM judge panels, a
> self-improvement flywheel, community marketplace, and multi-surface dashboard
> integration. 48 tasks across 8 phases.
>
> Sources: `impl/05-GATE-EVOLUTION.md`, `14-GATE-VIZ-01-Core-Abstractions.md`,
> `14-GATE-VIZ-04-Judge-Methodology.md`, `14-GATE-VIZ-05-Self-Improvement-Flywheel.md`,
> codebase analysis

---

## Overview

The gate pipeline (`crates/roko-gate/`) is the verification backbone. It works: 7 rungs,
adaptive thresholds, SPC detectors, composition modes. But it has three structural
limitations that block the next tier of quality:

| Limitation | Impact |
|---|---|
| Every gate spawns its own subprocess | Evidence cannot be shared across criteria; collector cost is duplicated |
| No typed evidence model | Gates produce strings; no structured findings, no severity, no source location grounding |
| `StubJudgeGate` at rung 6 | LLM judge gate is a stub that always skips -- zero judge evaluation capability |
| Monolithic `GateService` dispatch | No way to compose evaluation strategies beyond rung order |
| No preference/trace feedback loop | Gate outcomes are consumed but never compound into system-wide learning |

**Target state**: A new `roko-eval` crate family provides typed evidence collectors,
composable criteria, LLM judge panels with Bradley-Terry pairwise scoring, and a
self-improvement flywheel. A `BridgeGateService` wraps the existing `GateService` for
zero-regression incremental migration. New crates compose with existing learning
infrastructure (`roko-learn`), knowledge persistence (`roko-neuro`), and the runtime
event bus (`roko-core`).

**New crates**: `roko-eval`, `roko-eval-metrics`, `roko-eval-judge`, `roko-eval-community`

**Modified crates**: `roko-gate` (bridge adapters), `roko-learn` (flywheel wiring),
`roko-core` (event bus extensions), `roko-cli` (eval subcommands + TUI), `roko-serve`
(eval API routes), `roko-runtime` (new event variants)

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Severity |
|---|---|---|---|
| AP-SUBPROCESS | Each gate spawns its own subprocess (`tokio::process::Command`) | `CompileGate::verify()` at `crates/roko-gate/src/compile.rs`, `ClippyGate::verify()` at `crates/roko-gate/src/clippy_gate.rs`, `TestGate::verify()` at `crates/roko-gate/src/test_gate.rs` | High |
| AP-STUBJUDGE | `StubJudgeGate` always skips/fails, never actually evaluates | `crates/roko-gate/src/gate_service.rs:197-225`, hard-coded skip at line 249-256 | High |
| AP-STRINGVERDICTS | Gate output is unstructured `String` -- no parseable findings, no source locations | `GateVerdict.output: String` at `crates/roko-core/src/foundation.rs:284-296` | Medium |
| AP-NOEVIDENCE | Evidence (stdout/stderr/exit code) is produced and consumed inside the same function | Every gate `verify()` method creates and destroys evidence locally | Medium |
| AP-NOFEEDBACK | Gate outcomes are recorded in episodes but never feed back into agent routing or prompt optimization | `crates/roko-learn/src/episode_logger.rs` writes but learning subsystem only reads efficiency events | Medium |
| AP-SINGLEMODEL | `LlmJudgeGate` uses a single oracle -- no panel, no position swap, no family exclusion | `crates/roko-gate/src/llm_judge_gate.rs:53-56` defines `JudgeOracle` as single-model | Low |
| AP-RUNGONLY | Adaptive thresholds track per-rung granularity, not per-criterion | `AdaptiveThresholds::observe(rung, passed)` at `crates/roko-gate/src/adaptive_threshold.rs:321` | Low |

---

## Dependency Graph

```
Phase 1 (T5.1-T5.6): Core abstractions (roko-eval kernel)
  |
  +-- Phase 2 (T5.7-T5.12): Code criteria (roko-eval-metrics)  \
  |                                                              +-- Phase 5 (T5.27-T5.32): Flywheel
  +-- Phase 3 (T5.13-T5.17): AST/semantic/runtime criteria     /
  |
  +-- Phase 4 (T5.18-T5.26): LLM judge panel (roko-eval-judge)
  |
  +-- Phase 6 (T5.33-T5.39): Community marketplace (roko-eval-community)
  |
  Phase 7 (T5.40-T5.45): Dashboard integration
  |
  Phase 8 (T5.46-T5.48): Integration + migration
```

Phases 2, 3, and 4 can run in parallel after Phase 1. Phase 5 depends on Phases 2+3.
Phase 6 depends on Phase 1 only. Phase 7 depends on Phases 1+4. Phase 8 depends on all.

---

## Phase 1: Core Abstractions (`roko-eval` Kernel)

The trait definitions, type system, and contracts that every other phase builds on.
Everything in `crates/roko-eval/src/`. Depends on `roko-core` for `Engram`, `Score`,
`Verdict`, `Context`, `Verify`, `ContentHash`.

### Task 5.1: Scaffold `roko-eval` crate with core types
**Priority**: P0
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/types.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (add to `members`)
**Depends On**: none

#### Context
No `roko-eval` crate exists yet. The crate will house the evaluation framework kernel.
Dependencies: `roko-core`, `async-trait`, `serde`, `serde_json`, `thiserror`, `uuid`, `chrono`.
Register in workspace `Cargo.toml` under `members` (NOT `default-members` -- it is a library, not a shipped binary).

#### Implementation Steps
1. Create `crates/roko-eval/Cargo.toml` with dependencies on `roko-core = { path = "../roko-core" }`, `async-trait`, `serde`, `serde_json`, `thiserror`, `uuid`, `chrono`.
2. Create `crates/roko-eval/src/lib.rs` declaring the crate's modules and re-exports.
3. Create `crates/roko-eval/src/types.rs` defining the core types:
   - `EvidenceKind` enum (ProcessOutput, ProcessStatus, Diff, SemanticDiff, Ast, RuntimeTrace, StaticAnalysis, Custom). Start with the evidence kinds that existing gates produce. New kinds (Dom, ComputedStyles, Screenshot, etc.) are added when criteria need them.
   - `ArtifactRef` struct (id: String, path: Option<PathBuf>, url: Option<String>, artifact_type: String, content_hash: Option<ContentHash>, metadata: serde_json::Value).
   - `EvidenceBag` struct wrapping `Vec<EvidenceItem>` with typed accessors: `get_one(kind) -> Option<&EvidenceItem>`, `get_all(kind) -> Vec<&EvidenceItem>`, `has(kind) -> bool`, `insert(item)`.
   - `EvidenceItem` struct (kind: EvidenceKind, data: serde_json::Value, source: String, collected_at: DateTime<Utc>, content_hash: Option<ContentHash>).
   - `Severity` enum (Critical, Hard, Soft, Info).
   - `CriterionKind` enum (Deterministic, Computed, Heuristic, JudgePanel, Script).
   - `Finding` struct (criterion: String, severity: Severity, summary: String, detail: Option<String>, source_location: Option<SourceLocation>, rule_id: Option<String>, source_tool: Option<String>, fix_hint: Option<String>, confidence: Option<f64>).
   - `SourceLocation` struct (file: String, line: Option<u32>, col: Option<u32>, end_line: Option<u32>, end_col: Option<u32>).
   - `CriterionResult` struct (criterion_name: String, kind: CriterionKind, score: f64, passed: bool, findings: Vec<Finding>, metadata: Option<serde_json::Value>, duration_ms: u64, cost_usd: f64).
   - `EvalVerdict` struct (passed: bool, score: f64, hard_failures: Vec<String>, soft_scores: Vec<(String, f64)>, findings: Vec<Finding>, criteria_passed: usize, criteria_total: usize).
   - `EvalError` enum (EvidenceUnavailable { kind: EvidenceKind, collector: String }, Evaluation(String), CollectorFailed { name: String, source: String }, Timeout { duration_ms: u64 }, Configuration(String), Internal(String)).
4. Add `"crates/roko-eval"` to workspace `members` in `/Users/will/dev/nunchi/roko/roko/Cargo.toml`.

#### Design Guidance
All types must derive `Debug, Clone, Serialize, Deserialize`. Use `#[serde(rename_all = "snake_case")]` for enums. `EvidenceBag` should be `#[derive(Default)]` so empty bags can be constructed trivially. The `EvalError` should implement `std::error::Error` via `thiserror`.

#### Verification Criteria
- [ ] `cargo build -p roko-eval` compiles without errors
- [ ] `cargo test -p roko-eval` passes (basic type construction tests)
- [ ] All types round-trip through serde_json

---

### Task 5.2: Define `EvidenceCollector` trait
**Priority**: P0
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/collector.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.1

#### Context
Currently, each gate in `roko-gate` spawns its own subprocess: `CompileGate::verify()` creates a `tokio::process::Command("cargo", ["check", ...])` internally. This means evidence (stdout, stderr, exit code) is produced and consumed inside the same function -- no sharing between criteria.

The `EvidenceCollector` trait separates evidence production from evaluation. Collectors produce typed `EvidenceItem`s that multiple criteria can consume.

#### Implementation Steps
1. Define `CollectorRequirements` struct:
   ```rust
   pub struct CollectorRequirements {
       pub needs_filesystem: bool,
       pub needs_network: bool,
       pub timeout_ms: u64,
   }
   ```
2. Define `EvidenceCollector` trait:
   ```rust
   #[async_trait]
   pub trait EvidenceCollector: Send + Sync {
       fn name(&self) -> &str;
       fn produces(&self) -> &[EvidenceKind];
       fn requires(&self) -> CollectorRequirements;
       async fn collect(
           &self,
           artifact: &ArtifactRef,
           ctx: &Context,
       ) -> Result<Vec<EvidenceItem>, EvalError>;
   }
   ```
3. Implement `ProcessCollector` (spawns a shell command, captures stdout/stderr/exit code as ProcessOutput + ProcessStatus evidence items). Factory methods: `for_compile(build_system: BuildSystem)`, `for_lint(build_system: BuildSystem)`, `for_test(build_system: BuildSystem)`, `for_format(build_system: BuildSystem)`. Import `BuildSystem` from `roko_gate::payload::BuildSystem` (add `roko-gate` as dependency).
4. Implement `DiffCollector` (runs `git diff`, produces Diff evidence).
5. Implement `CompositeCollector` wrapping `Vec<Box<dyn EvidenceCollector>>` that runs all inner collectors and merges results into a single `Vec<EvidenceItem>`.
6. Add `pub mod collector;` to `lib.rs` and re-export public types.

#### Design Guidance
`ProcessCollector` should reuse the subprocess spawning pattern from `ShellGate::verify()` at `crates/roko-gate/src/shell.rs` but instead of producing a `Verdict`, it produces `Vec<EvidenceItem>`. The timeout comes from `CollectorRequirements`. The `for_compile()` factory should produce the same command as `CompileGate::new()` -- check `crates/roko-gate/src/compile.rs` for the exact arguments per `BuildSystem`. `CompositeCollector` runs inner collectors sequentially (not parallel) to avoid resource contention.

#### Verification Criteria
- [ ] Unit tests for `ProcessCollector` with `echo hello` (passing) and `false` (failing) commands
- [ ] `DiffCollector` test in a temp git repo with a staged change
- [ ] `CompositeCollector` merges results from 2 inner collectors

---

### Task 5.3: Define `Criterion` trait
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/criterion.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.1

#### Context
Currently, each gate in `roko-gate` both collects evidence and evaluates it in a single `verify()` call. The `Criterion` trait separates evaluation: it receives pre-collected evidence via `EvidenceBag` and produces a `CriterionResult`.

#### Implementation Steps
1. Define the `Criterion` trait:
   ```rust
   #[async_trait]
   pub trait Criterion: Send + Sync {
       fn name(&self) -> &str;
       fn criterion_kind(&self) -> CriterionKind;
       fn is_hard(&self) -> bool;
       fn required_evidence(&self) -> &[EvidenceKind];
       fn optional_evidence(&self) -> &[EvidenceKind] { &[] }
       fn default_threshold(&self) -> f64 { 0.5 }
       async fn evaluate(
           &self,
           artifact: &ArtifactRef,
           evidence: &EvidenceBag,
           ctx: &Context,
       ) -> Result<CriterionResult, EvalError>;
   }
   ```
2. Implement `fn check_evidence(criterion: &dyn Criterion, bag: &EvidenceBag) -> Result<(), EvalError>` as a standalone function that verifies all `required_evidence()` kinds exist in the bag, returning `EvalError::EvidenceUnavailable` if any are missing.
3. Add `pub mod criterion;` to `lib.rs` and re-export.

#### Verification Criteria
- [ ] Unit test with a mock criterion and a pre-populated `EvidenceBag`
- [ ] `check_evidence` returns error when required evidence is missing
- [ ] `check_evidence` returns Ok when required evidence is present

---

### Task 5.4: Define `Profile` and `EvalService`
**Priority**: P0
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/profile.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/service.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.1, 5.2, 5.3

#### Context
`Profile` composes criteria into a named evaluation strategy. `EvalService` orchestrates the full evaluation lifecycle: resolve profile -> collect evidence -> run criteria -> aggregate verdict -> emit trace.

The existing `GatePipeline` at `crates/roko-gate/src/gate_pipeline.rs` implements sequential composition of `Verify` impls with short-circuit on failure. `EvalService` does the same but over `Criterion` impls with typed evidence and richer aggregation.

#### Implementation Steps
1. Define `Profile`:
   ```rust
   pub struct Profile {
       pub id: String,
       pub name: String,
       pub tags: Vec<String>,
       pub strategy: EvalStrategy,
       pub criteria: Vec<CriterionRef>,
   }
   pub enum EvalStrategy {
       Sequential,
       ConjunctiveHardParetoSoft,
       WeightedSum { weights: Vec<f64> },
   }
   pub struct CriterionRef {
       pub name: String,
       pub hard: Option<bool>,
       pub threshold: Option<f64>,
       pub params: serde_json::Value,
   }
   ```
2. Define `EvalService`:
   ```rust
   pub struct EvalService {
       pub collectors: Vec<Box<dyn EvidenceCollector>>,
       pub criteria: Vec<Box<dyn Criterion>>,
   }
   impl EvalService {
       pub async fn evaluate(
           &self,
           artifact: &ArtifactRef,
           profile: &Profile,
           ctx: &Context,
       ) -> Result<EvalTrace, EvalError>;
   }
   ```
3. `evaluate()` flow:
   a. Determine which evidence kinds are needed from the profile's criteria.
   b. Run only the collectors that produce those kinds.
   c. Run criteria in profile order. For `Sequential` strategy, short-circuit on hard failure. For `ConjunctiveHardParetoSoft`, run all hard criteria first (short-circuit on failure), then run soft criteria and aggregate.
   d. Aggregate into `EvalVerdict`: passed = all hard criteria pass AND soft score >= profile threshold.
   e. Return `EvalTrace` (defined in Task 5.5).

#### Design Guidance
Mirror the short-circuit pattern from `GatePipeline::verify()` at `crates/roko-gate/src/gate_pipeline.rs:224-244`. The evidence optimization (only collecting what criteria need) is critical -- it avoids running expensive collectors (e.g., coverage) when no criterion requires that evidence. Use `HashSet<EvidenceKind>` to compute the union of all criteria's required evidence kinds, then filter collectors to those whose `produces()` intersects with the needed set.

#### Verification Criteria
- [ ] Integration test: ProcessCollector -> mock CompileCriterion -> EvalService -> EvalVerdict
- [ ] Short-circuit test: second criterion not called when first hard criterion fails
- [ ] Evidence optimization test: unused collector not invoked

---

### Task 5.5: Define `EvalTrace` and JSONL storage
**Priority**: P0
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/trace.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/trace_store.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.1

#### Context
`EvalTrace` is the durable record of every evaluation run. It parallels the existing `Episode` from `crates/roko-learn/src/episode_logger.rs` (agent turn accounting) but captures full evaluation detail: evidence, per-criterion scores, findings, pipeline context, cost. Cross-referenced by `task_id` and timestamp.

#### Implementation Steps
1. Define `EvalTrace`:
   ```rust
   pub struct EvalTrace {
       pub id: String,
       pub timestamp: DateTime<Utc>,
       pub artifact: ArtifactRef,
       pub profile_id: String,
       pub evidence_phase: Vec<CollectorPhaseRecord>,
       pub criterion_results: Vec<CriterionResult>,
       pub verdict: EvalVerdict,
       pub pipeline_context: PipelineContext,
       pub cost: EvalCost,
       pub duration_ms: u64,
       pub task_id: Option<String>,
       pub plan_id: Option<String>,
   }
   ```
2. Define `PipelineContext` (populated from `AgentEfficiencyEvent` fields at trace emission time):
   ```rust
   pub struct PipelineContext {
       pub model: String,
       pub backend: String,
       pub prompt_variant: Option<String>,
       pub agent_role: String,
       pub generation_cost_usd: f64,
       pub generation_tokens: u64,
   }
   ```
3. Define `EvalCost`:
   ```rust
   pub struct EvalCost {
       pub total_usd: f64,
       pub evidence_usd: f64,
       pub criteria_usd: f64,
       pub judge_usd: f64,
   }
   ```
4. Define `CollectorPhaseRecord`:
   ```rust
   pub struct CollectorPhaseRecord {
       pub collector_name: String,
       pub evidence_kinds: Vec<EvidenceKind>,
       pub duration_ms: u64,
       pub success: bool,
       pub error: Option<String>,
   }
   ```
5. Define `TraceStore` with JSONL persistence at `.roko/eval/traces.jsonl`:
   - `append(trace: &EvalTrace) -> Result<(), EvalError>`: append one line of JSON. Crash-tolerant: write + sync.
   - `recent(limit: usize) -> Result<Vec<EvalTrace>, EvalError>`: read last N traces. Tolerant of malformed trailing lines (skip, do not error).
   - `by_id(id: &str) -> Result<Option<EvalTrace>, EvalError>`: scan for specific trace.
   - `by_task(task_id: &str) -> Result<Vec<EvalTrace>, EvalError>`: filter by task_id.

#### Design Guidance
Follow the exact persistence pattern from `EpisodeLogger` at `crates/roko-learn/src/episode_logger.rs`: append-only JSONL, malformed-line tolerance, `parking_lot::Mutex` for concurrent writers. Use `tokio::fs` for async I/O. The `recent()` method reads the entire file and returns the last N entries -- acceptable for MVP (traces accumulate slowly).

#### Verification Criteria
- [ ] Write/read round-trip test
- [ ] Malformed line tolerance test (corrupt last line, still read preceding lines)
- [ ] `by_task()` filter test

---

### Task 5.6: Bridge adapter -- `BridgeGateService`
**Priority**: P0
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/bridge.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/lib.rs` (add `pub mod bridge;`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/Cargo.toml` (add `roko-eval` dependency)
**Depends On**: 5.1, 5.4, 5.5

#### Context
The migration from `roko-gate` to `roko-eval` must be incremental. `BridgeGateService` wraps the existing `GateService` at `crates/roko-gate/src/gate_service.rs` and intercepts gate names that have been migrated to the new system. For migrated gates, it runs through `EvalService` and projects the `EvalTrace` back to `GateVerdict` for backward compatibility. For non-migrated gates, it delegates to the inner `GateService` unchanged.

The `GateRunner` trait is defined at `crates/roko-core/src/foundation.rs:311-320`:
```rust
#[async_trait]
pub trait GateRunner: Send + Sync {
    async fn run_gates(&self, config: GateConfig) -> Result<GateReport>;
}
```

#### Implementation Steps
1. Add `roko-eval = { path = "../roko-eval" }` to `crates/roko-gate/Cargo.toml`.
2. Define `BridgeGateService`:
   ```rust
   pub struct BridgeGateService {
       legacy: GateService,
       eval_service: Option<Arc<EvalService>>,
       migrated: HashSet<String>,
   }
   impl BridgeGateService {
       pub fn new(legacy: GateService) -> Self;
       pub fn with_eval_service(self, svc: Arc<EvalService>) -> Self;
       pub fn migrate_gate(mut self, name: &str) -> Self;
   }
   ```
3. Implement `GateRunner` for `BridgeGateService`:
   - Split `config.enabled_gates` into migrated and non-migrated.
   - For non-migrated gates: build a sub-`GateConfig` and delegate to `self.legacy.run_gates()`.
   - For migrated gates: construct `ArtifactRef` from `config.workdir`, run through `self.eval_service.evaluate()`, project `EvalTrace` to `Vec<GateVerdict>` via a `From` impl.
   - Merge verdicts in rung order.
4. Implement `From<&EvalTrace> for Vec<GateVerdict>`:
   - Each `CriterionResult` becomes a `GateVerdict` with `gate_name = criterion_name`, `passed`, `output = summary of findings`, `duration_ms`.
   - The overall `EvalVerdict` is not a separate verdict -- it is the conjunction of its parts.
5. Add `pub mod bridge;` to `crates/roko-gate/src/lib.rs` and `pub use bridge::BridgeGateService;`.

#### Design Guidance
The bridge is the zero-regression guarantee. With no migrated gates (`migrated.is_empty()`), it must behave **identically** to `GateService`. Test this by running the exact same `GateConfig` through both and asserting verdict-for-verdict equivalence. The bridge does NOT change the `GateRunner` trait or `GateReport` struct -- it produces the same output type.

#### Verification Criteria
- [ ] Test: `BridgeGateService` with no migrations behaves identically to `GateService`
- [ ] Test: a migrated gate produces a `GateVerdict` matching the legacy gate's pass/fail
- [ ] `cargo test -p roko-gate` passes including existing tests

---

## Phase 2: Code Criteria (`roko-eval-metrics`)

Migrate existing gates to the new criterion model. Each criterion consumes evidence from
collectors rather than spawning its own subprocess.

### Task 5.7: Scaffold `roko-eval-metrics` crate
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (add to `members`)
**Depends On**: 5.1

#### Context
Dependencies: `roko-eval`, `roko-core`, `roko-gate` (for parse functions in `compile_errors.rs` and `test_gate.rs`), `async-trait`, `serde`, `serde_json`. Optional: `tree-sitter`, `tree-sitter-rust` behind `ast` feature flag (Phase 3).

#### Implementation Steps
1. Create `crates/roko-eval-metrics/Cargo.toml`. Use feature gating: `[features] ast = ["dep:tree-sitter", "dep:tree-sitter-rust"]`.
2. Create `crates/roko-eval-metrics/src/lib.rs` with module declarations and re-exports.
3. Register in workspace `Cargo.toml`.

#### Verification Criteria
- [ ] `cargo build -p roko-eval-metrics` compiles without errors

---

### Task 5.8: `CompileCriterion`
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/compile.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
**Depends On**: 5.7

#### Context
Migrates `CompileGate` from `crates/roko-gate/src/compile.rs`. Consumes `EvidenceKind::ProcessOutput` + `EvidenceKind::ProcessStatus` from `ProcessCollector::for_compile()`. Reuses existing parse functions: `roko_gate::parse_cargo_json` and `roko_gate::parse_plain_stderr` at `crates/roko-gate/src/compile_errors.rs` for structured error extraction.

The existing `CompileGate` at `crates/roko-gate/src/compile.rs` spawns `cargo check --message-format=json`, parses the output, and produces a `Verdict`. The new `CompileCriterion` does the same evaluation logic but reads from the `EvidenceBag` instead of spawning the process itself.

#### Implementation Steps
1. Implement `CompileCriterion` implementing `Criterion`:
   - `name()` = "compile"
   - `criterion_kind()` = `CriterionKind::Deterministic`
   - `is_hard()` = `true`
   - `required_evidence()` = `[EvidenceKind::ProcessOutput, EvidenceKind::ProcessStatus]`
   - `default_threshold()` = `1.0` (binary: must compile)
   - `evaluate()`: extract stdout from ProcessOutput evidence, parse with `parse_cargo_json()` or `parse_plain_stderr()`, compute binary score (0.0 or 1.0), emit up to N `Finding` items with source_location, rule_id, fix_hint from `CompileError` structs.
2. Add configurable `max_error_findings: usize` (default 20) to limit Finding count.

#### Verification Criteria
- [ ] Unit test with mock evidence bag containing a failing cargo build output
- [ ] Test that findings include source_location with file/line

---

### Task 5.9: `LintCriterion`
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lint.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
**Depends On**: 5.7

#### Context
Migrates `ClippyGate` from `crates/roko-gate/src/clippy_gate.rs`. Two modes: strict (binary pass/fail matching exit code) and graduated (weighted score: errors * 0.2 + warnings * 0.05, subtracted from 1.0).

#### Implementation Steps
1. Implement `LintCriterion` with configurable `LintMode` (Strict, Graduated).
2. Parse clippy diagnostic output into `LintDiagnostic` structs (severity, rule_id, message, file, line, col, suggestion).
3. In Strict mode: score = 0.0 if exit code != 0, else 1.0.
4. In Graduated mode: score = (1.0 - errors * 0.2 - warnings * 0.05).clamp(0.0, 1.0).
5. Emit Findings with rule_id (clippy lint name) and suggestion text.

#### Verification Criteria
- [ ] Unit tests for both strict and graduated modes
- [ ] Test that findings carry clippy rule_id

---

### Task 5.10: `TestCriterion`
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/test.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
**Depends On**: 5.7

#### Context
Migrates `TestGate` from `crates/roko-gate/src/test_gate.rs`. Reuses `roko_gate::parse_test_counts` (exported from `crates/roko-gate/src/test_gate.rs`). Score = pass_rate = passed / (passed + failed). Extracts failing test names and their stdout blocks as Findings.

#### Implementation Steps
1. Implement `TestCriterion`:
   - `name()` = "test"
   - `criterion_kind()` = `CriterionKind::Deterministic`
   - `is_hard()` = configurable (default true)
   - `required_evidence()` = `[EvidenceKind::ProcessOutput, EvidenceKind::ProcessStatus]`
   - `evaluate()`: parse test output with `parse_test_counts()`, compute pass_rate, extract failing test names and their output blocks as Findings.
2. Support `TestSelector` (All, Changed, Specific) for evidence filtering.

#### Verification Criteria
- [ ] Unit test with mock test output containing 2 failures out of 14
- [ ] Test that pass_rate = 12/14

---

### Task 5.11: `FormatCriterion`, `SecurityCriterion`, `DiffCriterion`
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/format.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/security.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/diff.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
**Depends On**: 5.7

#### Context
Three smaller criteria following the same pattern as the above.

#### Implementation Steps
1. `FormatCriterion`: consumes ProcessOutput from `cargo fmt --check`. Extracts unformatted file paths from diff output. Score: 0.0 if any unformatted files, 1.0 otherwise. Hard severity. Findings list unformatted file paths.
2. `SecurityCriterion`: consumes ProcessOutput from `cargo audit`. Emits Info-level findings when audit tool is missing (do not fail). Parses advisory list when available.
3. `DiffCriterion`: consumes Diff evidence. Analyzes git diff stats (files changed, insertions, deletions). Optionally consumes SemanticDiff evidence for richer analysis. Score = 1.0 (always passes; the criterion is informational). Soft severity.

#### Verification Criteria
- [ ] Unit tests for each criterion with mock evidence

---

### Task 5.12: `CriterionStats` -- per-criterion adaptive tracking
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/stats.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/service.rs`
**Depends On**: 5.4

#### Context
The existing `AdaptiveThresholds` at `crates/roko-gate/src/adaptive_threshold.rs` tracks per-rung statistics (EMA pass rate, consecutive passes, CUSUM). `CriterionStats` extends this concept to per-criterion granularity, enabling criterion-level skip decisions and cost tracking.

#### Implementation Steps
1. Define `CriterionStats`:
   ```rust
   pub struct CriterionStats {
       pub ema_pass_rate: f64,
       pub consecutive_passes: u32,
       pub cusum_high: f64,
       pub cusum_low: f64,
       pub score_history: VecDeque<f64>,  // last 50
       pub avg_duration_ms: f64,
       pub avg_cost_usd: f64,
       pub total_observations: u64,
   }
   ```
2. Implement `observe(passed: bool, score: f64, duration_ms: u64, cost_usd: f64)` with the same EMA/CUSUM logic as `AdaptiveThresholds::observe()`.
3. Implement `should_skip() -> bool` based on consecutive pass streak (threshold: 20).
4. Define `CriterionStatsStore` persisting to `.roko/eval/criterion-stats.json` with `load()` and `save()` (same pattern as `AdaptiveThresholds::load/save`).
5. Wire into `EvalService::evaluate()`: after each criterion evaluation, call `stats.observe(passed, score, duration_ms, cost_usd)`.

#### Verification Criteria
- [ ] Test that 20+ consecutive passes triggers skip suggestion
- [ ] Round-trip persistence test

---

## Phase 3: AST/Semantic/Runtime Evidence and Criteria

Novel analysis capabilities beyond text-level gates. Feature-gated behind `ast` flag.

### Task 5.13: `AstCollector` -- tree-sitter AST extraction
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/ast_collector.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/Cargo.toml`
**Depends On**: 5.7

#### Context
Uses `tree-sitter` + `tree-sitter-rust` to parse source files and produce `EvidenceKind::Ast` evidence. The existing `crates/roko-lang-rust/` already has tree-sitter integration for the code intelligence indexer. Reuse patterns from there.

#### Implementation Steps
1. Define `FileAst` struct: `{ path: String, items: Vec<AstItem>, complexity: Vec<FunctionComplexity> }`.
2. Define `AstItem`: `{ kind: String, name: String, visibility: String, span: (usize, usize), children: Vec<AstItem>, body_text: Option<String> }`.
3. Define `FunctionComplexity`: `{ function: String, cyclomatic: u32, cognitive: u32, body_lines: u32 }`.
4. Implement `AstCollector` implementing `EvidenceCollector`:
   - `produces()` = `[EvidenceKind::Ast]`
   - `collect()`: parse source files using tree-sitter, walk the AST, extract items and complexity.
   - Factory method: `AstCollector::for_changed_files(workdir: &Path)` -- uses `git diff --name-only` to find changed files, parses only those.
5. Gate behind `#[cfg(feature = "ast")]`.

#### Design Guidance
Tree-sitter C bindings can be slow to compile. Gate behind `ast` feature flag so the default workspace build is unaffected. Complexity calculation: cyclomatic = number of branch points (if, match arm, while, for, &&, ||); cognitive = cyclomatic + nesting depth bonus; body_lines = line count excluding braces.

#### Verification Criteria
- [ ] Parse a small Rust file, assert item count and kinds
- [ ] Complexity calculation for a function with 5 nested if-else branches

---

### Task 5.14: `StructuralCompletenessCriterion`
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/structural_completeness.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
**Depends On**: 5.13

#### Context
AST-based replacement for `SymbolGate` at `crates/roko-gate/src/symbol_gate.rs`. Takes a list of structural expectations and matches them against the flattened AST items.

#### Implementation Steps
1. Define `StructuralExpectation`: `{ kind: String, name_pattern: String, path: Option<String>, visibility: Option<String>, substantive_body: bool }`.
2. Implement `StructuralCompletenessCriterion`:
   - Consumes `EvidenceKind::Ast`.
   - Matches expectations against flattened AST items using regex on name_pattern.
   - When `substantive_body = true`, checks that matched items do not contain `todo!()` or `unimplemented!()` in body_text.
   - Score = met_expectations / total_expectations. Hard severity.
3. Gate behind `#[cfg(feature = "ast")]`.

#### Verification Criteria
- [ ] Test against a file with 3 expected functions, 1 missing
- [ ] Test `todo!()` detection

---

### Task 5.15: `ComplexityCriterion`
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/complexity.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
**Depends On**: 5.13

#### Context
Checks cyclomatic/cognitive complexity and body line count per function from AST evidence. Default thresholds: cyclomatic 15, cognitive 20, body lines 100.

#### Implementation Steps
1. Implement `ComplexityCriterion`:
   - Consumes `EvidenceKind::Ast`.
   - Score = fraction of functions within all three thresholds. Soft severity. Default threshold 0.9.
   - Emit Findings for each function exceeding any threshold.
2. Gate behind `#[cfg(feature = "ast")]`.

#### Verification Criteria
- [ ] Test with functions of varying complexity

---

### Task 5.16: `SemanticDiffCollector` and `SubstanceCriterion`
**Priority**: P2
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/semantic_diff.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/substance.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
**Depends On**: 5.13

#### Context
Compares before/after ASTs to classify changes at the structural level. Catches "did nothing" failure mode with higher precision than the existing `DiffGate`'s forbidden-token matching at `crates/roko-gate/src/diff_gate.rs`.

#### Implementation Steps
1. `SemanticDiffCollector`: compares before/after ASTs, classifies each change as `SemanticChange { kind, significance }`. Kinds: FunctionAdded, FunctionModified, TypeChanged, FormattingOnly, DocumentationChanged, ImportChanged.
2. `SubstanceCriterion`: consumes SemanticDiff evidence, scores average significance. Threshold 0.2 (below this = "did nothing").
3. Gate behind `#[cfg(feature = "ast")]`.

#### Verification Criteria
- [ ] Test: diff adding only comments scores near 0.0 significance
- [ ] Test: diff adding a new function scores high significance

---

### Task 5.17: `RuntimeTraceCollector` and `CoverageCriterion`
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/runtime_trace.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/coverage.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-metrics/src/lib.rs`
**Depends On**: 5.7

#### Context
Runs tests with coverage instrumentation (cargo-tarpaulin for Rust). Parses coverage output.

#### Implementation Steps
1. `RuntimeTraceCollector`: runs `cargo tarpaulin --out Json`, parses JSON output into `CoverageData { line_coverage: f64, branch_coverage: f64, files: Vec<FileCoverage> }`. Produces `EvidenceKind::RuntimeTrace`.
2. `CoverageCriterion`: checks minimum line coverage threshold (default 0.7). Optional `diff_only` mode: only consider coverage for changed files. Reports files with lowest coverage as Findings. Soft severity.

#### Verification Criteria
- [ ] Test with mock coverage JSON output

---

## Phase 4: LLM Judge Panel (`roko-eval-judge`)

Bradley-Terry pairwise comparison, disjoint-family panels, position bias
mitigation, and the full judge methodology.

### Task 5.18: Scaffold `roko-eval-judge` crate
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/lib.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (add to `members`)
**Depends On**: 5.1

#### Context
Dependencies: `roko-eval`, `roko-core`, `roko-gate` (for `JudgeOracle` trait at `crates/roko-gate/src/llm_judge_gate.rs:53-56`), `async-trait`, `serde`, `serde_json`, `rand`.

#### Verification Criteria
- [ ] `cargo build -p roko-eval-judge` compiles without errors

---

### Task 5.19: Bradley-Terry MLE with Davidson ties
**Priority**: P1
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/bt_model.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/lib.rs`
**Depends On**: 5.18

#### Context
The core statistical model. BT MLE via logistic regression with high regularization (C=10^6). Davidson tie parameter (nu). Elo scale mapping: `Elo_i = theta_i * 400 / ln(10)`.

#### Implementation Steps
1. Define `ComparisonTriple { candidate_a: String, candidate_b: String, outcome: ComparisonOutcome }` where `ComparisonOutcome` is `APreferred | BPreferred | Tie`.
2. Define `BtResult { elo_scores: BTreeMap<String, f64>, tie_parameter: f64, comparison_count: u32, log_likelihood: f64 }`.
3. Implement BT MLE fitting:
   - Construct the logistic regression problem: for each triple, create a row in X with +1 for candidate_a, -1 for candidate_b.
   - Solve via iterative reweighted least squares (IRLS) with regularization.
   - Map fitted theta values to Elo scale.
   - Include Davidson tie parameter when ties are present.
4. BCa bootstrap confidence intervals (B=1000 resamples) are an optional extension (can be a `compute_confidence_intervals()` method that is expensive to call).

#### Design Guidance
For the IRLS solver, use a simple iterative approach rather than pulling in `nalgebra` as a hard dependency. The problem is small (typically 2-5 candidates). 20 iterations of Newton-Raphson suffice. Regularize by adding C * theta to the gradient and C to the Hessian diagonal.

#### Verification Criteria
- [ ] Known-answer test: feed 10 comparisons where A always beats B, verify Elo_A > Elo_B by >200 points
- [ ] Test with ties: verify tie_parameter > 0 when ties are present
- [ ] Test with equal outcomes: verify Elo scores are approximately equal

---

### Task 5.20: Judge panel construction with family exclusion
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/panel.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/lib.rs`
**Depends On**: 5.18

#### Context
The critical composition rule: never use the same model family as both generator and judge. Panel construction integrates with the existing cascade router at `crates/roko-learn/src/cascade_router.rs` to determine the generator family from `AgentEfficiencyEvent.model`.

The `slug_family()` function at `crates/roko-learn/src/cascade/helpers.rs` maps model slugs to families (e.g., "claude-opus-4-6" -> "anthropic", "gpt-4o" -> "openai").

#### Implementation Steps
1. Define `JudgePanelConfig { min_panel_size: usize, preferred_panel_size: usize, exclude_generator_family: bool }` with defaults (3, 3, true).
2. Define `JudgeSpec { model_id: String, family: String, endpoint: Option<String>, max_tokens: u32, temperature: f64 }`.
3. Implement `construct_panel(available_models: &[JudgeSpec], generator_family: Option<&str>, config: &JudgePanelConfig) -> Result<Vec<JudgeSpec>, EvalError>`:
   - Exclude models from generator family.
   - Select one model per remaining family.
   - Sort by priority (configurable or hardcoded: frontier closed > rubric-conditioned > open).
   - Take top `preferred_panel_size`.
   - Error if fewer than `min_panel_size` available.
4. Implement `generator_family_from_model_slug(slug: &str) -> Option<String>` reusing the `slug_family()` logic from `crates/roko-learn/src/cascade/helpers.rs`.

#### Verification Criteria
- [ ] Test with 4 model families, exclude one, assert panel = 3
- [ ] Test error when fewer than min_panel_size available

---

### Task 5.21: Position swap-and-discard
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/position_swap.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/lib.rs`
**Depends On**: 5.18

#### Context
Mandatory position bias mitigation. For every pairwise comparison by every judge, present both orderings and discard inconsistent results. See PRD-04 Section 5 for the full rationale.

#### Implementation Steps
1. Define `PairwiseVerdict { PreferA, PreferB, Tie }`.
2. Define `PositionSwapResult { judge: JudgeSpec, verdict_ab: PairwiseVerdict, verdict_ba: PairwiseVerdict, consistent: bool, effective_verdict: Option<PairwiseVerdict> }`.
3. Implement `check_consistency(verdict_ab, verdict_ba) -> bool`:
   - `(PreferA, PreferB)` -> true (same artifact preferred regardless of position)
   - `(PreferB, PreferA)` -> true
   - `(Tie, Tie)` -> true
   - Everything else -> false (position-dependent, discard)
4. Implement IPI metric: `ipi(results: &[PositionSwapResult]) -> f64` = fraction of inconsistent results.

#### Verification Criteria
- [ ] Test consistent case: (PreferA, PreferB) -> consistent=true
- [ ] Test inconsistent case: (PreferA, PreferA) -> consistent=false, discarded
- [ ] Test IPI calculation

---

### Task 5.22: Anchor store and rotation
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/anchor.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/lib.rs`
**Depends On**: 5.18

#### Context
The fixed-anchor protocol: always compare new_candidate vs prev_best_release. Anchors persist to `.roko/eval/anchors.json`. Bootstrapping protocol establishes first anchor via absolute scoring.

#### Implementation Steps
1. Define `JudgeAnchor { content_hash, established_at_ms, provenance: AnchorProvenance, artifact: ArtifactRef, elo: f64, comparison_count: u64 }`.
2. Define `AnchorProvenance { HumanApproved, GatePassed, ArenaWinner, Bootstrapped }`.
3. Define `AnchorRotationConfig { max_anchor_age_days: u32 (30), rotation_win_rate: f64 (0.8), rotation_min_evals: u32 (20), bootstrap_rotation_evals: u32 (10) }`.
4. Implement `AnchorStore` with methods: `get(task_id) -> Option<JudgeAnchor>`, `set(anchor)`, `rotate_if_due(config) -> bool`. Persists to `.roko/eval/anchors.json`.
5. Bootstrapping: when no anchor exists, accept candidate with provenance `Bootstrapped`. After `bootstrap_rotation_evals` subsequent evaluations, auto-promote best candidate.

#### Verification Criteria
- [ ] Test bootstrap -> rotate lifecycle
- [ ] Persistence round-trip test

---

### Task 5.23: Aggregation and disagreement detection
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/aggregation.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/disagreement.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/lib.rs`
**Depends On**: 5.18

#### Context
Trimmed mean aggregation, learned judge weights, and disagreement detection. See PRD-04 Section 7.

#### Implementation Steps
1. Implement `trimmed_mean(scores: &mut [f64], trim_fraction: f64) -> Option<f64>`.
2. Define `LearnedJudgeWeights { weights: BTreeMap<String, f64>, fit_at_ms: i64, n_canary: u32, r_squared: f64, active: bool }`. Active when >= 500 canary examples.
3. Define `PanelDisagreement { agreement_rate: f64, score_spread: f64, krippendorff_alpha: f64, needs_human_review: bool, reason: Option<String> }`.
4. Implement `detect_disagreement(verdicts: &[PositionSwapResult]) -> PanelDisagreement`:
   - agreement_rate = consistent_judges / total_judges
   - score_spread = max_score - min_score
   - krippendorff_alpha: implement nominal alpha for a small panel
   - Flag for review when agreement < 0.5, spread > 0.3, or alpha < 0.4.

#### Verification Criteria
- [ ] Test trimmed mean with 5 scores
- [ ] Test disagreement detection with high/low agreement panels

---

### Task 5.24: Judge prompt templates and rubrics
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/prompts/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/prompts/code.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/rubric.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/lib.rs`
**Depends On**: 5.18

#### Context
Code evaluation 7-dimension rubric: correctness (0.30), maintainability (0.20), safety (0.15), performance (0.10), test_coverage (0.10), api_design (0.10), documentation (0.05). All prompts use analyze-before-rate format, JSON-only output.

#### Implementation Steps
1. Define `Rubric { dimensions: Vec<RubricDimension> }` and `RubricDimension { name: String, weight: f64, description: String, min_score: f64 }`.
2. Implement `code_rubric() -> Rubric` with the 7 code dimensions.
3. Define `JudgePrompt { system: String, user: String }` and implement `render_pairwise_prompt(rubric, artifact_a, artifact_b) -> JudgePrompt`.
4. Implement `parse_judge_response(response: &str) -> Result<JudgeRating, EvalError>` parsing the JSON output.
5. Define `JudgeRating { analysis_a: String, analysis_b: String, rubric_a: HashMap<String, f64>, rubric_b: HashMap<String, f64>, findings: Vec<Finding>, preference: PairwiseVerdict, confidence: f64, reasoning: String }`.

#### Verification Criteria
- [ ] Test prompt rendering contains all 7 dimensions
- [ ] Test JSON parsing of a sample response

---

### Task 5.25: `JudgePanelCriterion` and `PanelJudgeOracle` gate adapter
**Priority**: P1
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/criterion.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/gate_adapter.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/lib.rs`
**Depends On**: 5.19, 5.20, 5.21, 5.22, 5.23, 5.24

#### Context
The full evaluation flow. `JudgePanelCriterion` implements the `Criterion` trait from `roko-eval`. `PanelJudgeOracle` implements the existing `JudgeOracle` trait from `roko-gate` for backward compatibility.

The existing `JudgeOracle` trait at `crates/roko-gate/src/llm_judge_gate.rs:53-56`:
```rust
pub trait JudgeOracle: Send + Sync {
    async fn judge(&self, prompt: &str) -> Result<f32, String>;
}
```

#### Implementation Steps
1. `JudgePanelCriterion` implements `Criterion`:
   - `criterion_kind()` = `CriterionKind::JudgePanel`
   - `required_evidence()` = `[EvidenceKind::Diff]` (minimum)
   - `evaluate()` flow: construct panel -> for each judge run pairwise with position swap -> discard inconsistent -> aggregate -> BT model -> return CriterionResult.
   - The actual LLM calls are delegated to a `JudgeInvoker` trait (abstracts the HTTP call).
2. `PanelJudgeOracle` implements `JudgeOracle`:
   - Wraps the full panel flow.
   - Returns a normalized f32 score.
3. Define `JudgeInvoker` trait:
   ```rust
   #[async_trait]
   pub trait JudgeInvoker: Send + Sync {
       async fn invoke(&self, spec: &JudgeSpec, prompt: &JudgePrompt) -> Result<String, EvalError>;
   }
   ```
   This is the adapter point for real LLM backends. A mock implementation is provided for testing.

#### Design Guidance
The `JudgeInvoker` trait is critical for testability. Tests inject a mock invoker that returns predetermined responses. The real implementation (wiring to `roko-agent` backends) is done in Task 5.46 when the orchestrator integrates the full eval service.

#### Verification Criteria
- [ ] Integration test with mock judge invokers returning consistent preferences
- [ ] Test that inconsistent judges are discarded
- [ ] Test `PanelJudgeOracle` returns a score in [0, 1]

---

### Task 5.26: Sampling strategy and adaptive budget
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/sampling.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-judge/src/lib.rs`
**Depends On**: 5.18

#### Context
N=3 at T=0 per position (base). Adaptive increase when panel disagrees.

#### Implementation Steps
1. Implement `adaptive_sample_count(initial_agreement: f64, base_samples: u32, max_samples: u32) -> u32`:
   - agreement >= 0.8 -> base_samples
   - agreement >= 0.5 -> 2 * base_samples
   - agreement < 0.5 -> max_samples

#### Verification Criteria
- [ ] Unit test for each agreement tier

---

## Phase 5: Self-Improvement Flywheel

Every evaluation produces compounding value. Traces, preferences, patterns, curricula, and
experiment data feed back into the system.

### Task 5.27: Preference mining and `PreferenceTriple`
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/preference.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.5

#### Context
Every preference signal is logged as a `PreferenceTriple`. This collection is a private arena for pairwise learning.

#### Implementation Steps
1. Define `PreferenceTriple { id, prompt, candidate_a, candidate_b, preferred: PreferenceChoice, source: PreferenceSource, trace_id_a, trace_id_b, task_id, timestamp, criterion_deltas: Vec<CriterionDelta>, confidence: f64 }`.
2. Define `PreferenceSource { UserEdit, UserSelection, JudgePanel, ExternalBenchmark, RegressionComparison }`.
3. Define `CriterionDelta { criterion: String, score_a: f64, score_b: f64 }`.
4. Define `PreferenceStore` appending to `.roko/learn/preferences.jsonl` (same pattern as `EpisodeLogger`).
5. Implement mining: `mine_from_judge_panel(trace: &EvalTrace) -> Vec<PreferenceTriple>` -- every pairwise comparison produces a triple.

#### Verification Criteria
- [ ] Write/read round-trip test
- [ ] Mining from a trace with 3 judge comparisons produces 3 triples

---

### Task 5.28: Auto-grade bridge to existing `FeedbackService`
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/feedback_bridge.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/Cargo.toml` (add `roko-learn` dependency)
**Depends On**: 5.5

#### Context
Converts EvalTrace verdicts into `KnowledgeOutcome` records for the existing `FeedbackService` at `crates/roko-learn/src/feedback_service.rs`. Also wires into `ExperimentStore`: when trace carries a `prompt_variant`, report pass/fail to `VariantStats` for UCB1 convergence.

The existing `KnowledgeOutcome` enum at `crates/roko-learn/src/feedback_service.rs:27-33`:
```rust
pub enum KnowledgeOutcome { Success, Failure, Partial }
```

#### Implementation Steps
1. Implement `eval_trace_to_knowledge_outcome(trace: &EvalTrace) -> KnowledgeOutcome`: passed -> Success, failed -> Failure.
2. Implement `bridge_to_experiment_store(trace: &EvalTrace, store: &ExperimentStore)`: if `trace.pipeline_context.prompt_variant` is Some, record pass/fail to `VariantStats`.
3. The bridge functions are free-standing (not a trait impl) -- they are called from the orchestrator wiring in Task 5.46.

#### Design Guidance
Do NOT add the `roko-learn` dependency to `roko-eval` (circular risk). Instead, define the bridge as a conversion function that returns intermediate types. The actual `FeedbackService` calls happen in the orchestrator (which already depends on both crates). Define a `FeedbackBridgeOutput { knowledge_outcomes: Vec<(String, KnowledgeOutcome)>, experiment_outcome: Option<(String, bool)> }` that the orchestrator can consume.

Wait -- re-check: `roko-eval` depending on `roko-learn` may create a cycle since `roko-learn` may later depend on `roko-eval`. Safer approach: define the bridge output types in `roko-eval` and let the orchestrator do the actual FeedbackService/ExperimentStore calls.

#### Verification Criteria
- [ ] Test that a passing trace produces Success outcome
- [ ] Test that a failing trace produces Failure outcome
- [ ] Test experiment bridge with a trace carrying prompt_variant

---

### Task 5.29: Pattern library and neuro store promotion
**Priority**: P2
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/pattern_library.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/neuro_bridge.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.5

#### Context
Patterns extracted from successful evaluation traces are promoted into the `roko-neuro` knowledge store as engrams. Queried at dispatch time for system prompt enrichment.

#### Implementation Steps
1. Define `PatternEntry { id, name, category, fingerprint: Option<String>, polarity: PatternPolarity, support_count: u32, avg_score: f64, template: Option<String>, anti_pattern_description: Option<String>, tags: Vec<String>, updated_at }`.
2. Define `PatternPolarity { Positive, Negative }`.
3. Define `PatternLibrary` storing to `.roko/eval/patterns.json` with `add(entry)`, `query(category, limit) -> Vec<PatternEntry>`.
4. Define `NeuroBridgeOutput` as a type that the orchestrator can use to create engrams in the neuro store (avoiding direct `roko-neuro` dependency from `roko-eval`).

#### Verification Criteria
- [ ] Test pattern creation and query
- [ ] Persistence round-trip test

---

### Task 5.30: Curriculum from failures (WebRL pattern)
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/curriculum.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.5

#### Context
Cluster failed evaluation traces by judge rationale text. Generate synthetic tasks targeting failure modes. Integrates with existing `crates/roko-learn/src/curriculum.rs` and `crates/roko-learn/src/post_gate_reflection.rs`.

#### Implementation Steps
1. Define `CurriculumTask { id, source_cluster_id, cluster_size: u32, prompt, acceptance_criteria: Vec<String>, eval_profile: String, variants: Vec<CurriculumVariant>, priority: f64, status: CurriculumStatus }`.
2. Define `CurriculumStatus { Pending, InProgress, Promoted, Retained, Superseded }`.
3. Define `CurriculumVariant { description: String, edge_case: String }`.
4. Implement `cluster_failures(traces: &[EvalTrace], min_cluster_size: usize) -> Vec<FailureCluster>`: group by common finding rule_ids and criterion names. Simple text similarity (Jaccard over tokenized rationale) rather than embedding-based clustering (MVP).
5. Implement `generate_curriculum_tasks(cluster: &FailureCluster) -> Vec<CurriculumTask>`.

#### Verification Criteria
- [ ] Test cluster creation from 5 failing traces with similar rule_ids
- [ ] Test curriculum task generation

---

### Task 5.31: Pipeline arm bandits (CascadeRouter extension)
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/pipeline_arm.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/events.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.5

#### Context
`PipelineArm` represents a pipeline configuration (model + retrieval_k + flags). Feature vector for LinUCB contextual bandit. Integrates with existing `CascadeRouter` at `crates/roko-learn/src/cascade_router.rs`.

#### Implementation Steps
1. Define `PipelineArm { model: String, retrieval_k: u32, with_clarifying_turn: bool, with_post_fixer: bool }`.
2. Implement `to_features(&self) -> Vec<f64>` for bandit integration.
3. Define `FlywheelEvent { id, timestamp, event_type: FlywheelEventType, trace_id, task_id, metadata }`.
4. Define `FlywheelEventType { TraceEmitted, AutoGradeCompleted, PreferenceMined, PatternExtracted, CurriculumGenerated, ExperimentCreated, CanaryEvaluated, AnchorRotated, DriftDetected }`.

#### Verification Criteria
- [ ] Test feature vector generation
- [ ] Test FlywheelEvent serialization round-trip

---

### Task 5.32: Anti-Goodhart safeguards
**Priority**: P2
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/canary.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/anti_goodhart.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.5

#### Context
Canary set management and Spearman rho tracking. Integrates with `crates/roko-learn/src/drift.rs` for correlation as drift signal.

#### Implementation Steps
1. Define `CanarySet { items: Vec<CanaryItem>, metadata: CanaryMetadata }` persisted to `.roko/eval/canary.json`.
2. Define `CanaryItem { id, prompt, human_rating: f64, last_evaluated: Option<DateTime<Utc>> }`.
3. Implement `spearman_rho(x: &[f64], y: &[f64]) -> f64`: Spearman rank correlation.
4. Implement `check_canary_drift(canary: &CanarySet, recent_scores: &[(String, f64)], threshold: f64) -> Option<DriftDetection>`: if rho < threshold (default 0.6), return drift alert.
5. Define `RubricRotationSchedule { current_emphasis: HashMap<String, f64>, last_rotated: DateTime<Utc>, rotation_interval_days: u32 }` for quarterly rubric emphasis changes.

#### Verification Criteria
- [ ] Test Spearman rho calculation with known ranked lists
- [ ] Test drift detection with correlated and uncorrelated lists

---

## Phase 6: Community Marketplace (`roko-eval-community`)

Publish, discover, fork, and compose evaluation artifacts.

### Task 5.33: Scaffold `roko-eval-community` crate
**Priority**: P3
**Estimated Effort**: 2 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/lib.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (add to `members`)
**Depends On**: 5.1

#### Verification Criteria
- [ ] `cargo build -p roko-eval-community` compiles

---

### Task 5.34: Package format and manifest
**Priority**: P3
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/package.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/namespace.rs`
**Depends On**: 5.33

#### Implementation Steps
1. Define `.rokoeval` ZIP archive format with `manifest.toml`.
2. Implement `create_bundle(dir) -> PathBuf` and `extract_bundle(path) -> TempDir`.
3. Namespace model: `roko/` (official), `@username/` (user).

#### Verification Criteria
- [ ] Create bundle, verify checksums, extract and compare contents

---

### Task 5.35: Dependency resolution and lock file
**Priority**: P3
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/resolver.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/install.rs`
**Depends On**: 5.34

#### Implementation Steps
1. Implement dependency resolution: expand transitive deps, unify version ranges, resolve to highest matching stable.
2. Produce `eval.lock` file.
3. Local installation layout: `.roko/eval/installed/<ns>/<name>/<version>/`.

#### Verification Criteria
- [ ] Test resolution with diamond dependency

---

### Task 5.36: Ed25519 signing and trust system
**Priority**: P3
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/signature.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/trust.rs`
**Depends On**: 5.33

#### Verification Criteria
- [ ] Sign and verify round-trip test

---

### Task 5.37: Fork lineage and attribution
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/lineage.rs`
**Depends On**: 5.33

#### Verification Criteria
- [ ] Test fork chain through 3 generations

---

### Task 5.38: Registry client (CLI operations)
**Priority**: P3
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/registry.rs`
**Depends On**: 5.34, 5.36

#### Implementation Steps
1. REST client for configurable registry URL.
2. Methods: `publish`, `search`, `download`, `fork`, `yank`.
3. CLI integration stubs for `roko eval publish/search/install/fork`.

#### Verification Criteria
- [ ] Mock HTTP server test for search and download

---

### Task 5.39: Gate and learn bridges for community artifacts
**Priority**: P3
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/gate_bridge.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval-community/src/learn_bridge.rs`
**Depends On**: 5.33

#### Implementation Steps
1. Convert installed community criterion into `Criterion` impl.
2. Route by `CriterionKind`.

#### Verification Criteria
- [ ] Test criterion -> Criterion conversion for a deterministic criterion

---

## Phase 7: Dashboard Integration

Eval results surfaced across TUI, web dashboard, and CLI.

### Task 5.40: Runtime event bus extensions
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/runtime_event.rs`
**Depends On**: 5.1

#### Context
The existing `RuntimeEvent` enum at `crates/roko-core/src/runtime_event.rs:56-128` has gate events (`GateStarted`, `GatePassed`, `GateFailed`). Add parallel eval events.

#### Implementation Steps
1. Add variants to `RuntimeEvent`:
   ```rust
   EvalStarted { run_id: String, profile_id: String, task_id: Option<String> },
   EvalCriterionCompleted { run_id: String, criterion_name: String, passed: bool, score: f64, duration_ms: u64 },
   EvalCompleted { run_id: String, verdict_passed: bool, score: f64, criteria_passed: usize, criteria_total: usize, duration_ms: u64, cost_usd: f64 },
   ```
2. Update the `run_id()` method to handle new variants.
3. Update the `Display` impl if one exists.

#### Design Guidance
Do NOT remove existing `GateStarted/GatePassed/GateFailed` variants -- they must remain for backward compatibility. The new `Eval*` variants coexist. The SSE adapter and TUI will consume both.

#### Verification Criteria
- [ ] `cargo build -p roko-core` compiles
- [ ] Event serialization round-trip test for new variants

---

### Task 5.41: `roko eval` CLI command family
**Priority**: P1
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/eval.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml` (add `roko-eval` dependency)
**Depends On**: 5.4, 5.5

#### Context
Commands: `roko eval run <path>`, `roko eval list`, `roko eval show <profile>`, `roko eval history`, `roko eval trace <id>`, `roko eval compare <id1> <id2>`, `roko eval calibrate`.

#### Implementation Steps
1. Define `EvalCommand` enum with clap subcommands.
2. Implement `roko eval list`: reads profiles from built-in + `.roko/eval/profiles/*.toml`.
3. Implement `roko eval history`: reads from `TraceStore::recent(limit)`, renders table.
4. Implement `roko eval trace <id>`: renders full trace detail.
5. Implement `roko eval run <path>`: constructs `ArtifactRef`, resolves profile, runs `EvalService::evaluate()`, prints results.
6. Register in main CLI dispatch.

#### Verification Criteria
- [ ] `cargo build -p roko-cli` compiles
- [ ] Smoke test: `roko eval list` parses and runs (even with no profiles)

---

### Task 5.42: `roko eval` serve routes
**Priority**: P2
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/eval.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/Cargo.toml` (add `roko-eval` dependency)
**Depends On**: 5.5

#### Implementation Steps
1. `GET /api/eval/traces` -- list recent, supports limit/offset/filter.
2. `GET /api/eval/traces/{id}` -- full trace with evidence.
3. `GET /api/eval/summary` -- aggregate stats.
4. `GET /api/eval/criteria` -- list registered criteria.
5. `GET /api/eval/profiles` -- list profiles.
6. `POST /api/eval/run` -- trigger ad-hoc evaluation.
7. Register routes in `crates/roko-serve/src/routes/mod.rs`.

#### Verification Criteria
- [ ] `cargo build -p roko-serve` compiles
- [ ] API integration test for trace listing

---

### Task 5.43: TUI eval trace widget and evidence browser
**Priority**: P2
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/eval_trace.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/criterion_bar.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/mod.rs`
**Depends On**: 5.5, 5.40

#### Implementation Steps
1. `EvalTraceWidget`: compact table rendering criterion name, pass/fail, duration, score bar, finding count.
2. `CriterionBarWidget`: horizontal bar for 0.0-1.0 scores with color coding (red < 0.5, yellow 0.5-0.8, green > 0.8).

#### Verification Criteria
- [ ] Widget render test for correct line count

---

### Task 5.44: Web dashboard -- Arena pages (React)
**Priority**: P3
**Estimated Effort**: 12 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/demo/demo-app/src/pages/ArenaOverview.tsx`
- `/Users/will/dev/nunchi/roko/roko/demo/demo-app/src/pages/EvalHistory.tsx`
- `/Users/will/dev/nunchi/roko/roko/demo/demo-app/src/stores/evalStore.ts`
**Depends On**: 5.42

#### Implementation Steps
1. ArenaOverview: metric cards (total evals, pass rate, mean duration, mean cost) + recent eval timeline.
2. EvalHistory: table with expandable rows showing per-criterion detail.
3. Zustand store subscribing to SSE eval events.

#### Verification Criteria
- [ ] Component renders without errors

---

### Task 5.45: `roko status` eval enhancement
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs` (or status command location)
**Depends On**: 5.5

#### Implementation Steps
1. When eval traces exist, add an "Evaluation Summary" section to `roko status` output:
   - Runs (pass/fail/error counts)
   - Pass rate with trend
   - Mean duration and cost
   - Top failing criteria
2. Read from `TraceStore::recent(1000)` and aggregate.

#### Verification Criteria
- [ ] Test output format with mock trace data

---

## Phase 8: Integration and Migration

Wire everything together through the orchestrator and run the end-to-end path.

### Task 5.46: Wire EvalService into the runtime
**Priority**: P0
**Estimated Effort**: 10 hours
**Files to Modify**:
- The orchestrator or workflow engine dispatch path (either `crates/roko-cli/src/orchestrate.rs` or `crates/roko-runtime/src/workflow_engine.rs` depending on Task 2.x progress)
**Depends On**: 5.4, 5.5, 5.6, 5.8, 5.9, 5.10, 5.11, 5.40

#### Context
After agent dispatch produces output, the runtime must construct `ArtifactRef` from task workdir, select evaluation profile, run `EvalService::evaluate()`, project `EvalTrace` to `GateVerdict`, and emit both `RuntimeEvent::EvalCompleted` and gate events.

The profile selection logic: check `task.eval_profile` -> `plan.eval_profile` -> `roko.toml [eval.default_profile]` -> built-in `rust-strict` for Rust projects.

#### Implementation Steps
1. Construct `BridgeGateService` wrapping `GateService` at startup.
2. Migrate the compile, lint, test, format, and diff gates (add their names to `BridgeGateService.migrated`).
3. After agent dispatch, construct `ArtifactRef` from task workdir.
4. Select profile via the cascade: task config -> plan config -> workspace config -> default.
5. Run `EvalService::evaluate()` through the bridge.
6. Feed eval outcomes to:
   - `EpisodeLogger` (eval_trace_id in episode.extra)
   - `FeedbackService` (via feedback_bridge output)
   - `ExperimentStore` (via prompt_variant)
   - `AdaptiveThresholds` (via per-criterion stats observation)
   - `PreferenceTriple` logger
7. Emit `RuntimeEvent::EvalCompleted`.

#### Design Guidance
This is the highest-risk task. The bridge pattern ensures zero regression: if `EvalService` fails, fall back to legacy `GateService`. Log warnings when fallback occurs. The existing gate call site must be found in the orchestrator -- search for `run_gates` or `GateRunner` calls.

#### Verification Criteria
- [ ] End-to-end: `roko plan run` with a simple task produces an `EvalTrace` in `.roko/eval/traces.jsonl`
- [ ] Events appear on the runtime event bus
- [ ] Legacy gate behavior is preserved for non-migrated gates

---

### Task 5.47: Built-in profiles (TOML)
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/builtin_profiles.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.4

#### Implementation Steps
1. `rust-strict`: compile -> lint -> test -> format -> diff. Sequential. All deterministic.
2. `code-review`: compile -> lint -> test -> format -> diff -> substance -> judge_panel. Sequential.
3. Profile loader: read `.roko/eval/profiles/*.toml`, merge with built-in profiles. Built-in profiles can be overridden by same-named TOML files.

#### Verification Criteria
- [ ] Test profile loading and criterion resolution
- [ ] Test built-in override by TOML file

---

### Task 5.48: Criterion authoring format (user TOML criteria)
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/custom_criterion.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/src/lib.rs`
**Depends On**: 5.3, 5.4

#### Implementation Steps
1. Users author criteria at `.roko/criteria/*.toml`.
2. Two modes:
   - Shell (deterministic): `[criterion.check] type = "shell" command = "..."`. Exit 0 = pass.
   - Judge (stochastic): `[criterion.check] type = "judge"`. Delegates to judge panel with custom rubric.
3. Parser: read TOML into `CustomCriterionDef`, construct `ShellCriterion` or `JudgePanelCriterion`.

#### Verification Criteria
- [ ] Parse and execute a custom shell criterion
- [ ] Parse a judge criterion definition (execution requires judge panel from Phase 4)

---

## Migration Strategy Summary

| Step | What Changes | Risk |
|---|---|---|
| Phase 1 complete | `BridgeGateService` wraps `GateService`, zero migrations | None -- identical behavior |
| Phases 2-3 complete | Individual gates migrated one at a time via `bridge.migrate_gate("compile")` | Low -- bridge projects back to `GateVerdict` |
| Phase 4 complete | `PanelJudgeOracle` replaces `StubJudgeGate` at rung 6 | Medium -- first new behavior (opt-in via profile) |
| Phase 8 (T5.46) | Orchestrator switches to `BridgeGateService` | Medium -- bridge ensures fallback |
| Future | All gates migrated, `GateService` becomes shim | Low -- incremental |

---

## Key Integration Points

| Existing Crate | File | Integration |
|---|---|---|
| `roko-gate` | `gate_service.rs` | `BridgeGateService` wraps `GateService` |
| `roko-gate` | `adaptive_threshold.rs` | `CriterionStats` extends per-rung to per-criterion |
| `roko-gate` | `llm_judge_gate.rs` | `PanelJudgeOracle` implements `JudgeOracle` |
| `roko-gate` | `compile_errors.rs` | `CompileCriterion` reuses parse functions |
| `roko-gate` | `test_gate.rs` | `TestCriterion` reuses `parse_test_counts` |
| `roko-learn` | `cascade_router.rs` | Pipeline arm features extend routing context |
| `roko-learn` | `prompt_experiment.rs` | Auto-grade closes experiment loop |
| `roko-learn` | `feedback_service.rs` | Eval verdicts -> `KnowledgeOutcome` |
| `roko-learn` | `episode_logger.rs` | Traces cross-referenced by task_id |
| `roko-learn` | `post_gate_reflection.rs` | Reflections enrich curriculum clusters |
| `roko-learn` | `drift.rs` | Canary correlation as drift signal |
| `roko-neuro` | `knowledge_store.rs` | Patterns stored as engrams |
| `roko-core` | `runtime_event.rs` | Eval events on the event bus |
| `roko-core` | `foundation.rs` | `GateRunner`, `GateReport`, `GateVerdict` unchanged |
